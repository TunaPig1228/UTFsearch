use std::path::{Path, PathBuf};

/// Drop Windows `\\?\` / `\\.\` prefixes so strip_prefix works across jwalk and config paths.
pub fn slim(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{rest}"))
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else if let Some(rest) = s.strip_prefix(r"\\.\") {
        PathBuf::from(rest)
    } else {
        p.to_path_buf()
    }
}

pub fn rel_to(root: &Path, path: &Path) -> Option<String> {
    let r = slim(root);
    let p = slim(path);
    p.strip_prefix(&r).ok().map(|rel| {
        rel.to_string_lossy()
            .replace('\\', "/")
            .trim_start_matches('/')
            .to_string()
    })
}
