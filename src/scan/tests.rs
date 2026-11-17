//! Scanner tests against temporary trees with known sizes.

use std::fs;
use std::io::Write;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::path::Path;
use std::sync::atomic::AtomicBool;

use super::*;
use crate::tree::{Kind, flags};

fn run(root: &Path) -> Tree {
    let cancel = AtomicBool::new(false);
    scan(root, &ScanOptions::default(), &Progress::default(), &cancel).expect("scan")
}

fn write(path: &Path, len: usize) {
    let mut f = fs::File::create(path).unwrap();
    f.write_all(&vec![0xAB; len]).unwrap();
    f.sync_all().unwrap();
}

fn disk(path: &Path) -> u64 {
    fs::symlink_metadata(path).unwrap().blocks() * 512
}

#[test]
fn apparent_and_disk_sizes() {
    let tmp = tempfile::tempdir().unwrap();
    let r = tmp.path();
    fs::create_dir_all(r.join("a/b")).unwrap();
    write(&r.join("a/one"), 1000);
    write(&r.join("a/b/two"), 5000);
    write(&r.join("three"), 12345);

    let t = run(r);
    let root = t.root();
    assert_eq!(t.root_path(), fs::canonicalize(r).unwrap());
    let dirs_apparent: u64 = ["", "a", "a/b"]
        .iter()
        .map(|d| fs::metadata(r.join(d)).unwrap().size())
        .sum();
    assert_eq!(root.apparent, 1000 + 5000 + 12345 + dirs_apparent);
    let expected_disk: u64 = ["", "a", "a/b", "a/one", "a/b/two", "three"]
        .iter()
        .map(|p| disk(&r.join(p)))
        .sum();
    assert_eq!(root.disk, expected_disk);
    assert_eq!(root.files, 3);
    assert_eq!(root.dirs, 2);
    let b = t.find(Path::new("a/b")).unwrap();
    assert_eq!(t.node(b).files, 1);
    assert_eq!(t.node(b).kind, Kind::Dir);
}

#[test]
fn sparse_file_uses_little_disk() {
    let tmp = tempfile::tempdir().unwrap();
    let f = fs::File::create(tmp.path().join("sparse")).unwrap();
    f.set_len(64 * 1024 * 1024).unwrap();
    let t = run(tmp.path());
    let id = t.find(Path::new("sparse")).unwrap();
    let n = t.node(id);
    assert_eq!(n.apparent, 64 * 1024 * 1024);
    assert!(n.disk < n.apparent);
}

#[test]
fn symlinks_are_not_followed() {
    let tmp = tempfile::tempdir().unwrap();
    let r = tmp.path();
    fs::create_dir(r.join("real")).unwrap();
    write(&r.join("real/big"), 100_000);
    symlink(r.join("real"), r.join("dirlink")).unwrap();
    symlink(r.join("real/big"), r.join("filelink")).unwrap();

    let t = run(r);
    let dl = t.find(Path::new("dirlink")).unwrap();
    assert_eq!(t.node(dl).kind, Kind::Symlink);
    assert!(!t.has_children(dl));
    let fl = t.find(Path::new("filelink")).unwrap();
    assert_eq!(t.node(fl).kind, Kind::Symlink);
    assert!(
        t.node(fl).apparent < 1000,
        "symlink size is its target path"
    );
    assert_eq!(t.root().files, 3);
}

#[test]
fn hard_links_counted_once() {
    let tmp = tempfile::tempdir().unwrap();
    let r = tmp.path();
    write(&r.join("orig"), 50_000);
    fs::hard_link(r.join("orig"), r.join("link1")).unwrap();
    fs::create_dir(r.join("sub")).unwrap();
    fs::hard_link(r.join("orig"), r.join("sub/link2")).unwrap();

    let t = run(r);
    let file_apparent: u64 = (0..t.len() as u32)
        .map(crate::tree::NodeId)
        .filter(|&id| t.node(id).kind == Kind::File)
        .map(|id| t.node(id).apparent)
        .sum();
    assert_eq!(file_apparent, 50_000);
    let dups = (0..t.len() as u32)
        .filter(|&i| t.node(crate::tree::NodeId(i)).has(flags::HARDLINK))
        .count();
    assert_eq!(dups, 2);
    assert_eq!(t.root().files, 3);
}

