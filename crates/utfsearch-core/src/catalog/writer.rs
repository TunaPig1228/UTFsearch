use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use roaring::RoaringBitmap;

use crate::catalog::format::{
    put_u16, put_u32, Header, PackedEntry, ENTRY_LEN, FLAG_DIR, FLAG_MTIME_MISSING, FLAG_OTHER,
    FLAG_SYMLINK, HEADER_LEN, HDR_CASEFOLD, NONE,
};
use crate::catalog::intern::Intern;
use crate::catalog::trigram::trigrams;
use crate::catalog::Catalog;
use crate::error::Result;
use crate::normalize::search_key;
use crate::root::RootSet;
use crate::walk::{walk_root, WalkRec};

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct BuildStats {
    pub entries: u64,
    pub pruned_dirs: u64,
    pub visited_dirs: u64,
    pub catalog: String,
}

pub fn build(catalog_path: &Path, roots: &RootSet, old: Option<&Catalog>) -> Result<BuildStats> {
    let casefold = cfg!(windows);
    let mut intern = Intern::new();
    let mut entries: Vec<PackedEntry> = Vec::new();
    let mut rel_of: Vec<String> = Vec::new();
    let mut parent_stack: HashMap<(u16, String), u32> = HashMap::new();
    let mut dir_stats: Vec<(u32, i64, u32)> = Vec::new();
    let mut pruned_total = 0u64;
    let mut visited_dirs = 0u64;

    let prev_mtimes = old.map(Catalog::dir_mtimes).unwrap_or_default();

    for root in &roots.roots {
        let mut pruned = Vec::new();
        let recs = walk_root(root, &prev_mtimes, &mut pruned)?;
        pruned_total += pruned.len() as u64;

        if let Some(oldc) = old {
            for rel in &pruned {
                copy_subtree(oldc, root.id, rel, &mut intern, &mut entries, &mut rel_of, &mut parent_stack)?;
            }
        }

        let mut recs = recs;
        recs.sort_by(|a, b| {
            let da = a.rel.matches('/').count();
            let db = b.rel.matches('/').count();
            da.cmp(&db).then_with(|| a.rel.cmp(&b.rel))
        });

        for rec in recs {
            if rec.is_dir {
                visited_dirs += 1;
            }
            push_rec(
                root.id,
                &rec,
                casefold,
                &mut intern,
                &mut entries,
                &mut rel_of,
                &mut parent_stack,
                &mut dir_stats,
            );
        }
    }

    write_catalog(catalog_path, roots, casefold, intern, entries, dir_stats)?;
    Ok(BuildStats {
        entries: rel_of.len() as u64,
        pruned_dirs: pruned_total,
        visited_dirs,
        catalog: catalog_path.display().to_string(),
    })
}

fn push_rec(
    root_id: u16,
    rec: &WalkRec,
    casefold: bool,
    intern: &mut Intern,
    entries: &mut Vec<PackedEntry>,
    rel_of: &mut Vec<String>,
    parent_stack: &mut HashMap<(u16, String), u32>,
    dir_stats: &mut Vec<(u32, i64, u32)>,
) -> u32 {
    let parent_rel = rec
        .rel
        .rsplit_once('/')
        .map(|(p, _)| p.to_string())
        .unwrap_or_default();
    let parent = if parent_rel.is_empty() {
        NONE
    } else {
        parent_stack
            .get(&(root_id, parent_rel))
            .copied()
            .unwrap_or(NONE)
    };
    let name_id = intern.intern(&rec.name);
    let ext_id = if rec.ext.is_empty() {
        0
    } else {
        intern.intern(&rec.ext) as u16
    };
    let owner_id = if rec.owner.is_empty() {
        0
    } else {
        intern.intern(&search_key(&rec.owner, true)) as u16
    };
    let mut flags = 0u8;
    if rec.is_dir {
        flags |= FLAG_DIR;
    } else if rec.is_symlink {
        flags |= FLAG_OTHER | FLAG_SYMLINK;
    }
    if rec.mtime_missing {
        flags |= FLAG_MTIME_MISSING;
    }
    let id = entries.len() as u32;
    entries.push(PackedEntry {
        parent,
        name_id,
        ext_id,
        owner_id,
        root_id: root_id as u8,
        flags,
        size: rec.size,
        mtime: rec.mtime,
    });
    rel_of.push(rec.rel.clone());
    if rec.is_dir {
        parent_stack.insert((root_id, rec.rel.clone()), id);
        let rel_id = intern.intern(&rec.rel);
        dir_stats.push((rel_id, rec.mtime_fine, id));
    }
    let _ = casefold;
    id
}

