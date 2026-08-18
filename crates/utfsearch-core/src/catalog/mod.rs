mod format;
mod intern;
mod trigram;
pub mod writer;

use std::collections::HashMap;
use std::fs::File;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use lru::LruCache;
use memmap2::Mmap;
use roaring::RoaringBitmap;
use serde::Serialize;

use crate::catalog::format::{
    read_u16, read_u32, Header, PackedEntry, ENTRY_LEN, HEADER_LEN, NONE,
};
use crate::catalog::intern::Intern;
use crate::catalog::trigram::{intersect_postings, trigrams};
use crate::error::{Error, Result};
use crate::jail::jail_join;
use crate::normalize::{ext_key, search_key};
use crate::query::{Cursor, Query, View, PAGE_BUDGET};
use crate::walk::DirMtimes;

pub use writer::{build, BuildStats};

/// Query-result cache capacity (distinct queries kept warm).
const SEARCH_CACHE_CAP: usize = 256;

pub struct Catalog {
    _file: File,
    map: Mmap,
    hdr: Header,
    intern: Intern,
    name_trigrams: Vec<(u32, RoaringBitmap)>,
    path_trigrams: Vec<(u32, RoaringBitmap)>,
    ext_idx: Vec<(u16, RoaringBitmap)>,
    owner_idx: Vec<(u16, RoaringBitmap)>,
    mtime_ids: Vec<u32>,
    /// Position of each entry id within `mtime_ids` (newest = 0). Built lazily:
    /// only the small-candidate search path needs it, so pure large searches
    /// never pay for it.
    mtime_rank: OnceLock<Vec<u32>>,
    roots: Vec<StoredRoot>,
    /// Parent -> children adjacency. Built lazily on first directory listing;
    /// `search` never touches it.
    children: OnceLock<Vec<Vec<u32>>>,
    /// (root_id, relative-path) -> id. Built lazily on first `id_by_rel`; this
    /// is the single most expensive structure to build (a parent-chain walk
    /// per entry) and `search` never needs it.
    rel_index: OnceLock<HashMap<(u16, String), u32>>,
    /// Memoized query results. Immutable catalog, so entries never go stale.
    search_cache: Mutex<LruCache<Query, Page>>,
}

#[derive(Debug, Clone)]
pub struct StoredRoot {
    pub id: u16,
    pub name: String,
    pub path: PathBuf,
    pub follow_links: bool,
    pub excludes: Vec<String>,
    pub skip_system: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    pub rel: String,
    pub root: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<String>,
    pub size: u64,
    pub mtime: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Page {
    pub hits: Vec<Hit>,
    pub more: bool,
    pub next_cursor: Option<String>,
    pub dropped_unsafe: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Status {
    pub complete: bool,
    pub entry_count: u64,
    pub built_at: i64,
    pub casefold: bool,
    pub roots: Vec<StatusRoot>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusRoot {
    pub name: String,
    pub path: String,
}

impl Catalog {
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let map = unsafe { Mmap::map(&file)? };
        if map.len() < HEADER_LEN {
            return Err(Error::Corrupt("truncated"));
        }
        let hdr = Header::decode(&map)?;
        let intern = load_intern(slice(&map, hdr.intern_off, hdr.intern_len)?)?;
        let name_trigrams = load_u32_bitmaps(slice(&map, hdr.tri_off, hdr.tri_len)?)?;
        let path_trigrams = if hdr.path_tri_len > 0 {
            load_u32_bitmaps(slice(&map, hdr.path_tri_off, hdr.path_tri_len)?)?
        } else {
            Vec::new()
        };
        let ext_idx = load_u16_bitmaps(slice(&map, hdr.ext_off, hdr.ext_len)?)?;
        let owner_idx = load_u16_bitmaps(slice(&map, hdr.owner_off, hdr.owner_len)?)?;
        let mtime_ids = load_u32_list(slice(&map, hdr.mtime_off, hdr.mtime_len)?)?;
        let roots = load_roots(slice(&map, hdr.roots_off, hdr.roots_len)?, &intern)?;

        let n = hdr.entry_count as usize;
        let _ = n;
        // `children`, `rel_index`, and `mtime_rank` are intentionally NOT built
        // here. They are O(n) (and `rel_index` walks the parent chain for every
        // entry) but are only needed by directory listing / refresh / the
        // small-candidate search path. Building them lazily keeps `open()` — and
        // therefore every one-shot `search` CLI invocation — fast on huge
        // catalogs. This mirrors Tantivy's "tiny startup time" design goal.
        let c = Self {
            _file: file,
            map,
            hdr,
            intern,
            name_trigrams,
            path_trigrams,
            ext_idx,
            owner_idx,
            mtime_ids,
            mtime_rank: OnceLock::new(),
            roots,
            children: OnceLock::new(),
            rel_index: OnceLock::new(),
            search_cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(SEARCH_CACHE_CAP).expect("cache cap is nonzero"),
            )),
        };
        Ok(c)
    }

