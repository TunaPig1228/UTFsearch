use std::fs;
use std::time::{Duration, SystemTime};

use tempfile::tempdir;
use utfsearch_core::{build, Catalog, Query, Root, RootSet};

fn roots(path: &std::path::Path, excludes: &[&str]) -> RootSet {
    RootSet::new(vec![Root {
        id: 0,
        name: "demo".into(),
        path: path.to_path_buf(),
        follow_links: false,
        excludes: excludes.iter().map(|s| s.to_string()).collect(),
        skip_system: true,
    }])
    .unwrap()
}

#[test]
fn index_search_ext_exclude_newest() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("root");
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::create_dir_all(root.join("skipme")).unwrap();
    fs::write(root.join("docs").join("發票.xlsx"), b"a").unwrap();
    fs::write(root.join("docs").join("cafe.txt"), b"b").unwrap();
    fs::create_dir_all(root.join("deep/a/b/c")).unwrap();
    fs::write(root.join("deep/a/b/c/needle.txt"), b"c").unwrap();
    fs::write(root.join("skipme/secret.bin"), b"no").unwrap();

    let cat_path = dir.path().join("catalog.uts");
    let stats = build(&cat_path, &roots(&root, &["skipme"]), None).unwrap();
    assert!(stats.entries >= 4, "stats={stats:?}");
    let cat = Catalog::open(&cat_path).unwrap();

    let mut q = Query::new();
    q.name = Some("發票".into());
    let page = cat.search(q).unwrap();
    assert_eq!(page.hits.len(), 1);
    assert!(page.hits[0].rel.ends_with("發票.xlsx") || page.hits[0].rel.contains("發票"));

    let mut q = Query::new();
    q.name = Some("needle".into());
    assert_eq!(cat.search(q).unwrap().hits.len(), 1);

    let mut q = Query::new();
    q.name_or_path = Some("secret".into());
    assert!(cat.search(q).unwrap().hits.is_empty());

    let mut q = Query::new();
    q.ext = Some("txt".into());
    let txt = cat.search(q).unwrap();
    assert!(txt.hits.len() >= 2);
    // newest-first: mtimes non-increasing
    for w in txt.hits.windows(2) {
        assert!(w[0].mtime >= w[1].mtime);
    }
}

#[test]
fn refresh_picks_up_new_file() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("root");
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("docs/a.txt"), b"1").unwrap();
    let cat_path = dir.path().join("c.uts");
    let rs = roots(&root, &[]);
    build(&cat_path, &rs, None).unwrap();

    std::thread::sleep(Duration::from_millis(50));
    let now = SystemTime::now();
    fs::write(root.join("docs/new.txt"), b"2").unwrap();
    let _ = filetime_now(root.join("docs"), now);

    let old = Catalog::open(&cat_path).unwrap();
    build(&cat_path, &rs, Some(&old)).unwrap();
    let cat = Catalog::open(&cat_path).unwrap();
    let mut q = Query::new();
    q.name = Some("new".into());
    assert_eq!(cat.search(q).unwrap().hits.len(), 1);
}

fn filetime_now(path: std::path::PathBuf, _now: SystemTime) {
    let _ = fs::File::open(path);
}

#[test]
fn page_cap_200() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("root");
    fs::create_dir_all(&root).unwrap();
    for i in 0..250 {
        fs::write(root.join(format!("bulk-{i:03}.txt")), b"x").unwrap();
    }
    let cat_path = dir.path().join("c.uts");
    build(&cat_path, &roots(&root, &[]), None).unwrap();
    let cat = Catalog::open(&cat_path).unwrap();
    let mut q = Query::new();
    q.name = Some("bulk".into());
    let page = cat.search(q).unwrap();
    assert_eq!(page.hits.len(), 200);
    assert!(page.more);
    assert!(page.next_cursor.is_some());
}

