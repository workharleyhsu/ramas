//! UI-level tests of marking, the delete flow and rescans. Only temporary
//! directories are touched; trash is never used here (see actions tests).

use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use crossterm::event::KeyCode;

use crate::scan::{Progress, ScanOptions, scan};
use crate::ui::app::test_util::{key, press, settings};
use crate::ui::app::{App, Button, Overlay};
use crate::ui::render::test_util::render;

fn fixture() -> (tempfile::TempDir, App) {
    let tmp = tempfile::tempdir().unwrap();
    let r = tmp.path();
    fs::create_dir_all(r.join("big/inner")).unwrap();
    fs::write(r.join("big/inner/a"), vec![1u8; 40_000]).unwrap();
    fs::write(r.join("big/b"), vec![1u8; 30_000]).unwrap();
    fs::write(r.join("small"), vec![1u8; 100]).unwrap();
    let tree = scan(
        r,
        &ScanOptions::default(),
        &Progress::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    (tmp, App::new(tree, settings()))
}

fn wait(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.busy() {
        app.tick();
        assert!(Instant::now() < deadline, "job timed out");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn select(app: &mut App, rel: &str) {
    let id = app.tree.find(Path::new(rel)).unwrap();
    app.reveal(id);
}

#[test]
fn space_marks_moves_down_and_status_counts() {
    let (_tmp, mut app) = fixture();
    select(&mut app, "big");
    press(&mut app, " ");
    let big = app.tree.find(Path::new("big")).unwrap();
    assert!(app.marks.is_marked(big));
    assert_ne!(app.selected(), big, "cursor advanced");
    let (n, bytes) = app.marked_summary();
    assert_eq!(n, 1);
    assert_eq!(bytes, app.tree.node(big).disk);
    let body = render(&mut app, 120, 20).join("\n");
    assert!(body.contains("1 marked — "), "{body}");
}

#[test]
fn root_cannot_be_marked() {
    let (_tmp, mut app) = fixture();
    press(&mut app, " ");
    assert!(app.marks.is_empty());
    assert_eq!(app.message.as_deref(), Some(app.s.cannot_mark_root));
}

#[test]
fn delete_is_blocked_when_read_only_imported_or_empty() {
    let (_tmp, mut app) = fixture();
    press(&mut app, "d");
    assert_eq!(app.overlay, Overlay::None);
    assert_eq!(app.message.as_deref(), Some(app.s.nothing_marked));

    select(&mut app, "small");
    press(&mut app, " ");
    app.read_only = true;
    press(&mut app, "d");
    assert_eq!(app.overlay, Overlay::None);
    assert_eq!(app.message.as_deref(), Some(app.s.delete_disabled_ro));

    app.read_only = false;
    app.imported = true;
    press(&mut app, "d");
    assert_eq!(app.overlay, Overlay::None);
    assert_eq!(app.message.as_deref(), Some(app.s.delete_disabled_import));
}

#[test]
fn dialog_focus_starts_on_cancel_and_cancel_keeps_files() {
    let (tmp, mut app) = fixture();
    select(&mut app, "small");
    press(&mut app, " d");
    assert_eq!(
        app.overlay,
        Overlay::Confirm {
            focus: Button::Cancel
        }
    );
    let body = render(&mut app, 120, 30).join("\n");
    assert!(
        body.contains("[ Trash ]") && body.contains("[ Cancel ]"),
        "{body}"
    );
    assert!(body.contains("Total to free"), "{body}");
    app.on_key(key(KeyCode::Enter));
    assert_eq!(app.overlay, Overlay::None);
    assert!(!app.busy());
    assert!(tmp.path().join("small").exists());
}

#[test]
fn permanent_delete_requires_typed_word_and_updates_tree() {
    let (tmp, mut app) = fixture();
    let total = app.tree.root().disk;
    select(&mut app, "big/inner/a");
    press(&mut app, " ");
    select(&mut app, "big");
    press(&mut app, " "); // absorbs the mark on big/inner/a
    assert_eq!(app.marks.len(), 1);
    let big = app.tree.find(Path::new("big")).unwrap();
    let freed = app.tree.node(big).disk;

    press(&mut app, "d");
    app.on_key(key(KeyCode::Left)); // Cancel -> Permanent
    assert_eq!(
        app.overlay,
        Overlay::Confirm {
            focus: Button::Permanent
        }
    );
    app.on_key(key(KeyCode::Enter));
    assert!(matches!(app.overlay, Overlay::TypeConfirm { .. }));
    press(&mut app, "nope");
    app.on_key(key(KeyCode::Enter));
    assert!(matches!(&app.overlay, Overlay::TypeConfirm { input } if input.is_empty()));
    assert!(!app.busy(), "wrong word deletes nothing");
    assert!(tmp.path().join("big").exists());

    press(&mut app, "delete");
    app.on_key(key(KeyCode::Enter));
    wait(&mut app);
    let Overlay::Summary(sum) = &app.overlay else {
        panic!("expected summary, got {:?}", app.overlay);
    };
    assert_eq!(sum.removed, 1);
    assert_eq!(sum.freed, freed);
    assert!(sum.failed.is_empty());
    assert!(!tmp.path().join("big").exists());
    assert!(tmp.path().join("small").exists());
    assert_eq!(app.tree.root().disk, total - freed, "ancestors updated");
    assert!(app.tree.find(Path::new("big")).is_none());
    assert!(app.marks.is_empty());
    let body = render(&mut app, 120, 30).join("\n");
    assert!(body.contains("Freed"), "{body}");
    app.on_key(key(KeyCode::Esc));
    assert_eq!(app.overlay, Overlay::None);
}

#[test]
fn spanish_confirmation_word() {
    let (tmp, mut app) = fixture();
    app.s = &crate::i18n::ES;
    select(&mut app, "small");
    press(&mut app, " d");
    app.on_key(key(KeyCode::Left));
    app.on_key(key(KeyCode::Enter));
    press(&mut app, "delete");
    app.on_key(key(KeyCode::Enter));
    assert!(!app.busy(), "English word rejected in Spanish UI");
    press(&mut app, "borrar");
    app.on_key(key(KeyCode::Enter));
    wait(&mut app);
    assert!(!tmp.path().join("small").exists());
}

#[test]
fn failures_are_reported_with_reason() {
    let (tmp, mut app) = fixture();
    select(&mut app, "small");
    press(&mut app, " ");
    fs::remove_file(tmp.path().join("small")).unwrap();
    press(&mut app, "d");
    app.on_key(key(KeyCode::Left));
    app.on_key(key(KeyCode::Enter));
    press(&mut app, "delete");
    app.on_key(key(KeyCode::Enter));
    wait(&mut app);
    let Overlay::Summary(sum) = &app.overlay else {
        panic!("expected summary");
    };
    assert_eq!(sum.removed, 0);
    assert_eq!(sum.failed.len(), 1);
    assert!(
        sum.failed[0].1.contains("no longer exists"),
        "{:?}",
        sum.failed
    );
    assert!(
        app.tree.find(Path::new("small")).is_some(),
        "failed item stays"
    );
}

#[test]
fn rescan_picks_up_changes_and_updates_totals() {
    let (tmp, mut app) = fixture();
    fs::write(tmp.path().join("big/new"), vec![1u8; 9_999]).unwrap();
    select(&mut app, "big");
    press(&mut app, "r");
    assert!(app.busy());
    wait(&mut app);
    assert!(app.tree.find(Path::new("big/new")).is_some());
    let fresh = scan(
        tmp.path(),
        &ScanOptions::default(),
        &Progress::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(app.tree.root().apparent, fresh.root().apparent);
    assert_eq!(app.tree.root().disk, fresh.root().disk);
    assert_eq!(app.tree.root().files, fresh.root().files);
    let big = app.tree.find(Path::new("big")).unwrap();
    assert_eq!(app.selected(), big);
}

#[test]
fn rescanning_root_clears_imported_mode() {
    let (_tmp, mut app) = fixture();
    app.imported = true;
    app.move_to(0);
    press(&mut app, "r");
    wait(&mut app);
    assert!(!app.imported);
}

#[test]
fn rescan_of_a_file_is_refused() {
    let (_tmp, mut app) = fixture();
    select(&mut app, "small");
    press(&mut app, "r");
    assert!(!app.busy());
    assert_eq!(app.message.as_deref(), Some(app.s.rescan_not_dir));
}

#[test]
fn e_exports_a_reimportable_file() {
    let (tmp, mut app) = fixture();
    let out = tempfile::tempdir().unwrap();
    app.export_dir = out.path().to_path_buf();
    app.scanned_at = 42;
    press(&mut app, "e");
    let msg = app.message.clone().unwrap();
    assert!(msg.starts_with("Exported to"), "{msg}");
    let files: Vec<_> = fs::read_dir(out.path()).unwrap().collect();
    assert_eq!(files.len(), 1);
    let path = files[0].as_ref().unwrap().path();
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    assert!(
        name.starts_with("ramas-") && name.ends_with(".json"),
        "{name}"
    );
    let back = crate::export::read_file(&path).unwrap();
    assert_eq!(back.scanned_at, 42);
    assert_eq!(back.tree.root().disk, app.tree.root().disk);
    assert_eq!(back.tree.root_path(), tmp.path().canonicalize().unwrap());
}

#[test]
fn diff_highlights_changes_in_tree_and_panel() {
    let (tmp, app) = fixture();
    let old = app.tree.clone();
    fs::write(tmp.path().join("big/b"), vec![1u8; 90_000]).unwrap();
    fs::write(tmp.path().join("fresh"), vec![1u8; 10]).unwrap();
    fs::remove_file(tmp.path().join("small")).unwrap();
    let tree = scan(
        tmp.path(),
        &ScanOptions::default(),
        &Progress::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let mut s = settings();
    s.diff = Some((old, "before.json".into()));
    let mut app = App::new(tree, s);
    let diff = app.diff.as_ref().unwrap();
    assert_eq!(diff.removed, 1, "small is gone");
    select(&mut app, "big/b");
    let body = render(&mut app, 140, 30).join("\n");
    assert!(body.contains("+"), "{body}");
    assert!(body.contains("new"), "fresh is new: {body}");
    assert!(body.contains("diff vs before.json"), "{body}");
    assert!(body.contains("grew +"), "panel shows the change: {body}");
}

#[test]
fn imported_tree_blocks_delete_until_root_rescan() {
    let (tmp, app) = fixture();
    let mut buf = Vec::new();
    crate::export::write(&app.tree, 1, &mut buf).unwrap();
    let imported = crate::export::read(&buf[..]).unwrap();
    let mut s = settings();
    s.imported = true;
    let mut app = App::new(imported.tree, s);
    select(&mut app, "small");
    press(&mut app, " d");
    assert_eq!(app.overlay, Overlay::None);
    assert_eq!(app.message.as_deref(), Some(app.s.delete_disabled_import));
    app.move_to(0);
    press(&mut app, "r");
    wait(&mut app);
    assert!(!app.imported);
    select(&mut app, "small");
    press(&mut app, " d");
    assert!(matches!(app.overlay, Overlay::Confirm { .. }));
    assert!(tmp.path().join("small").exists());
}

#[test]
fn unreadable_folder_is_explained_not_alarming() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    let locked = tmp.path().join("locked");
    fs::create_dir(&locked).unwrap();
    fs::write(locked.join("inside"), b"x").unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    let readable = fs::read_dir(&locked).is_ok(); // running as root
    let tree = scan(
        tmp.path(),
        &ScanOptions::default(),
        &Progress::default(),
        &AtomicBool::new(false),
    );
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    if readable {
        return;
    }
    let mut app = App::new(tree.unwrap(), settings());
    select(&mut app, "locked");
    let body = render(&mut app, 140, 30).join("\n");
    assert!(body.contains("⊘ locked/ [no access]"), "{body}");
    assert!(body.contains("Nothing is damaged"), "{body}");
    assert!(body.contains("sudo ramas --read-only"), "{body}");
    assert!(body.contains("Owner"), "{body}");
    assert!(body.contains("Unreadable 1"), "{body}");
    assert!(!body.contains("Errors"), "{body}");
    app.move_to(0);
    let body = render(&mut app, 140, 30).join("\n");
    assert!(body.contains("Unreadable inside: 1"), "{body}");
}
