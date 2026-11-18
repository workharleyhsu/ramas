//! Synthetic 1M-entry tree: times the UI-side operations and reports memory.
//!
//! `cargo run --release --example bench_ui -- [DIRS] [FILES_PER_DIR]`

use std::ffi::OsStr;
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ramas::classify::Classifier;
use ramas::cli::IconMode;
use ramas::scan::ScanOptions;
use ramas::tree::{Kind, Meta, NodeId, Tree};
use ramas::ui::app::{App, Settings};
use ramas::ui::icons::Icons;
use ramas::ui::render;
use ramas::ui::theme::{Themes, default_theme};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn meta(kind: Kind, size: u64, i: u64) -> Meta {
    Meta {
        kind,
        disk: size,
        apparent: size,
        mtime: 1_600_000_000 + (i as i64 % 100_000_000),
        mode: if kind == Kind::Dir {
            0o040755
        } else {
            0o100644
        },
        dev: 1,
        ino: i,
        flags: 0,
    }
}

fn build(dirs: u64, files: u64) -> Tree {
    const EXT: [&str; 8] = ["mp4", "jpg", "flac", "pdf", "zip", "rs", "tmp", "dat"];
    let mut t = Tree::new("/synthetic", meta(Kind::Dir, 4096, 0));
    let mut ino = 1;
    for d in 0..dirs {
        let dir = t.add_child(
            NodeId::ROOT,
            OsStr::new(&format!("dir{d:05}")),
            &meta(Kind::Dir, 4096, ino),
        );
        ino += 1;
        for f in 0..files {
            let size = (ino * 2_654_435_761) % 10_000_000;
            let name = format!("file{f:05}_{}.{}", ino % 977, EXT[(ino % 8) as usize]);
            t.add_child(dir, OsStr::new(&name), &meta(Kind::File, size, ino));
            ino += 1;
        }
    }
    t.aggregate();
    t
}

fn peak_rss_mib() -> f64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    status
        .lines()
        .find(|l| l.starts_with("VmHWM:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|kb| kb.parse::<f64>().ok())
        .map_or(0.0, |kb| kb / 1024.0)
}

fn timed<T>(label: &str, f: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let out = f();
    println!(
        "{label:<34} {:>9.1} ms",
        start.elapsed().as_secs_f64() * 1000.0
    );
    out
}

fn key(app: &mut App, code: KeyCode) {
    app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let dirs: u64 = args.next().and_then(|a| a.parse().ok()).unwrap_or(1000);
    let files: u64 = args.next().and_then(|a| a.parse().ok()).unwrap_or(1000);
    let tree = timed("build synthetic tree", || build(dirs, files));
    println!(
        "nodes {}, arena ~{:.1} MiB",
        tree.len(),
        tree.mem_bytes() as f64 / 1048576.0
    );

    let mut buf = Vec::new();
    timed("export JSON", || ramas::export::write(&tree, 0, &mut buf))?;
    println!("export size {:.1} MiB", buf.len() as f64 / 1048576.0);
    let imported = timed("import JSON", || ramas::export::read(&buf[..]))?;
    drop(imported);
    drop(buf);

    let settings = Settings {
        apparent: false,
        read_only: true,
        imported: false,
        strings: &ramas::i18n::EN,
        theme: default_theme(),
        themes: Themes::default(),
        config_path: None,
        notice: None,
        icons: Icons {
            mode: IconMode::Unicode,
        },
        scan_opts: ScanOptions::default(),
        classifier: Classifier::default(),
        diff: None,
        scanned_at: 0,
    };
    let mut app = timed("App::new (sort + rows)", || App::new(tree, settings));
    let mut term = Terminal::new(TestBackend::new(200, 60))?;
    timed("first frame (root stats)", || {
        term.draw(|f| render::draw(f, &mut app))
    })?;
    timed("frame (cached)", || {
        term.draw(|f| render::draw(f, &mut app))
    })?;
    key(&mut app, KeyCode::Down);
    timed("expand dir (1k children)", || key(&mut app, KeyCode::Right));
    timed("frame after expand", || {
        term.draw(|f| render::draw(f, &mut app))
    })?;
    timed("cycle sort (name)", || {
        app.on_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE))
    });
    timed("search keystroke over all nodes", || {
        app.run_search("file00042_5")
    });
    timed("frame with search", || {
        term.draw(|f| render::draw(f, &mut app))
    })?;
    timed("filter evaluate (ext + size)", || {
        app.set_filter(ramas::filter::Filter {
            min_size: Some(5_000_000),
            exts: vec!["mp4".into()],
            older_than_days: None,
            hide: true,
        })
    });
    timed("frame with hide filter", || {
        term.draw(|f| render::draw(f, &mut app))
    })?;
    key(&mut app, KeyCode::End);
    timed("frame at end", || term.draw(|f| render::draw(f, &mut app)))?;
    println!("peak RSS {:.1} MiB", peak_rss_mib());
    Ok(())
}