    pub fn packed(&self, id: u32) -> Result<PackedEntry> {
        let off = self.hdr.entries_off as usize + id as usize * ENTRY_LEN;
        let end = off + ENTRY_LEN;
        if end > self.map.len() {
            return Err(Error::Corrupt("entry oob"));
        }
        PackedEntry::decode(&self.map[off..end])
    }

    pub fn intern_str(&self, id: u32) -> String {
        self.intern.get_str(id)
    }

    pub fn children(&self, id: u32) -> Vec<u32> {
        self.children_index()
            .get(id as usize)
            .cloned()
            .unwrap_or_default()
    }

    pub fn id_by_rel(&self, root_id: u16, rel: &str) -> Option<u32> {
        self.rel_index_map()
            .get(&(root_id, rel.to_string()))
            .copied()
    }

    /// Lazily build the parent -> children adjacency list. Only directory
    /// listing and refresh need it; `search` never does.
    fn children_index(&self) -> &Vec<Vec<u32>> {
        self.children.get_or_init(|| {
            let n = self.hdr.entry_count as usize;
            let mut children = vec![Vec::new(); n];
            for id in 0..n as u32 {
                if let Ok(e) = self.packed(id) {
                    if e.parent != NONE && (e.parent as usize) < n {
                        children[e.parent as usize].push(id);
                    }
                }
            }
            children
        })
    }

    /// Lazily build the (root_id, rel) -> id map. This is the single most
    /// expensive structure (a parent-chain walk per entry) and is only needed
    /// by directory listing / refresh — never by `search`.
    fn rel_index_map(&self) -> &HashMap<(u16, String), u32> {
        self.rel_index.get_or_init(|| {
            let n = self.hdr.entry_count as usize;
            let mut map = HashMap::with_capacity(n);
            for id in 0..n as u32 {
                if let Ok(e) = self.packed(id) {
                    map.insert((e.root_id as u16, self.rel_path(id)), id);
                }
            }
            map
        })
    }

    pub fn rel_path(&self, mut id: u32) -> String {
        let mut parts = Vec::new();
        let mut guard = 0;
        while id != NONE && guard < 4096 {
            let Ok(e) = self.packed(id) else { break };
            parts.push(self.intern.get_str(e.name_id));
            id = e.parent;
            guard += 1;
        }
        parts.reverse();
        parts.join("/")
    }

    pub fn dir_mtimes(&self) -> DirMtimes {
        let mut map = DirMtimes::new();
        let Ok(sec) = slice(&self.map, self.hdr.dirstat_off, self.hdr.dirstat_len) else {
            return map;
        };
        let mut off = 0;
        let Ok(n) = read_u32(sec, &mut off) else {
            return map;
        };
        for _ in 0..n {
            let Ok(rel_id) = read_u32(sec, &mut off) else { break };
            if off + 8 > sec.len() {
                break;
            }
            let mtime = i64::from_le_bytes(sec[off..off + 8].try_into().unwrap());
            off += 8;
            let Ok(_eid) = read_u32(sec, &mut off) else { break };
            let rel = self.intern.get_str(rel_id);
            map.insert(rel, mtime);
        }
        map
    }

    pub fn stored_roots(&self) -> &[StoredRoot] {
        &self.roots
    }

    /// Roots remembered inside this catalog (for refresh without --root).
    pub fn root_set(&self) -> Result<crate::root::RootSet> {
        use crate::root::Root;
        crate::root::RootSet::new(
            self.roots
                .iter()
                .map(|r| Root {
                    id: r.id,
                    name: r.name.clone(),
                    path: r.path.clone(),
                    follow_links: r.follow_links,
                    excludes: r.excludes.clone(),
                    skip_system: true,
                })
                .collect(),
        )
    }

