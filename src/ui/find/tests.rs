//! Search and filter behaviour in the TUI.

use std::ffi::OsStr;
use std::path::Path;

use crossterm::event::KeyCode;

use crate::tree::test_util::{dir, file};
use crate::tree::{NodeId, Tree};
use crate::ui::app::test_util::{key, press, settings};
use crate::ui::app::{App, Overlay};
use crate::ui::render::test_util::render;

/// /r/{photos/{2023/{beach.jpg, party.jpg}, cat.png}, work/{deep/{report.pdf}}, notes.txt}
fn app() -> App {
    let mut t = Tree::new("/r", dir());
    let photos = t.add_child(NodeId::ROOT, OsStr::new("photos"), &dir());
    let y = t.add_child(photos, OsStr::new("2023"), &dir());
    t.add_child(y, OsStr::new("beach.jpg"), &file(4000));
    t.add_child(y, OsStr::new("party.jpg"), &file(3000));
    t.add_child(photos, OsStr::new("cat.png"), &file(500));
    let work = t.add_child(NodeId::ROOT, OsStr::new("work"), &dir());
    let deep = t.add_child(work, OsStr::new("deep"), &dir());
    t.add_child(deep, OsStr::new("report.pdf"), &file(100));
    t.add_child(NodeId::ROOT, OsStr::new("notes.txt"), &file(10));
    t.aggregate();
    App::new(t, settings())
}

fn name(app: &App) -> String {
    app.tree.name(app.selected()).to_string_lossy().into_owned()
}

fn open(app: &App, rel: &str) -> bool {
    app.is_open(app.tree.find(Path::new(rel)).unwrap())
}

#[test]
fn incremental_search_jumps_and_expands_branch() {
    let mut app = app();
    assert!(!open(&app, "work/deep"));
    press(&mut app, "/");
    assert!(matches!(app.overlay, Overlay::Search { .. }));
    press(&mut app, "rep");
    assert_eq!(name(&app), "report.pdf");
    assert!(open(&app, "work"), "ancestors auto-expanded");
    assert!(open(&app, "work/deep"));
    let body = render(&mut app, 100, 20).join("\n");
    assert!(body.contains("Search: rep"), "{body}");
    assert!(body.contains("1/1"), "{body}");
    app.on_key(key(KeyCode::Enter));
    assert_eq!(app.overlay, Overlay::None);
    assert_eq!(name(&app), "report.pdf");
}

#[test]
fn n_and_shift_n_cycle_through_hits() {
    let mut app = app();
    press(&mut app, "/jpg");
    app.on_key(key(KeyCode::Enter));
    let hits = app.search.as_ref().unwrap().hits.len();
    assert_eq!(hits, 2);
    let first = name(&app);
    press(&mut app, "n");
    let second = name(&app);
    assert_ne!(first, second);
    assert!(second.ends_with(".jpg"));
    press(&mut app, "n");
    assert_eq!(name(&app), first, "wraps around");
    press(&mut app, "N");
    assert_eq!(name(&app), second);
}

#[test]
fn enter_expands_all_branches_with_hits() {
    let mut app = app();
    press(&mut app, "/a"); // beach, party, cat, ... many
    app.on_key(key(KeyCode::Enter));
    assert!(open(&app, "photos"));
    assert!(open(&app, "photos/2023"));
    assert!(
        open(&app, "work/deep"),
        "report.pdf contains an 'a'? no: p-o-r-t"
    );
}

#[test]
fn esc_cancels_and_returns_to_origin() {
    let mut app = app();
    press(&mut app, "j"); // photos
    press(&mut app, "/report");
    assert_eq!(name(&app), "report.pdf");
    app.on_key(key(KeyCode::Esc));
    assert_eq!(name(&app), "photos");
    assert!(app.search.is_none());
}

#[test]
fn no_match_reports_and_keeps_cursor() {
    let mut app = app();
    press(&mut app, "/zzzz");
    let body = render(&mut app, 100, 20).join("\n");
    assert!(body.contains("no matches"), "{body}");
    assert_eq!(app.selected(), NodeId::ROOT);
}

#[test]
fn filter_form_dims_then_hides() {
    let mut app = app();
    press(&mut app, "f");
    app.on_key(key(KeyCode::Tab)); // extensions
    press(&mut app, "jpg");
    app.on_key(key(KeyCode::Enter));
    assert_eq!(app.overlay, Overlay::None);
    let notes = app.tree.find(Path::new("notes.txt")).unwrap();
    let photos = app.tree.find(Path::new("photos")).unwrap();
    assert!(!app.matches_filter(notes));
    assert!(app.matches_filter(photos));
    assert!(
        app.rows.iter().any(|r| r.id == notes),
        "dim mode keeps rows"
    );
    let body = render(&mut app, 120, 20).join("\n");
    assert!(body.contains("[filtered]"), "{body}");

    press(&mut app, "f");
    app.on_key(key(KeyCode::BackTab)); // mode toggle
    press(&mut app, " ");
    app.on_key(key(KeyCode::Enter));
    assert!(app.filter.hide);
    assert!(
        !app.rows.iter().any(|r| r.id == notes),
        "hide mode drops rows"
    );
    assert!(app.rows.iter().any(|r| r.id == photos));
}

#[test]
fn filter_by_size_and_age_and_clear() {
    let mut app = app();
    press(&mut app, "f");
    press(&mut app, "1k");
    app.on_key(key(KeyCode::Enter));
    let cat = app.tree.find(Path::new("photos/cat.png")).unwrap();
    let beach = app.tree.find(Path::new("photos/2023/beach.jpg")).unwrap();
    assert!(!app.matches_filter(cat));
    assert!(app.matches_filter(beach));

    press(&mut app, "f");
    app.on_key(key(KeyCode::Delete));
    app.on_key(key(KeyCode::Enter));
    assert!(app.filter_match.is_none());
    assert!(app.matches_filter(cat));
}

#[test]
fn invalid_filter_value_keeps_form_open() {
    let mut app = app();
    press(&mut app, "f");
    press(&mut app, "lots");
    app.on_key(key(KeyCode::Enter));
    assert!(matches!(&app.overlay, Overlay::Filters(f) if f.error));
    let body = render(&mut app, 100, 24).join("\n");
    assert!(body.contains("Invalid value"), "{body}");
}

#[test]
fn search_skips_hidden_nodes() {
    let mut app = app();
    press(&mut app, "f");
    app.on_key(key(KeyCode::Tab));
    press(&mut app, "pdf");
    app.on_key(key(KeyCode::BackTab));
    app.on_key(key(KeyCode::BackTab));
    press(&mut app, " ");
    app.on_key(key(KeyCode::Enter));
    press(&mut app, "/beach");
    assert!(app.search.as_ref().unwrap().hits.is_empty());
}
