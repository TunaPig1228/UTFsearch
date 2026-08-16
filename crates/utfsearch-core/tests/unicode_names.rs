use std::fs;
use tempfile::tempdir;
use utfsearch_core::{build, Catalog, Query, Root, RootSet};

#[test]
fn cjk_and_nfc() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("r");
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("docs").join("發票.xlsx"), b"1").unwrap();
    let nfc = "café.txt";
    fs::write(root.join("docs").join(nfc), b"2").unwrap();

    let cat_path = dir.path().join("c.uts");
    let rs = RootSet::new(vec![Root {
        id: 0,
        name: "r".into(),
        path: root,
        follow_links: false,
        excludes: vec![],
        skip_system: true,
    }])
    .unwrap();
    build(&cat_path, &rs, None).unwrap();
    let cat = Catalog::open(&cat_path).unwrap();

    let mut q = Query::new();
    q.name = Some("發票".into());
    assert_eq!(cat.search(q).unwrap().hits.len(), 1);

    let mut q = Query::new();
    q.name = Some("cafe\u{0301}".into());
    let hits = cat.search(q).unwrap();
    assert_eq!(hits.hits.len(), 1);
}