    pub fn status(&self) -> Status {
        Status {
            complete: true,
            entry_count: self.hdr.entry_count,
            built_at: self.hdr.built_at,
            casefold: self.hdr.casefold(),
            roots: self
                .roots
                .iter()
                .map(|r| StatusRoot {
                    name: r.name.clone(),
                    path: r.path.display().to_string(),
                })
                .collect(),
        }
    }

    pub fn search(&self, q: Query) -> Result<Page> {
        let q = q.sanitize()?;
        // Cache hit: identical query on an immutable catalog returns the same page.
        if let Ok(mut cache) = self.search_cache.lock() {
            if let Some(page) = cache.get(&q) {
                return Ok(page.clone());
            }
        }
        let page = self.search_uncached(&q)?;
        if let Ok(mut cache) = self.search_cache.lock() {
            cache.put(q, page.clone());
        }
        Ok(page)
    }

    fn search_uncached(&self, q: &Query) -> Result<Page> {
        let casefold = self.hdr.casefold();
        let mut cand = self.seed_candidates(q, casefold)?;

        if let Some(ext) = q.ext.as_deref() {
            if let Some(id) = self.intern.lookup(&ext_key(ext)) {
                if let Some(bm) = lookup_u16(&self.ext_idx, id as u16) {
                    cand = intersect_opt(cand, bm);
                } else {
                    cand = Some(RoaringBitmap::new());
                }
            } else {
                cand = Some(RoaringBitmap::new());
            }
        }
        if let Some(owner) = q.owner.as_deref() {
            let key = search_key(owner, true);
            if let Some(id) = self.intern.lookup(&key) {
                if let Some(bm) = lookup_u16(&self.owner_idx, id as u16) {
                    cand = intersect_opt(cand, bm);
                } else {
                    // contains fallback: no index hit — scan
                    cand = cand.or(None);
                }
            }
        }

        let limit = q.limit as usize;

        // Fast path preserved from the original design: stream the pre-sorted
        // mtime index (newest first) and stop as soon as `limit` hits are
        // collected. This early termination is what keeps large-catalog
        // searches fast, so it must remain the default.
        //
        // The only case where materializing the candidate set wins is when it
        // is *small in absolute terms*: then sorting is negligible and we skip
        // scanning millions of mtime entries whose ids are not candidates.
        // Using an absolute (not ratio) threshold guarantees we never trade
        // away early termination for a broad query.
        const CANDIDATE_SCAN_MAX: usize = 8192;
        if let Some(c) = cand.as_ref().map(|c| c.len() as usize) {
            if c <= CANDIDATE_SCAN_MAX {
                let bm = cand.as_ref().expect("cardinality implies Some");
                let mut ids: Vec<u32> = bm.iter().collect();
                ids.sort_unstable_by_key(|&id| self.mtime_rank(id));
                return self.collect_hits(q, casefold, &ids, true, None, limit);
            }
        }
        self.collect_hits(q, casefold, &self.mtime_ids, true, cand.as_ref(), limit)
    }

    #[inline]
    fn mtime_rank(&self, id: u32) -> u32 {
        self.mtime_rank_vec()
            .get(id as usize)
            .copied()
            .unwrap_or(u32::MAX)
    }

    /// Lazily invert `mtime_ids` into id -> newest-first rank. Only the
    /// small-candidate search path needs this.
    fn mtime_rank_vec(&self) -> &Vec<u32> {
        self.mtime_rank.get_or_init(|| {
            let n = self.hdr.entry_count as usize;
            let mut rank = vec![u32::MAX; n];
            for (r, &id) in self.mtime_ids.iter().enumerate() {
                if (id as usize) < n {
                    rank[id as usize] = r as u32;
                }
            }
            rank
        })
    }

