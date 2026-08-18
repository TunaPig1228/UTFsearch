use std::collections::HashMap;

use crate::root::Root;
use crate::error::Result;

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

/// Walk directory using WizTree (much faster than jwalk)
pub fn walk_root(
    root: &Root,
    _prev: &DirMtimes,
    pruned: &mut Vec<String>,
) -> Result<Vec<WalkRec>> {
    // Use WizTree for scanning (fast MFT-based on Windows)
    let mut records = crate::wiztree::walk_root_wiztree(root, _prev, pruned)?;
    
    // Sort by depth then by name (same as original walk_root)
    records.sort_by(|a, b| {
        let da = a.rel.matches('/').count();
        let db = b.rel.matches('/').count();
        da.cmp(&db).then_with(|| a.rel.cmp(&b.rel))
    });
    
    Ok(records)
}
