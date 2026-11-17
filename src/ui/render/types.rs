//! Type analysis popup.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::tree::{bar, fit};
use super::widgets::popup;
use crate::i18n::fill;
use crate::ui::app::{App, CategoryList, TypesView};
use crate::ui::format;

/// Draws the table or the category file list over most of the screen.
pub fn draw(f: &mut Frame, app: &mut App, view: &TypesView) {
    let full = f.area();
    let area = Rect {
        x: full.x + 2,
        y: full.y + 1,
        width: full.width.saturating_sub(4),
        height: full.height.saturating_sub(3),
    };
    let path = app.tree.path(view.node).display().to_string();
    let title = match &view.list {
        None => fill(app.s.types_title, &[("path", &path)]),
        Some(l) => format!(
            "{} — {}",
            fill(app.s.types_title, &[("path", &path)]),
            app.s.category(app.classifier.name(l.cat))
        ),
    };
    let inner = popup(f, area, &title, &app.theme.clone());
    let lines = match &view.list {
        None => table(app, view, inner),
        Some(list) => files(app, view, list, inner),
    };
    f.render_widget(Paragraph::new(lines), inner);
}

fn table(app: &mut App, view: &TypesView, area: Rect) -> Vec<Line<'static>> {
    let rows = app.type_rows();
    let stats = app.selected_stats().clone();
    let t = app.theme.clone();
    let s = app.s;
    let total: u64 = stats.cats.iter().map(|c| c.size).sum();
    let width = usize::from(area.width);
    let bar_w = width.saturating_sub(16 + 11 + 11 + 8 + 1).clamp(0, 40);
    let mut out = vec![Line::from(Span::styled(
        format!(" {:<15}{:>11}{:>9}  ", s.category, s.size, s.count),
        t.dimmed().add_modifier(Modifier::BOLD),
    ))];
    for (i, &cat) in rows.iter().enumerate() {
        let c = stats.cats[usize::from(cat)];
        let frac = format::fraction(c.size, total);
        let label = fit(s.category(app.classifier.name(cat)), 14);
        let mut line = Line::from(vec![
            Span::styled(format!(" {label:<15}"), t.text()),
            Span::styled(format!("{:>11}", format::size(c.size)), t.fg(t.size)),
            Span::styled(format!("{:>9}  ", c.count), t.dimmed()),
            Span::styled(bar(frac, bar_w), t.fg(t.gradient(frac))),
            Span::styled(format!(" {}", format::percent(frac)), t.dimmed()),
        ]);
        if i == view.cursor {
            line = line.patch_style(t.selected());
        }
        out.push(line);
    }
    out
}

fn files(app: &App, view: &TypesView, list: &CategoryList, area: Rect) -> Vec<Line<'static>> {
    let t = &app.theme;
    let height = usize::from(area.height).max(1);
    let width = usize::from(area.width);
    let offset = list.cursor.saturating_sub(height - 1);
    let base = app.tree.path(view.node);
    list.files
        .iter()
        .enumerate()
        .skip(offset)
        .take(height)
        .filter(|(_, id)| !app.tree.is_removed(**id))
        .map(|(i, &id)| {
            let covered = app.marks.covers(&app.tree, id);
            let mark = if covered { app.icons.mark() } else { " " };
            let size = format!("{:>10}  ", format::size(app.size_of(id)));
            let path = app.tree.path(id);
            let rel = path
                .strip_prefix(&base)
                .unwrap_or(&path)
                .display()
                .to_string();
            let rel = fit(&rel, width.saturating_sub(size.len() + 3));
            let name_style = if covered {
                t.fg(t.marked).add_modifier(Modifier::BOLD)
            } else {
                t.text()
            };
            let mut line = Line::from(vec![
                Span::styled(format!(" {mark}"), t.fg(t.marked)),
                Span::styled(size, t.fg(t.size)),
                Span::styled(rel, name_style),
            ]);
            if i == list.cursor {
                line = line.patch_style(t.selected());
            }
            line
        })
        .collect()
}