    /// Build a result page by streaming `ids` (already newest-first). When
    /// `apply_filter` is true each id is tested against `cand` (contains) and
    /// `passes()`; when false the ids are assumed already filtered.
    fn collect_hits(
        &self,
        q: &Query,
        casefold: bool,
        ids: &[u32],
        apply_filter: bool,
        cand: Option<&RoaringBitmap>,
        limit: usize,
    ) -> Result<Page> {
        let mut hits = Vec::new();
        let mut dropped_unsafe = 0u32;
        let mut last: Option<Cursor> = None;
        let mut more = false;

        for &id in ids {
            if apply_filter {
                if let Some(c) = cand {
                    if !c.contains(id) {
                        continue;
                    }
                }
            }
            let e = self.packed(id)?;
            if let Some(cur) = q.cursor {
                if !after_cursor(e.mtime, id, cur) {
                    continue;
                }
            }
            if apply_filter && !self.passes(q, id, &e, casefold) {
                continue;
            }
            match self.to_hit(id, &e, q.view) {
                Ok(hit) => {
                    if budget_full(&hits) {
                        more = true;
                        break;
                    }
                    last = Some(Cursor {
                        last_mtime: e.mtime,
                        last_id: id,
                    });
                    hits.push(hit);
                    if hits.len() == limit {
                        more = true;
                        break;
                    }
                }
                Err(Error::Jail) => dropped_unsafe += 1,
                Err(err) => return Err(err),
            }
        }

        Ok(Page {
            hits,
            more,
            next_cursor: if more {
                last.map(|c| c.encode())
            } else {
                None
            },
            dropped_unsafe,
        })
    }

    pub fn children_of(&self, abs_or_rel: &Path, root_hint: Option<&str>, view: View) -> Result<Page> {
        let (root, rel) = self.resolve_dir(abs_or_rel, root_hint)?;
        let id = self
            .id_by_rel(root.id, &rel)
            .ok_or(Error::Msg("not in catalog".into()))?;
        let mut hits = Vec::new();
        let mut dropped_unsafe = 0;
        let mut kids = self.children(id);
        kids.sort_by_key(|&c| {
            self.packed(c)
                .map(|e| std::cmp::Reverse(e.mtime))
                .unwrap_or(std::cmp::Reverse(0))
        });
        for cid in kids {
            let e = self.packed(cid)?;
            match self.to_hit(cid, &e, view) {
                Ok(h) => hits.push(h),
                Err(Error::Jail) => dropped_unsafe += 1,
                Err(err) => return Err(err),
            }
            if hits.len() >= crate::query::MAX_LIMIT as usize {
                break;
            }
        }
        Ok(Page {
            more: false,
            next_cursor: None,
            hits,
            dropped_unsafe,
        })
    }