#[test]
fn skips_recycle_bin_name() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("root");
    fs::create_dir_all(root.join("$Recycle.Bin")).unwrap();
    fs::write(root.join("$Recycle.Bin").join("junk.txt"), b"x").unwrap();
    fs::write(root.join("keep.txt"), b"y").unwrap();
    let cat_path = dir.path().join("c.uts");
    build(&cat_path, &roots(&root, &[]), None).unwrap();
    let cat = Catalog::open(&cat_path).unwrap();
    let mut q = Query::new();
    q.name = Some("junk".into());
    assert!(cat.search(q).unwrap().hits.is_empty());
    let mut q = Query::new();
    q.name = Some("keep".into());
    assert_eq!(cat.search(q).unwrap().hits.len(), 1);
}

#[test]
fn skips_node_modules_and_venv() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("root");
    fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
    fs::write(root.join("node_modules/pkg/index.js"), b"x").unwrap();
    fs::create_dir_all(root.join(".venv/lib")).unwrap();
    fs::write(root.join(".venv/lib/foo.py"), b"x").unwrap();
    fs::create_dir_all(root.join("build")).unwrap();
    fs::write(root.join("build/out.o"), b"x").unwrap();
    fs::write(root.join("report.docx"), b"y").unwrap();
    let cat_path = dir.path().join("c.uts");
    build(&cat_path, &roots(&root, &[]), None).unwrap();
    let cat = Catalog::open(&cat_path).unwrap();
    for noise in ["index", "foo", "out"] {
        let mut q = Query::new();
        q.name = Some(noise.into());
        assert!(cat.search(q).unwrap().hits.is_empty(), "{noise}");
    }
    let mut q = Query::new();
    q.name = Some("report".into());
    assert_eq!(cat.search(q).unwrap().hits.len(), 1);
}

#[test]
fn dir_scope_restricts_to_subtree() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("root");
    // Same filename in two different subtrees.
    fs::create_dir_all(root.join("finance/2024/invoices")).unwrap();
    fs::create_dir_all(root.join("finance/2023/invoices")).unwrap();
    fs::create_dir_all(root.join("archive/old")).unwrap();
    fs::write(root.join("finance/2024/invoices/report.xlsx"), b"a").unwrap();
    fs::write(root.join("finance/2023/invoices/report.xlsx"), b"b").unwrap();
    fs::write(root.join("archive/old/report.xlsx"), b"c").unwrap();

    let cat_path = dir.path().join("c.uts");
    build(&cat_path, &roots(&root, &[]), None).unwrap();
    let cat = Catalog::open(&cat_path).unwrap();

    // --dir limits results to the 2024 subtree only.
    let mut q = Query::new();
    q.name = Some("report".into());
    q.dir = Some("finance/2024".into());
    let page = cat.search(q).unwrap();
    assert_eq!(page.hits.len(), 1, "hits={:?}", page.hits);
    assert!(page.hits[0].rel.contains("finance/2024/invoices/report.xlsx"));

    // Deeper --dir still works.
    let mut q = Query::new();
    q.name = Some("report".into());
    q.dir = Some("finance/2024/invoices".into());
    assert_eq!(cat.search(q).unwrap().hits.len(), 1);

    // A --dir with no name returns every file under the subtree.
    let mut q = Query::new();
    q.dir = Some("finance/2023".into());
    let page = cat.search(q).unwrap();
    assert!(page.hits.iter().all(|h| h.rel.contains("finance/2023")));
    assert!(page.hits.iter().any(|h| h.rel.ends_with("report.xlsx")));

    // Backslash separators and trailing slash normalize the same way.
    let mut q = Query::new();
    q.name = Some("report".into());
    q.dir = Some("finance\\2024\\".into());
    assert_eq!(cat.search(q).unwrap().hits.len(), 1);

    // A --dir that names no real directory yields no hits (explicit, not a
    // silent whole-catalog scan).
    let mut q = Query::new();
    q.name = Some("report".into());
    q.dir = Some("finance/2099".into());
    assert_eq!(cat.search(q).unwrap().hits.len(), 0);

    // --path stays a substring match on the relative path: "invoices" matches
    // both files that live under an "invoices" directory.
    let mut q = Query::new();
    q.name = Some("report".into());
    q.path = Some("invoices".into());
    assert_eq!(cat.search(q).unwrap().hits.len(), 2);
}
