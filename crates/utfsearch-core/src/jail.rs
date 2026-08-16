use std::path::{Component, Path, PathBuf};

use crate::error::{Error, Result};

/// Lexical join of `rel` onto `root`. Rejects escape via `..` or absolute `rel`.
pub fn jail_join(root: &Path, rel: &Path) -> Result<PathBuf> {
    if rel.is_absolute() {
        return jail_absolute(rel, &[root]);
    }
    let mut out = root.to_path_buf();
    for c in rel.components() {
        match c {
            Component::CurDir => {}
            Component::Normal(s) => out.push(s),
            Component::ParentDir => {
                if !out.pop() || !out.starts_with(root) {
                    return Err(Error::Jail);
                }
            }
            Component::Prefix(_) | Component::RootDir => return Err(Error::Jail),
        }
    }
    if !out.starts_with(root) {
        return Err(Error::Jail);
    }
    Ok(out)
}

/// Canonicalize `path` (must exist) and require it to sit under one of `roots`.
pub fn jail_absolute(path: &Path, roots: &[&Path]) -> Result<PathBuf> {
    let canon = std::fs::canonicalize(path).map_err(|_| Error::Jail)?;
    for root in roots {
        let r = std::fs::canonicalize(root).map_err(|_| Error::Jail)?;
        if canon.starts_with(&r) {
            return Ok(canon);
        }
    }
    Err(Error::Jail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn lexical_blocks_dotdot() {
        let root = Path::new("/allowed");
        assert!(jail_join(root, Path::new("a/b")).is_ok());
        assert!(jail_join(root, Path::new("a/../../outside")).is_err());
        assert!(jail_join(root, Path::new("/etc/passwd")).is_err());
    }

    #[test]
    fn absolute_blocks_sibling() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        let other = dir.path().join("other");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&other).unwrap();
        fs::write(root.join("ok.txt"), b"x").unwrap();
        fs::write(other.join("secret.txt"), b"x").unwrap();
        assert!(jail_absolute(&root.join("ok.txt"), &[root.as_path()]).is_ok());
        assert!(jail_absolute(&other.join("secret.txt"), &[root.as_path()]).is_err());
    }
}