    fn resolve_dir<'a>(
        &'a self,
        path: &Path,
        root_hint: Option<&str>,
    ) -> Result<(&'a StoredRoot, String)> {
        if let Some(name) = root_hint {
            let root = self
                .roots
                .iter()
                .find(|r| r.name == name)
                .ok_or(Error::Jail)?;
            let rel = path.to_string_lossy().replace('\\', "/");
            let _ = jail_join(&root.path, Path::new(&rel))?;
            return Ok((root, rel));
        }
        for root in &self.roots {
            if let Some(rel) = crate::paths::rel_to(&root.path, path) {
                let _ = jail_join(&root.path, Path::new(&rel))?;
                return Ok((root, rel));
            }
            let rel = path.to_string_lossy().replace('\\', "/");
            if self.id_by_rel(root.id, &rel).is_some() {
                let _ = jail_join(&root.path, Path::new(&rel))?;
                return Ok((root, rel));
            }
        }
        // Existing path: canonicalize against roots.
        let roots: Vec<&Path> = self.roots.iter().map(|r| r.path.as_path()).collect();
        let canon = crate::jail::jail_absolute(path, &roots)?;
        for root in &self.roots {
            if let Ok(st) = canon.strip_prefix(std::fs::canonicalize(&root.path).map_err(|_| Error::Jail)?)
            {
                return Ok((root, st.to_string_lossy().replace('\\', "/")));
            }
        }
        Err(Error::Jail)
    }

    fn seed_candidates(&self, q: &Query, casefold: bool) -> Result<Option<RoaringBitmap>> {
        let mut acc: Option<RoaringBitmap> = None;
        if let Some(name) = q.name.as_deref() {
            acc = and_trigrams(acc, &self.name_trigrams, &search_key(name, casefold));
        }
        if let Some(path) = q.path.as_deref() {
            acc = and_trigrams(acc, &self.path_trigrams, &search_key(path, casefold));
        }
        if let Some(nop) = q.name_or_path.as_deref() {
            let key = search_key(nop, casefold);
            let names = and_trigrams(None, &self.name_trigrams, &key);
            let paths = and_trigrams(None, &self.path_trigrams, &key);
            let uni = match (names, paths) {
                (Some(a), Some(b)) => Some(a | b),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            };
            acc = match (acc, uni) {
                (Some(a), Some(b)) => Some(a & b),
                (None, u) => u,
                (a, None) => a,
            };
        }
        Ok(acc)
    }

    fn passes(&self, q: &Query, id: u32, e: &PackedEntry, casefold: bool) -> bool {
        if let Some(rname) = q.root.as_deref() {
            let ok = self
                .roots
                .iter()
                .any(|r| (r.name == rname || r.id.to_string() == rname) && r.id as u8 == e.root_id);
            if !ok {
                return false;
            }
        }
        if let Some(min) = q.mtime_min {
            if e.mtime < min {
                return false;
            }
        }
        if let Some(max) = q.mtime_max {
            if e.mtime > max {
                return false;
            }
        }
        if let Some(min) = q.size_min {
            if e.size < min {
                return false;
            }
        }
        if let Some(max) = q.size_max {
            if e.size > max {
                return false;
            }
        }
        if let Some(ext) = q.ext.as_deref() {
            if ext_key(&self.intern.get_str(e.ext_id as u32)) != ext {
                return false;
            }
        }

        // Owner is a substring match on the interned owner string. Cheap and
        // does not require the parent-chain walk.
        if let Some(ow) = q.owner.as_deref() {
            let have = search_key(&self.intern.get_str(e.owner_id as u32), true);
            if !have.contains(&search_key(ow, true)) {
                return false;
            }
        }

        // Name / path substring checks. Compute the normalized name and the
        // relative path lazily — the parent-chain walk in `rel_path` is the
        // single most expensive per-entry operation, so it must only run when
        // a path-oriented filter is actually present.
        let needs_name = q.name.is_some() || !q.names.is_empty() || q.name_or_path.is_some();
        let needs_rel = q.path.is_some() || q.name_or_path.is_some();

        let name = if needs_name {
            search_key(&self.intern.get_str(e.name_id), casefold)
        } else {
            String::new()
        };
        let rel = if needs_rel {
            search_key(&self.rel_path(id), casefold)
        } else {
            String::new()
        };

        // Check single name filter (backward compatibility)
        if let Some(n) = q.name.as_deref() {
            if !name.contains(&search_key(n, casefold)) {
                return false;
            }
        }

        // Check multiple name filters - ALL must match (AND logic)
        for n in &q.names {
            if !name.contains(&search_key(n, casefold)) {
                return false;
            }
        }

        if let Some(p) = q.path.as_deref() {
            if !rel.contains(&search_key(p, casefold)) {
                return false;
            }
        }
        if let Some(nop) = q.name_or_path.as_deref() {
            let k = search_key(nop, casefold);
            if !name.contains(&k) && !rel.contains(&k) {
                return false;
            }
        }
        true
    }

    fn to_hit(&self, id: u32, e: &PackedEntry, view: View) -> Result<Hit> {
        let rel = self.rel_path(id);
        let root = self
            .roots
            .iter()
            .find(|r| r.id as u8 == e.root_id)
            .ok_or(Error::Corrupt("root"))?;
        let abs = jail_join(&root.path, Path::new(&rel))?;
        let ext = self.intern.get_str(e.ext_id as u32);
        let owner = self.intern.get_str(e.owner_id as u32);
        let _ = view;
        Ok(Hit {
            rel,
            root: root.path.display().to_string(),
            path: Some(abs.display().to_string()),
            kind: e.kind_str(),
            ext: if ext.is_empty() { None } else { Some(ext) },
            size: e.size,
            mtime: e.mtime,
            owner: if owner.is_empty() { None } else { Some(owner) },
        })
    }
}

fn after_cursor(mtime: i64, id: u32, cur: Cursor) -> bool {
    mtime < cur.last_mtime || (mtime == cur.last_mtime && id > cur.last_id)
}

fn budget_full(hits: &[Hit]) -> bool {
    let n: usize = hits.iter().map(|h| h.rel.len() + 96).sum();
    n >= PAGE_BUDGET
}

