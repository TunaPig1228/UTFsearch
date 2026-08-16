use std::fs;
use tempfile::tempdir;
use utfsearch_core::jail::{jail_absolute, jail_join};

#[test]
fn jail_blocks_escape() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("root");
    let other = dir.path().join("other");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&other).unwrap();
    fs::write(root.join("ok.txt"), b"x").unwrap();
    fs::write(other.join("secret.txt"), b"x").unwrap();

    assert!(jail_absolute(&root.join("ok.txt"), &[root.as_path()]).is_ok());
    assert!(jail_absolute(&other.join("secret.txt"), &[root.as_path()]).is_err());
    assert!(jail_join(&root, std::path::Path::new("..")).is_err());
    assert!(jail_join(&root, std::path::Path::new("ok.txt")).is_ok());
}
