use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_utfsearch"))
}

#[test]
fn cli_index_and_search_json() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("root");
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("docs/hello.txt"), b"hi").unwrap();
    let catalog = dir.path().join("c.uts");

    let st = bin()
        .args([
            "--catalog",
            catalog.to_str().unwrap(),
            "--format",
            "json",
            "index",
            "--root",
            root.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(st.status.success(), "{}", String::from_utf8_lossy(&st.stderr));

    fs::write(root.join("docs/later.txt"), b"x").unwrap();
    let st = bin()
        .args([
            "--catalog",
            catalog.to_str().unwrap(),
            "--format",
            "json",
            "refresh",
        ])
        .output()
        .unwrap();
    assert!(st.status.success(), "{}", String::from_utf8_lossy(&st.stderr));

    let st = bin()
        .args([
            "--catalog",
            catalog.to_str().unwrap(),
            "--format",
            "json",
            "search",
            "hello",
        ])
        .output()
        .unwrap();
    assert!(st.status.success(), "{}", String::from_utf8_lossy(&st.stderr));
    let stdout = String::from_utf8_lossy(&st.stdout);
    assert!(stdout.contains("hello.txt"), "{stdout}");

    let st = bin()
        .args(["mcp", "--http", "127.0.0.1:0"])
        .arg("--catalog")
        .arg(&catalog)
        .output()
        .unwrap();
    assert!(!st.status.success());
}