fn copy_subtree(
    old: &Catalog,
    root_id: u16,
    rel: &str,
    intern: &mut Intern,
    entries: &mut Vec<PackedEntry>,
    rel_of: &mut Vec<String>,
    parent_stack: &mut HashMap<(u16, String), u32>,
) -> Result<()> {
    let Some(old_id) = old.id_by_rel(root_id, rel) else {
        return Ok(());
    };
    let mut stack = vec![(old_id, NONE)];
    while let Some((oid, forced_parent)) = stack.pop() {
        let e = old.packed(oid)?;
        let name = old.intern_str(e.name_id);
        let rec_rel = old.rel_path(oid);
        let parent = if forced_parent != NONE {
            forced_parent
        } else {
            rec_rel
                .rsplit_once('/')
                .and_then(|(p, _)| parent_stack.get(&(root_id, p.to_string())).copied())
                .unwrap_or(NONE)
        };
        let name_id = intern.intern(&name);
        let ext = old.intern_str(e.ext_id as u32);
        let ext_id = if ext.is_empty() {
            0
        } else {
            intern.intern(&ext) as u16
        };
        let owner = old.intern_str(e.owner_id as u32);
        let owner_id = if owner.is_empty() {
            0
        } else {
            intern.intern(&owner) as u16
        };
        let nid = entries.len() as u32;
        entries.push(PackedEntry {
            parent,
            name_id,
            ext_id,
            owner_id,
            root_id: root_id as u8,
            flags: e.flags,
            size: e.size,
            mtime: e.mtime,
        });
        rel_of.push(rec_rel.clone());
        if e.is_dir() {
            parent_stack.insert((root_id, rec_rel), nid);
        }
        for child in old.children(oid).into_iter().rev() {
            stack.push((child, nid));
        }
    }
    Ok(())
}

fn write_catalog(
    path: &Path,
    roots: &RootSet,
    casefold: bool,
    mut intern: Intern,
    entries: Vec<PackedEntry>,
    dir_stats: Vec<(u32, i64, u32)>,
) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = tmp_path(path);

    for r in &roots.roots {
        intern.intern(&r.name);
        intern.intern(&r.path.to_string_lossy());
        for ex in &r.excludes {
            intern.intern(ex);
        }
    }

    let mut entries_sec = Vec::with_capacity(entries.len() * ENTRY_LEN);
    for e in &entries {
        entries_sec.extend_from_slice(&e.encode());
    }

    let mut name_tri: HashMap<u32, RoaringBitmap> = HashMap::new();
    let mut path_tri: HashMap<u32, RoaringBitmap> = HashMap::new();
    let mut ext_map: HashMap<u16, RoaringBitmap> = HashMap::new();
    let mut owner_map: HashMap<u16, RoaringBitmap> = HashMap::new();
    let mut mtime_ids: Vec<u32> = (0..entries.len() as u32).collect();

    for (i, e) in entries.iter().enumerate() {
        let id = i as u32;
        let name = intern.get_str(e.name_id);
        let key = search_key(&name, casefold);
        for g in trigrams(&key) {
            name_tri.entry(g).or_default().insert(id);
        }
        if e.ext_id != 0 {
            ext_map.entry(e.ext_id).or_default().insert(id);
        }
        if e.owner_id != 0 {
            owner_map.entry(e.owner_id).or_default().insert(id);
        }
    }

    // Path trigrams: walk parent chain once per entry.
    for (i, _) in entries.iter().enumerate() {
        let id = i as u32;
        let rel = rel_from_entries(&entries, &intern, id, casefold);
        for g in trigrams(&rel) {
            path_tri.entry(g).or_default().insert(id);
        }
    }

    mtime_ids.sort_by(|&a, &b| {
        entries[b as usize]
            .mtime
            .cmp(&entries[a as usize].mtime)
            .then(a.cmp(&b))
    });

    let tri_sec = write_u32_bitmaps(name_tri.into_iter().map(|(k, v)| (k, v)));
    let path_tri_sec = write_u32_bitmaps(path_tri.into_iter().map(|(k, v)| (k, v)));
    let ext_sec = write_u16_bitmaps(ext_map);
    let owner_sec = write_u16_bitmaps(owner_map);

    let mut dirstat_sec = Vec::new();
    put_u32(&mut dirstat_sec, dir_stats.len() as u32);
    for (rel_id, mtime, eid) in dir_stats {
        put_u32(&mut dirstat_sec, rel_id);
        dirstat_sec.extend_from_slice(&mtime.to_le_bytes());
        put_u32(&mut dirstat_sec, eid);
    }

    let mut mtime_sec = Vec::with_capacity(mtime_ids.len() * 4);
    for id in mtime_ids {
        put_u32(&mut mtime_sec, id);
    }

    let mut roots_sec = Vec::new();
    put_u32(&mut roots_sec, roots.roots.len() as u32);
    for r in &roots.roots {
        put_u16(&mut roots_sec, r.id);
        put_u32(
            &mut roots_sec,
            intern.lookup(&r.name).unwrap_or(0),
        );
        let p = r.path.to_string_lossy();
        let pb = p.as_bytes();
        put_u32(&mut roots_sec, pb.len() as u32);
        roots_sec.extend_from_slice(pb);
        roots_sec.push(if r.follow_links { 1 } else { 0 });
        put_u32(&mut roots_sec, r.excludes.len() as u32);
        for ex in &r.excludes {
            let eb = ex.as_bytes();
            put_u32(&mut roots_sec, eb.len() as u32);
            roots_sec.extend_from_slice(eb);
        }
    }

    let (offsets, blob) = intern.serialize();
    let mut intern_sec = Vec::new();
    put_u32(&mut intern_sec, offsets.len() as u32);
    for o in &offsets {
        put_u32(&mut intern_sec, *o);
    }
    intern_sec.extend_from_slice(&blob);

    let mut off = HEADER_LEN as u64;
    let mut hdr = Header {
        version: crate::catalog::format::VERSION,
        flags: if casefold { HDR_CASEFOLD } else { 0 },
        built_at: now_unix(),
        entry_count: entries.len() as u64,
        ..Header::default()
    };
    let place = |off: &mut u64, len: u64| -> (u64, u64) {
        let start = *off;
        *off += len;
        (start, len)
    };
    (hdr.intern_off, hdr.intern_len) = place(&mut off, intern_sec.len() as u64);
    (hdr.entries_off, hdr.entries_len) = place(&mut off, entries_sec.len() as u64);
    (hdr.tri_off, hdr.tri_len) = place(&mut off, tri_sec.len() as u64);
    (hdr.ext_off, hdr.ext_len) = place(&mut off, ext_sec.len() as u64);
    (hdr.owner_off, hdr.owner_len) = place(&mut off, owner_sec.len() as u64);
    (hdr.dirstat_off, hdr.dirstat_len) = place(&mut off, dirstat_sec.len() as u64);
    (hdr.mtime_off, hdr.mtime_len) = place(&mut off, mtime_sec.len() as u64);
    (hdr.roots_off, hdr.roots_len) = place(&mut off, roots_sec.len() as u64);
    (hdr.path_tri_off, hdr.path_tri_len) = place(&mut off, path_tri_sec.len() as u64);

    let mut opts = OpenOptions::new();
    opts.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(&tmp)?;
    f.write_all(&hdr.encode())?;
    f.write_all(&intern_sec)?;
    f.write_all(&entries_sec)?;
    f.write_all(&tri_sec)?;
    f.write_all(&ext_sec)?;
    f.write_all(&owner_sec)?;
    f.write_all(&dirstat_sec)?;
    f.write_all(&mtime_sec)?;
    f.write_all(&roots_sec)?;
    f.write_all(&path_tri_sec)?;
    f.sync_all()?;
    drop(f);
    
    // Retry rename operation with exponential backoff - file may still be locked
    let mut retry_count = 0;
    let max_retries = 10;
    loop {
        match fs::rename(&tmp, path) {
            Ok(_) => {
                eprintln!("Catalog written successfully to: {}", path.display());
                return Ok(());
            }
            Err(e) if retry_count < max_retries => {
                retry_count += 1;
                let delay_ms = 100 * retry_count;
                eprintln!("Failed to rename catalog (attempt {}): {}, retrying in {}ms...", retry_count, e, delay_ms);
                std::thread::sleep(std::time::Duration::from_millis(delay_ms as u64));
            }
            Err(e) => {
                eprintln!("Failed to rename catalog after {} attempts: {}", max_retries, e);
                return Err(e.into());
            }
        }
    }
}

