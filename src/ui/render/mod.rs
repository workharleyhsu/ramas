//! Drawing of the whole screen from an [`App`].

pub mod progress;
pub mod tree;
mod widgets;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::app::{App, Overlay};

/// Narrowest width at which the side panel is drawn.
pub const PANEL_MIN_WIDTH: u16 = 70;

/// Draws everything.
pub fn draw(f: &mut Frame, app: &mut App) {
    let [main, hint] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).areas(f.area());
    tree::draw(f, main, app);
    draw_hint(f, hint, app);
    match app.overlay {
        Overlay::None => {}
        Overlay::Help => widgets::help(f, app),
    }
}

fn draw_hint(f: &mut Frame, area: Rect, app: &App) {
    let text = app.message.as_deref().unwrap_or(app.s.hint_browse);
    let line = Line::from(Span::styled(format!(" {text}"), app.theme.dimmed()));
    f.render_widget(Paragraph::new(line), area);
}

#[cfg(test)]
pub(crate) mod test_util {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;

    /// Renders the app into a `w`x`h` test buffer and returns its lines.
    pub fn render(app: &mut App, w: u16, h: u16) -> Vec<String> {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| draw(f, app)).unwrap();
        let buf = term.backend().buffer().clone();
        (0..h)
            .map(|y| {
                (0..w)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect()
    }
}
