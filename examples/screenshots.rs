//! Renders text screenshots of the TUI for the README from a synthetic tree.
//!
//! `cargo run --example screenshots -- [en|es]`

use std::ffi::OsStr;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ramas::classify::Classifier;
use ramas::cli::IconMode;
use ramas::scan::ScanOptions;
use ramas::tree::{Kind, Meta, NodeId, Tree};
use ramas::ui::app::{App, Settings};
use ramas::ui::icons::Icons;
use ramas::ui::render;
use ramas::ui::theme::default_theme;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

const MIB: u64 = 1 << 20;

fn meta(kind: Kind, size: u64) -> Meta {
    Meta {
        kind,
        disk: size,
        apparent: size,
        mtime: 1_760_000_000,
        mode: if kind == Kind::Dir {
            0o040755
        } else {
            0o100644
        },
        dev: 1,
        ino: 1,
        flags: 0,
    }
}

fn tree() -> Tree {
    let mut t = Tree::new("/home/ana", meta(Kind::Dir, 4096));
    let add = |t: &mut Tree, parent: NodeId, name: &str, size: Option<u64>| {
        let m = match size {
            Some(s) => meta(Kind::File, s),
            None => meta(Kind::Dir, 4096),
        };
        t.add_child(parent, OsStr::new(name), &m)
    };
    let videos = add(&mut t, NodeId::ROOT, "Videos", None);
    add(&mut t, videos, "trip-2025.mkv", Some(3400 * MIB));
    add(&mut t, videos, "talk.mp4", Some(820 * MIB));
    let cache = add(&mut t, NodeId::ROOT, ".cache", None);
    let cargo = add(&mut t, cache, "cargo-build", None);
    add(&mut t, cargo, "deps.tmp", Some(900 * MIB));
    add(&mut t, cache, "thumbnails.cache", Some(240 * MIB));
    let photos = add(&mut t, NodeId::ROOT, "Pictures", None);
    add(&mut t, photos, "IMG_2041.jpg", Some(8 * MIB));
    add(&mut t, photos, "IMG_2042.jpg", Some(7 * MIB));
    add(&mut t, photos, "scan.png", Some(30 * MIB));
    let music = add(&mut t, NodeId::ROOT, "Music", None);
    add(&mut t, music, "album.flac", Some(410 * MIB));
    let code = add(&mut t, NodeId::ROOT, "code", None);
    add(&mut t, code, "main.rs", Some(MIB / 8));
    add(&mut t, code, "backup.tar.gz", Some(150 * MIB));
    add(&mut t, NodeId::ROOT, "notes.md", Some(MIB / 50));
    t.aggregate();
    t
}

fn shot(app: &mut App, w: u16, h: u16) -> String {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("terminal");
    term.draw(|f| render::draw(f, app)).expect("draw");
    let buf = term.backend().buffer().clone();
    (0..h)
        .map(|y| {
            let line: String = (0..w).map(|x| buf[(x, y)].symbol().to_string()).collect();
            line.trim_end().to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn keys(app: &mut App, keys: &[KeyCode]) {
    for &k in keys {
        app.on_key(KeyEvent::new(k, KeyModifiers::NONE));
    }
}

fn main() {
    let lang = std::env::args().nth(1).unwrap_or_else(|| "en".into());
    let strings = if lang == "es" {
        &ramas::i18n::ES
    } else {
        &ramas::i18n::EN
    };
    let settings = Settings {
        apparent: false,
        read_only: false,
        imported: false,
        strings,
        theme: default_theme(),
        themes: vec![default_theme()],
        icons: Icons {
            mode: IconMode::Unicode,
        },
        scan_opts: ScanOptions::default(),
        classifier: Classifier::default(),
        diff: None,
        scanned_at: 0,
    };
    use KeyCode::{Char, Down, Right};
    let mut app = App::new(tree(), settings.clone());
    keys(&mut app, &[Down, Right, Down, Down, Down, Right, Down]);
    println!("--- tree + detail ---\n{}\n", shot(&mut app, 110, 22));

    let mut app = App::new(tree(), settings.clone());
    keys(
        &mut app,
        &[
            Down,
            Right,
            Down,
            Char(' '),
            Down,
            Down,
            Char(' '),
            Char('d'),
        ],
    );
    println!("--- delete dialog ---\n{}\n", shot(&mut app, 110, 18));

    let mut app = App::new(tree(), settings);
    keys(&mut app, &[Char('t')]);
    println!("--- type analysis ---\n{}", shot(&mut app, 110, 14));
}
