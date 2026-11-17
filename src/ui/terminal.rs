//! Terminal setup and guaranteed restoration (drop guard + panic hook).

use std::io::{Stdout, stdout};
use std::sync::Once;

use anyhow::Result;
use crossterm::cursor::Show;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

/// Concrete terminal type.
pub type Term = Terminal<CrosstermBackend<Stdout>>;

/// Owns the terminal in raw/alternate-screen mode; restores it on drop.
pub struct TerminalGuard {
    pub terminal: Term,
}

impl TerminalGuard {
    /// Enters raw mode, the alternate screen and mouse capture.
    pub fn enter() -> Result<Self> {
        install_panic_hook();
        enable_raw_mode()?;
        let setup = execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)
            .map_err(anyhow::Error::from)
            .and_then(|()| Ok(Terminal::new(CrosstermBackend::new(stdout()))?));
        match setup {
            Ok(terminal) => Ok(TerminalGuard { terminal }),
            Err(e) => {
                restore();
                Err(e)
            }
        }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore();
    }
}

/// Puts the terminal back in cooked mode. Safe to call more than once.
pub fn restore() {
    let _ = disable_raw_mode();
    let _ = execute!(stdout(), DisableMouseCapture, LeaveAlternateScreen, Show);
}

fn install_panic_hook() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore();
            previous(info);
        }));
    });
}
