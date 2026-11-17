//! Terminal user interface: state, input handling and rendering.

pub mod app;
pub mod find;
pub mod format;
pub mod icons;
pub mod job;
pub mod mouse;
pub mod ops;
pub mod owner;
pub mod render;
pub mod rows;
pub mod terminal;
pub mod theme;
pub mod types;

use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use app::{App, Settings};
use job::ScanJob;
use terminal::{Term, TerminalGuard};

use crate::scan::ScanError;
use crate::tree::Tree;

/// Scans `root` with a progress screen, then opens the browser.
pub fn run(root: &Path, settings: Settings) -> Result<()> {
    let mut guard = TerminalGuard::enter()?;
    let Some(tree) = scan_screen(&mut guard.terminal, root, &settings)? else {
        return Ok(());
    };
    browse(&mut guard.terminal, App::new(tree, settings))
}

/// Opens the browser on an already built tree (import mode).
pub fn run_with_tree(tree: Tree, settings: Settings) -> Result<()> {
    let mut guard = TerminalGuard::enter()?;
    browse(&mut guard.terminal, App::new(tree, settings))
}

/// Shows progress until the scan ends. `None` when the user cancels.
fn scan_screen(term: &mut Term, root: &Path, settings: &Settings) -> Result<Option<Tree>> {
    let job = ScanJob::start(root.to_path_buf(), settings.scan_opts.clone(), None);
    loop {
        term.draw(|f| {
            render::progress::draw(f, f.area(), &job, settings.strings.scanning, settings)
        })?;
        if let Some(res) = job.try_result() {
            return match res {
                Ok(tree) => Ok(Some(tree)),
                Err(ScanError::Cancelled) => Ok(None),
                Err(e) => Err(e.into()),
            };
        }
        if event::poll(Duration::from_millis(80))?
            && let Event::Key(k) = event::read()?
            && k.kind == KeyEventKind::Press
        {
            let ctrl_c =
                k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c');
            if ctrl_c || matches!(k.code, KeyCode::Esc | KeyCode::Char('q')) {
                job.cancel();
            }
        }
    }
}

/// Main event loop.
fn browse(term: &mut Term, mut app: App) -> Result<()> {
    while !app.quit {
        term.draw(|f| render::draw(f, &mut app))?;
        let timeout = if app.busy() {
            Duration::from_millis(80)
        } else {
            Duration::from_millis(500)
        };
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(k) if k.kind == KeyEventKind::Press => app.on_key(k),
                Event::Mouse(m) => app.on_mouse(m),
                _ => {}
            }
        }
        app.tick();
    }
    Ok(())
}