fn and_trigrams(
    acc: Option<RoaringBitmap>,
    index: &[(u32, RoaringBitmap)],
    key: &str,
) -> Option<RoaringBitmap> {
    if key.chars().count() < 3 {
        return acc;
    }
    match intersect_postings(index, &trigrams(key)) {
        Some(bm) => intersect_opt(acc, &bm),
        None => acc,
    }
}

fn intersect_opt(acc: Option<RoaringBitmap>, bm: &RoaringBitmap) -> Option<RoaringBitmap> {
    Some(match acc {
        None => bm.clone(),
        Some(a) => a & bm,
    })
}

fn lookup_u16<'a>(idx: &'a [(u16, RoaringBitmap)], k: u16) -> Option<&'a RoaringBitmap> {
    idx.binary_search_by_key(&k, |(a, _)| *a)
        .ok()
        .map(|i| &idx[i].1)
}

fn slice(map: &Mmap, off: u64, len: u64) -> Result<&[u8]> {
    let a = off as usize;
    let b = a.saturating_add(len as usize);
    map.get(a..b).ok_or(Error::Corrupt("section oob"))
}

fn load_intern(sec: &[u8]) -> Result<Intern> {
    let mut off = 0;
    let n = read_u32(sec, &mut off)? as usize;
    let mut offsets = Vec::with_capacity(n);
    for _ in 0..n {
        offsets.push(read_u32(sec, &mut off)?);
    }
    let blob = sec.get(off..).unwrap_or(&[]);
    Ok(Intern::from_serialized(&offsets, blob))
}

fn load_u32_bitmaps(sec: &[u8]) -> Result<Vec<(u32, RoaringBitmap)>> {
    if sec.is_empty() {
        return Ok(Vec::new());
    }
    let mut off = 0;
    let n = read_u32(sec, &mut off)?;
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let k = read_u32(sec, &mut off)?;
        let ln = read_u32(sec, &mut off)? as usize;
        let bytes = sec.get(off..off + ln).ok_or(Error::Corrupt("trigram"))?;
        off += ln;
        let bm = RoaringBitmap::deserialize_from(bytes).map_err(|_| Error::Corrupt("roaring"))?;
        out.push((k, bm));
    }
    Ok(out)
}

fn load_u16_bitmaps(sec: &[u8]) -> Result<Vec<(u16, RoaringBitmap)>> {
    if sec.is_empty() {
        return Ok(Vec::new());
    }
    let mut off = 0;
    let n = read_u32(sec, &mut off)?;
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let k = read_u16(sec, &mut off)?;
        let ln = read_u32(sec, &mut off)? as usize;
        let bytes = sec.get(off..off + ln).ok_or(Error::Corrupt("idx"))?;
        off += ln;
        let bm = RoaringBitmap::deserialize_from(bytes).map_err(|_| Error::Corrupt("roaring"))?;
        out.push((k, bm));
    }
    Ok(out)
}

fn load_u32_list(sec: &[u8]) -> Result<Vec<u32>> {
    if sec.len() % 4 != 0 {
        return Err(Error::Corrupt("mtime list"));
    }
    Ok(sec
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect())
}

fn load_roots(sec: &[u8], intern: &Intern) -> Result<Vec<StoredRoot>> {
    if sec.is_empty() {
        return Ok(Vec::new());
    }
    let mut off = 0;
    let n = read_u32(sec, &mut off)?;
    let mut out = Vec::new();
    for _ in 0..n {
        let id = read_u16(sec, &mut off)?;
        let name_id = read_u32(sec, &mut off)?;
        let plen = read_u32(sec, &mut off)? as usize;
        let pb = sec.get(off..off + plen).ok_or(Error::Corrupt("root path"))?;
        off += plen;
        let follow = *sec.get(off).ok_or(Error::Corrupt("follow"))? != 0;
        off += 1;
        let exc_n = read_u32(sec, &mut off)?;
        let mut excludes = Vec::new();
        for _ in 0..exc_n {
            let ln = read_u32(sec, &mut off)? as usize;
            let eb = sec.get(off..off + ln).ok_or(Error::Corrupt("exclude"))?;
            off += ln;
            excludes.push(String::from_utf8_lossy(eb).into_owned());
        }
        out.push(StoredRoot {
            id,
            name: intern.get_str(name_id),
            path: PathBuf::from(String::from_utf8_lossy(pb).into_owned()),
            follow_links: follow,
            excludes,
            skip_system: true,
        });
    }
    Ok(out)
}