fn rel_from_entries(entries: &[PackedEntry], intern: &Intern, mut id: u32, casefold: bool) -> String {
    let mut parts = Vec::new();
    let mut guard = 0;
    while id != NONE && (id as usize) < entries.len() && guard < 4096 {
        let e = entries[id as usize];
        parts.push(search_key(&intern.get_str(e.name_id), casefold));
        id = e.parent;
        guard += 1;
    }
    parts.reverse();
    parts.join("/")
}

fn write_u32_bitmaps(iter: impl Iterator<Item = (u32, RoaringBitmap)>) -> Vec<u8> {
    let mut pairs: Vec<_> = iter.collect();
    pairs.sort_by_key(|(k, _)| *k);
    let mut out = Vec::new();
    put_u32(&mut out, pairs.len() as u32);
    for (k, bm) in pairs {
        put_u32(&mut out, k);
        let mut payload = Vec::new();
        bm.serialize_into(&mut payload).ok();
        put_u32(&mut out, payload.len() as u32);
        out.extend_from_slice(&payload);
    }
    out
}

fn write_u16_bitmaps(map: HashMap<u16, RoaringBitmap>) -> Vec<u8> {
    let mut pairs: Vec<_> = map.into_iter().collect();
    pairs.sort_by_key(|(k, _)| *k);
    let mut out = Vec::new();
    put_u32(&mut out, pairs.len() as u32);
    for (k, bm) in pairs {
        put_u16(&mut out, k);
        let mut payload = Vec::new();
        bm.serialize_into(&mut payload).ok();
        put_u32(&mut out, payload.len() as u32);
        out.extend_from_slice(&payload);
    }
    out
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut p = path.as_os_str().to_os_string();
    p.push(".tmp");
    PathBuf::from(p)
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
