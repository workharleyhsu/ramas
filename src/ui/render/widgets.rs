//! Small shared widgets: centered popups and the help overlay.

use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, Paragraph};

use crate::ui::app::App;
use crate::ui::theme::Theme;

/// Rect of `w`x`h` centered in `area`, shrunk to fit.
pub fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let [row] = Layout::vertical([Constraint::Length(h.min(area.height))])
        .flex(Flex::Center)
        .areas(area);
    let [rect] = Layout::horizontal([Constraint::Length(w.min(area.width))])
        .flex(Flex::Center)
        .areas(row);
    rect
}

/// Clears `area` and draws a titled rounded border; returns the inner area.
pub fn popup(f: &mut Frame, area: Rect, title: &str, theme: &Theme) -> Rect {
    f.render_widget(Clear, area);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme.fg(theme.accent))
        .style(theme.text())
        .title(Span::styled(
            format!(" {title} "),
            theme.fg(theme.accent).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);
    inner
}

/// Key help overlay.
pub fn help(f: &mut Frame, app: &App) {
    let lines = app.s.help_lines;
    let key_w = lines
        .iter()
        .map(|(k, _)| k.chars().count())
        .max()
        .unwrap_or(0);
    let desc_w = lines
        .iter()
        .map(|(_, d)| d.chars().count())
        .max()
        .unwrap_or(0);
    let w = (key_w + desc_w + 6) as u16;
    let h = lines.len() as u16 + 4;
    let area = centered(f.area(), w, h);
    let inner = popup(f, area, app.s.help_title, &app.theme);
    let t = &app.theme;
    let mut text: Vec<Line> = lines
        .iter()
        .map(|(k, d)| {
            Line::from(vec![
                Span::styled(format!(" {k:<key_w$}  "), t.fg(t.accent)),
                Span::styled(*d, t.text()),
            ])
        })
        .collect();
    text.push(Line::default());
    text.push(Line::from(Span::styled(
        format!(" {}", app.s.help_close),
        t.dimmed(),
    )));
    f.render_widget(Paragraph::new(text), inner);
}
