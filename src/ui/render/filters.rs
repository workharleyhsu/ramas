//! Filter form popup.

use ratatui::Frame;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::widgets::{centered, popup};
use crate::ui::app::{App, FilterForm};

/// Draws the filter form.
pub fn draw(f: &mut Frame, app: &App, form: &FilterForm) {
    let s = app.s;
    let t = &app.theme;
    let area = centered(f.area(), 64, 12);
    let inner = popup(f, area, s.filters_title, t, app.icons);
    let labels = [s.filter_min_size, s.filter_ext, s.filter_age];
    let mut lines = Vec::new();
    for (i, label) in labels.iter().enumerate() {
        let focused = form.focus == i;
        let marker = if focused { "▶ " } else { "  " };
        lines.push(Line::from(Span::styled(
            format!("{marker}{label}"),
            t.dimmed(),
        )));
        let value_style = if focused { t.selected() } else { t.text() };
        let cursor = if focused { "█" } else { "" };
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(format!("{}{cursor}", form.fields[i]), value_style),
        ]));
    }
    let focused = form.focus == 3;
    let marker = if focused { "▶ " } else { "  " };
    let (dim, hide) = if form.hide {
        ("( )", "(•)")
    } else {
        ("(•)", "( )")
    };
    let style = if focused { t.selected() } else { t.text() };
    lines.push(Line::from(vec![
        Span::styled(format!("{marker}{}: ", s.filter_mode), t.dimmed()),
        Span::styled(
            format!("{dim} {}  {hide} {}", s.filter_dim, s.filter_hide),
            style,
        ),
    ]));
    if form.error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("  {}", s.filter_invalid),
            t.fg(t.danger).add_modifier(Modifier::BOLD),
        )));
    }
    f.render_widget(Paragraph::new(lines), inner);
}
