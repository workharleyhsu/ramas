//! Delete tests. They only ever touch `tempfile` directories.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::sync::atomic::AtomicBool;

use super::*;
use crate::actions::marks::Marks;
use crate::scan::{Progress, ScanOptions, scan};

fn scan_dir(root: &Path) -> Tree {
    scan(
        root,
        &ScanOptions::default(),
        &Progress::default(),
        &AtomicBool::new(false),
    )
    .unwrap()
}

fn target(tree: &Tree, rel: &str) -> Target {
    Target::of(tree, tree.find(Path::new(rel)).unwrap(), false)
}

fn guard(root: &Path) -> Guard {
    Guard::with_home(root, None).unwrap()
}

/// root/{dir/{f1, sub/f2}, file, link -> outside/secret, dirlink -> outside}
fn fixture() -> (tempfile::TempDir, tempfile::TempDir) {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let r = root.path();
    fs::create_dir_all(r.join("dir/sub")).unwrap();
    fs::write(r.join("dir/f1"), b"1111").unwrap();
    fs::write(r.join("dir/sub/f2"), b"2222").unwrap();
    fs::write(r.join("file"), b"data").unwrap();
    fs::write(outside.path().join("secret"), b"keep me").unwrap();
    symlink(outside.path().join("secret"), r.join("link")).unwrap();
    symlink(outside.path(), r.join("dirlink")).unwrap();
    (root, outside)
}

#[test]
fn permanent_delete_of_file_and_directory() {
    let (root, _out) = fixture();
    let tree = scan_dir(root.path());
    let targets = [target(&tree, "file"), target(&tree, "dir")];
    let res = delete_all(&targets, Method::Permanent, &guard(root.path()), &FsRemover);
    assert!(res.iter().all(|o| o.result.is_ok()), "{res:?}");
    assert!(!root.path().join("file").exists());
    assert!(!root.path().join("dir").exists());
}

#[test]
fn symlinks_are_removed_not_followed() {
    let (root, out) = fixture();
    let tree = scan_dir(root.path());
    let targets = [target(&tree, "link"), target(&tree, "dirlink")];
    let res = delete_all(&targets, Method::Permanent, &guard(root.path()), &FsRemover);
    assert!(res.iter().all(|o| o.result.is_ok()), "{res:?}");
    assert!(fs::symlink_metadata(root.path().join("link")).is_err());
    assert!(fs::symlink_metadata(root.path().join("dirlink")).is_err());
    assert_eq!(fs::read(out.path().join("secret")).unwrap(), b"keep me");
}

#[test]
fn scan_root_is_refused() {
    let (root, _out) = fixture();
    let tree = scan_dir(root.path());
    let t = Target::of(&tree, NodeId::ROOT, false);
    let err = guard(root.path()).validate(&t).unwrap_err();
    assert!(matches!(err, DeleteError::ScanRoot), "{err}");
    assert!(root.path().exists());
}

#[test]
fn filesystem_root_is_refused() {
    let (root, _out) = fixture();
    let t = Target {
        node: NodeId(1),
        path: "/".into(),
        dev: 0,
        ino: 0,
        size: 0,
    };
    assert!(matches!(
        guard(root.path()).validate(&t),
        Err(DeleteError::FilesystemRoot)
    ));
}

#[test]
fn home_and_its_parents_are_refused() {
    let (root, _out) = fixture();
    let home = root.path().join("dir/sub").canonicalize().unwrap();
    let g = Guard::with_home(root.path(), Some(home)).unwrap();
    let tree = scan_dir(root.path());
    for rel in ["dir/sub", "dir"] {
        let err = g.validate(&target(&tree, rel)).unwrap_err();
        assert!(matches!(err, DeleteError::Home), "{rel}: {err}");
    }
    assert!(g.validate(&target(&tree, "file")).is_ok());
    assert!(root.path().join("dir/sub/f2").exists());
}

#[test]
fn paths_outside_root_are_refused() {
    let (root, out) = fixture();
    let other = scan_dir(out.path());
    let t = target(&other, "secret");
    let err = guard(root.path()).validate(&t).unwrap_err();
    assert!(matches!(err, DeleteError::OutsideRoot), "{err}");

    // Escaping through a symlinked parent directory is also outside.
    let mut t = t.clone();
    t.path = root.path().join("dirlink/secret");
    let err = guard(root.path()).validate(&t).unwrap_err();
    assert!(matches!(err, DeleteError::OutsideRoot), "{err}");
    assert!(out.path().join("secret").exists());
}

