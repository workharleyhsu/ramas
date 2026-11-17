//! Bottom status bar.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::ui::app::App;
use crate::ui::format;

/// Draws totals, free space, marks, errors and mode badges.
pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let s = app.s;
    let root = app.tree.root();
    let (n, _) = app.marked_summary();
    let mut left = format!(" {} {}", s.total, format::size(root.size(app.apparent)));
    if let Some(free) = app.free {
        left.push_str(&format!(" · {} {}", s.free, format::size(free)));
    }
    left.push_str(" · ");
    let marked = app.marked_text();
    let mut spans = vec![Span::raw(left)];
    let mark_style = if n > 0 {
        t.status().fg(t.marked).add_modifier(Modifier::BOLD)
    } else {
        t.status()
    };
    spans.push(Span::styled(marked, mark_style));
    if root.errors > 0 {
        spans.push(Span::raw(" · "));
        spans.push(Span::styled(
            format!("{} {}", s.errors, root.errors),
            if t.color {
                t.status().fg(t.error)
            } else {
                t.status()
            },
        ));
    }
    if let Some(diff) = &app.diff {
        let name = diff.against.file_name().map_or_else(
            || diff.against.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        spans.push(Span::raw(" · "));
        spans.push(Span::styled(
            crate::i18n::fill(s.diff_against, &[("path", &name)]),
            t.status().add_modifier(Modifier::BOLD),
        ));
    }
    let mut badges = Vec::new();
    if app.read_only {
        badges.push(s.read_only);
    }
    if app.imported {
        badges.push(s.imported);
    }
    if app.filter_match.is_some() {
        badges.push(s.filters_active);
    }
    let right = badges
        .iter()
        .map(|b| format!("[{b}]"))
        .collect::<Vec<_>>()
        .join(" ");
    let used: usize = spans.iter().map(|sp| sp.width()).sum();
    let pad = usize::from(area.width).saturating_sub(used + right.chars().count() + 1);
    spans.push(Span::raw(" ".repeat(pad)));
    spans.push(Span::styled(right, t.status().add_modifier(Modifier::BOLD)));
    f.render_widget(Paragraph::new(Line::from(spans)).style(t.status()), area);
}