#[test]
fn unreadable_directory_is_marked_not_fatal() {
    let tmp = tempfile::tempdir().unwrap();
    let r = tmp.path();
    let locked = r.join("locked");
    fs::create_dir(&locked).unwrap();
    write(&locked.join("hidden"), 10);
    write(&r.join("visible"), 10);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    let readable = fs::read_dir(&locked).is_ok(); // true when running as root

    let progress = Progress::default();
    let t = scan(
        r,
        &ScanOptions::default(),
        &progress,
        &AtomicBool::new(false),
    );
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    let t = t.unwrap();
    if readable {
        return;
    }
    let id = t.find(Path::new("locked")).unwrap();
    assert!(t.node(id).has(flags::ERROR));
    assert_eq!(t.root().errors, 1);
    assert_eq!(progress.snapshot().errors, 1);
    assert!(t.find(Path::new("visible")).is_some());
}

#[test]
fn single_file_root() {
    let tmp = tempfile::tempdir().unwrap();
    let f = tmp.path().join("only");
    write(&f, 777);
    let t = run(&f);
    assert_eq!(t.len(), 1);
    assert_eq!(t.root().apparent, 777);
    assert_eq!(t.root().kind, Kind::File);
}

#[test]
fn missing_root_is_an_error() {
    let err = scan(
        Path::new("/definitely/not/here"),
        &ScanOptions::default(),
        &Progress::default(),
        &AtomicBool::new(false),
    );
    assert!(matches!(err, Err(ScanError::Root { .. })));
}

#[test]
fn progress_counts_every_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let r = tmp.path();
    fs::create_dir(r.join("d")).unwrap();
    for i in 0..10 {
        write(&r.join(format!("d/f{i}")), 100);
    }
    let progress = Progress::default();
    let t = scan(
        r,
        &ScanOptions::default(),
        &progress,
        &AtomicBool::new(false),
    )
    .unwrap();
    let snap = progress.snapshot();
    assert_eq!(snap.entries, 12, "root + d + 10 files");
    assert_eq!(snap.bytes, t.root().disk);
    assert_eq!(snap.errors, 0);
    assert!(snap.current.starts_with(t.root_path()));
}

#[test]
fn cancelled_scan_stops_early() {
    let tmp = tempfile::tempdir().unwrap();
    let r = tmp.path();
    for d in 0..20 {
        fs::create_dir(r.join(format!("d{d}"))).unwrap();
        for f in 0..20 {
            write(&r.join(format!("d{d}/f{f}")), 1);
        }
    }
    let progress = Progress::default();
    let cancel = AtomicBool::new(true);
    let res = scan(r, &ScanOptions::default(), &progress, &cancel);
    assert!(matches!(res, Err(ScanError::Cancelled)));
    assert!(progress.snapshot().entries < 400);
}

#[test]
fn cancel_from_another_thread() {
    let tmp = tempfile::tempdir().unwrap();
    let r = tmp.path().to_path_buf();
    fs::create_dir(r.join("d")).unwrap();
    let progress = std::sync::Arc::new(Progress::default());
    let cancel = std::sync::Arc::new(AtomicBool::new(false));
    let (p, c) = (progress.clone(), cancel.clone());
    let handle = std::thread::spawn(move || scan(&r, &ScanOptions::default(), &p, &c));
    cancel.store(true, std::sync::atomic::Ordering::Relaxed);
    // Either the scan finished before the flag or it reports cancellation;
    // it must never hang or panic.
    let res = handle.join().unwrap();
    assert!(res.is_ok() || matches!(res, Err(ScanError::Cancelled)));
}