#[test]
fn parent_dir_components_are_refused() {
    let (root, _out) = fixture();
    let tree = scan_dir(root.path());
    let mut t = target(&tree, "file");
    t.path = root.path().join("dir/../file");
    assert!(matches!(
        guard(root.path()).validate(&t),
        Err(DeleteError::BadPath)
    ));
}

#[test]
fn replaced_file_is_refused() {
    let (root, _out) = fixture();
    let tree = scan_dir(root.path());
    let t = target(&tree, "file");
    fs::remove_file(root.path().join("file")).unwrap();
    // Keep the old inode alive so the new file cannot reuse its number.
    let _hold = fs::write(root.path().join("holder"), b"x");
    fs::write(root.path().join("file"), b"new content").unwrap();
    let res = delete_all(&[t], Method::Permanent, &guard(root.path()), &FsRemover);
    let err = res[0].result.as_ref().unwrap_err();
    assert!(
        matches!(err, DeleteError::Changed),
        "unexpected: {err} (inode reuse is possible but unlikely)"
    );
    assert!(root.path().join("file").exists());
}

#[test]
fn missing_file_reports_failure_and_others_continue() {
    let (root, _out) = fixture();
    let tree = scan_dir(root.path());
    let gone = target(&tree, "dir/f1");
    fs::remove_file(root.path().join("dir/f1")).unwrap();
    let res = delete_all(
        &[gone, target(&tree, "file")],
        Method::Permanent,
        &guard(root.path()),
        &FsRemover,
    );
    assert!(matches!(res[0].result, Err(DeleteError::Missing(_))));
    assert!(res[1].result.is_ok());
}

#[test]
fn dedup_parent_child_then_tree_update() {
    let (root, _out) = fixture();
    let mut tree = scan_dir(root.path());
    let total_before = tree.root().disk;
    let dir = tree.find(Path::new("dir")).unwrap();
    let f1 = tree.find(Path::new("dir/f1")).unwrap();
    let mut marks = Marks::default();
    marks.toggle(&tree, f1);
    marks.toggle(&tree, dir);
    assert_eq!(marks.ids(), vec![dir], "child mark absorbed");
    let targets: Vec<Target> = marks
        .ids()
        .iter()
        .map(|&id| Target::of(&tree, id, false))
        .collect();
    let freed: u64 = targets.iter().map(|t| t.size).sum();
    let res = delete_all(&targets, Method::Permanent, &guard(root.path()), &FsRemover);
    assert!(res[0].result.is_ok());
    tree.remove_subtree(dir);
    assert_eq!(tree.root().disk, total_before - freed);
    assert!(tree.find(Path::new("dir")).is_none());
}

/// Trash in a child process so `XDG_DATA_HOME` can point inside the temp
/// directory without touching the real trash (and without `set_var`).
#[test]
fn trash_moves_into_isolated_trash() {
    let (root, _out) = fixture();
    let data_home = root.path().join("xdg");
    fs::create_dir(&data_home).unwrap();
    let exe = std::env::current_exe().unwrap();
    let status = std::process::Command::new(exe)
        .args([
            "--exact",
            "actions::delete::tests::trash_child",
            "--ignored",
            "--nocapture",
        ])
        .env("XDG_DATA_HOME", &data_home)
        .env("RAMAS_TRASH_TEST_ROOT", root.path())
        .status()
        .unwrap();
    assert!(status.success());
    assert!(!root.path().join("file").exists());
    assert!(!root.path().join("dir").exists());
    let files = data_home.join("Trash/files");
    assert!(files.join("file").exists(), "file in isolated trash");
    assert!(files.join("dir/sub/f2").exists(), "dir in isolated trash");
}

#[test]
#[ignore = "runs only as the child of trash_moves_into_isolated_trash"]
fn trash_child() {
    let (Some(root), Some(xdg)) = (
        std::env::var_os("RAMAS_TRASH_TEST_ROOT"),
        std::env::var_os("XDG_DATA_HOME"),
    ) else {
        panic!("missing test environment");
    };
    let root = Path::new(&root);
    assert!(
        Path::new(&xdg).starts_with(root),
        "trash must be inside the temp dir"
    );
    let tree = scan_dir(root);
    let targets = [target(&tree, "file"), target(&tree, "dir")];
    let res = delete_all(&targets, Method::Trash, &guard(root), &FsRemover);
    for o in &res {
        assert!(o.result.is_ok(), "{:?}", o.result);
    }
}
