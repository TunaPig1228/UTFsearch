use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use globset::GlobSet;
use jwalk::{DirEntry, WalkDirGeneric};

use crate::normalize::ext_key;
use crate::owner::file_owner;
use crate::root::Root;

#[derive(Debug, Clone)]
pub struct WalkRec {
    pub rel: String,
    pub name: String,
    pub ext: String,
    pub owner: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size: u64,
    pub mtime: i64,
    /// Milliseconds; used only for directory prune (seconds collide in tests).
    pub mtime_fine: i64,
    pub mtime_missing: bool,
    pub non_utf8: bool,
}

/// Directory mtime from a previous catalog, keyed by relative path with `/`.
pub type DirMtimes = HashMap<String, i64>;

pub fn walk_root(
    root: &Root,
    prev: &DirMtimes,
    pruned: &mut Vec<String>,
) -> crate::error::Result<Vec<WalkRec>> {
    let excludes = root.globset()?;
    let excludes_walk = excludes.clone();
    let root_path = root.path.clone();
    let follow = root.follow_links;
    let skip_sys = root.skip_system;
    let prev_walk = prev.clone();

    let walker = WalkDirGeneric::<((), bool)>::new(&root_path)
        .follow_links(follow)
        .skip_hidden(false)
        .process_read_dir(move |_depth, path, _state, children| {
            let Some(rel) = crate::paths::rel_to(&root_path, path) else {
                return;
            };
            if should_exclude(&rel, path.file_name().and_then(|s| s.to_str()), &excludes_walk) {
                children.clear();
                return;
            }
            if skip_sys && crate::skip::skip_dir(&root_path, path) {
                children.clear();
                return;
            }
            if !rel.is_empty() {
                if let Some(&old) = prev_walk.get(&rel) {
                    if dir_mtime_fine(path) == Some(old) {
                        children.clear();
                        return;
                    }
                }
            }
        });

    let mut out = Vec::new();
    let mut seen_pruned = Vec::new();

    for ent in walker.into_iter().flatten() {
        let path = ent.path();
        let Some(rel) = crate::paths::rel_to(&root.path, &path) else {
            continue;
        };
        if rel.is_empty() {
            continue;
        };
        if should_exclude(&rel, path.file_name().and_then(|s| s.to_str()), &excludes) {
            continue;
        }
        let ft = ent.file_type();
        if root.skip_system {
            if ft.is_dir() && crate::skip::skip_dir(&root.path, &path) {
                continue;
            }
            if ft.is_file() && crate::skip::skip_file(&path) {
                continue;
            }
        }
        if ft.is_dir() {
            if let Some(&old) = prev.get(&rel) {
                if dir_mtime_fine(&path) == Some(old) {
                    seen_pruned.push(rel);
                    continue;
                }
            }
        }
        if let Some(rec) = record(&ent, &rel) {
            out.push(rec);
        }
    }
    pruned.extend(seen_pruned);
    Ok(out)
}

fn should_exclude(rel: &str, name: Option<&str>, set: &GlobSet) -> bool {
    if set.is_empty() {
        return false;
    }
    if set.is_match(rel) {
        return true;
    }
    if let Some(n) = name {
        if set.is_match(n) {
            return true;
        }
    }
    false
}

fn dir_mtime_fine(path: &Path) -> Option<i64> {
    let meta = std::fs::metadata(path).ok()?;
    stamp_millis(&meta)
}

fn stamp_millis(meta: &std::fs::Metadata) -> Option<i64> {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .map(|d| d.as_millis() as i64)
        })
}

fn record(ent: &DirEntry<((), bool)>, rel: &str) -> Option<WalkRec> {
    let path: PathBuf = ent.path();
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    if name.is_empty() {
        return None;
    }
    let meta = ent.metadata().ok()?;
    let fine = stamp_millis(&meta);
    let mtime = fine.map(|ms| ms / 1000);
    let ext = path
        .extension()
        .map(|s| ext_key(&s.to_string_lossy()))
        .unwrap_or_default();
    Some(WalkRec {
        rel: rel.to_string(),
        name,
        ext,
        owner: file_owner(&path).unwrap_or_default(),
        is_dir: meta.is_dir(),
        is_symlink: meta.file_type().is_symlink(),
        size: if meta.is_dir() { 0 } else { meta.len() },
        mtime: mtime.unwrap_or(0),
        mtime_fine: fine.unwrap_or(0),
        mtime_missing: mtime.is_none(),
        non_utf8: !path.to_str().is_some() || rel.bytes().any(|_| false),
    })
}
