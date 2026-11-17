//! Export format tests.

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::sync::atomic::AtomicBool;

use super::*;
use crate::scan::{Progress, ScanOptions, scan};
use crate::tree::test_util::{dir, file};

fn to_string(tree: &Tree, at: i64) -> String {
    let mut buf = Vec::new();
    write(tree, at, &mut buf).unwrap();
    String::from_utf8(buf).unwrap()
}

fn assert_same_tree(a: &Tree, b: &Tree) {
    assert_eq!(a.live(), b.live());
    assert_eq!(a.root_path(), b.root_path());
    let (mut a, mut b) = (a.clone(), b.clone());
    a.sort(crate::tree::SortKey::Name, false);
    b.sort(crate::tree::SortKey::Name, false);
    let (a, b) = (&a, &b);
    let pa: Vec<_> = std::iter::once(NodeId::ROOT)
        .chain(a.descendants_preorder(NodeId::ROOT))
        .collect();
    let pb: Vec<_> = std::iter::once(NodeId::ROOT)
        .chain(b.descendants_preorder(NodeId::ROOT))
        .collect();
    let la: Vec<_> = pa.iter().map(|&x| a.rel_path(x)).collect();
    let lb: Vec<_> = pb.iter().map(|&x| b.rel_path(x)).collect();
    assert_eq!(la, lb);
    for (x, y) in pa.iter().zip(&pb) {
        let (nx, ny) = (a.node(*x), b.node(*y));
        assert_eq!(a.rel_path(*x), b.rel_path(*y));
        assert_eq!(
            (nx.disk, nx.apparent, nx.files, nx.dirs, nx.errors),
            (ny.disk, ny.apparent, ny.files, ny.dirs, ny.errors)
        );
        assert_eq!(
            (nx.kind, nx.mtime, nx.mode, nx.ino, nx.flags),
            (ny.kind, ny.mtime, ny.mode, ny.ino, ny.flags)
        );
        assert_eq!(a.dev(*x), b.dev(*y));
    }
}

#[test]
fn round_trip_of_real_scan_is_identical() {
    let tmp = tempfile::tempdir().unwrap();
    let r = tmp.path();
    std::fs::create_dir_all(r.join("a/b/c")).unwrap();
    std::fs::write(r.join("a/one"), vec![0u8; 5000]).unwrap();
    std::fs::write(r.join("a/b/c/two"), vec![0u8; 70]).unwrap();
    std::fs::hard_link(r.join("a/one"), r.join("a/b/link")).unwrap();
    std::os::unix::fs::symlink("a/one", r.join("sym")).unwrap();
    let tree = scan(
        r,
        &ScanOptions::default(),
        &Progress::default(),
        &AtomicBool::new(false),
    )
    .unwrap();

    let first = to_string(&tree, 1234);
    let back = read(first.as_bytes()).unwrap();
    assert_eq!(back.scanned_at, 1234);
    assert_same_tree(&tree, &back.tree);
    let second = to_string(&back.tree, back.scanned_at);
    assert_eq!(first, second, "byte-identical round trip");
}

#[test]
fn non_utf8_names_and_deep_trees_survive() {
    let mut t = Tree::new("/r", dir());
    let mut cur = NodeId::ROOT;
    for i in 0..300 {
        cur = t.add_child(cur, OsStr::new(&format!("d{i}")), &dir());
    }
    t.add_child(cur, OsStr::from_bytes(b"bad\xffname"), &file(9));
    t.aggregate();
    let text = to_string(&t, 0);
    let back = read(text.as_bytes()).unwrap().tree;
    assert_same_tree(&t, &back);
    assert_eq!(back.root().disk, 9);
    assert_eq!(to_string(&back, 0), text);
}

#[test]
fn removed_nodes_are_not_exported() {
    let mut t = crate::tree::test_util::sample();
    let a = t.find(Path::new("a")).unwrap();
    t.remove_subtree(a);
    let back = read(to_string(&t, 0).as_bytes()).unwrap().tree;
    assert_eq!(back.live(), 3);
    assert_eq!(back.root().disk, 50);
}

#[test]
fn rejects_foreign_or_broken_files() {
    assert!(read(&b"{}"[..]).is_err());
    assert!(
        read(&br#"{"format":"other","version":1,"root":"/","scanned_at":0,"nodes":[]}"#[..])
            .is_err()
    );
    assert!(
        read(&br#"{"format":"ramas","version":99,"root":"/","scanned_at":0,"nodes":[]}"#[..])
            .is_err()
    );
    assert!(
        read(&br#"{"format":"ramas","version":1,"root":"/","scanned_at":0,"nodes":[]}"#[..])
            .is_err()
    );
    let orphan = br#"{"format":"ramas","version":1,"root":"/","scanned_at":0,"nodes":[
        {"p":null,"n":"/","k":"d","d":0,"a":0,"m":0,"mode":0,"dev":0,"ino":0,"f":0},
        {"p":7,"n":"x","k":"f","d":0,"a":0,"m":0,"mode":0,"dev":0,"ino":0,"f":0}]}"#;
    assert!(read(&orphan[..]).is_err());
}

#[test]
fn output_does_not_depend_on_display_order() {
    let mut t = crate::tree::test_util::sample();
    let by_size = to_string(&t, 0);
    t.sort(crate::tree::SortKey::Name, false);
    assert_eq!(to_string(&t, 0), by_size);
    t.sort(crate::tree::SortKey::Mtime, true);
    assert_eq!(to_string(&t, 0), by_size);
}

#[test]
fn file_helpers() {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("scan.json");
    let t = crate::tree::test_util::sample();
    write_file(&t, 5, &p).unwrap();
    let back = read_file(&p).unwrap();
    assert_same_tree(&t, &back.tree);
    assert!(read_file(&tmp.path().join("missing.json")).is_err());
}
