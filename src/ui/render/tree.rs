//! Tree pane: connectors, icons, names, sizes, percentages and bars.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::export::diff::Change;
use crate::tree::{Kind, NodeId, flags};
use crate::ui::app::App;
use crate::ui::format;
use crate::ui::rows::Row;

const SIZE_W: usize = 10;
const PCT_W: usize = 7;
const DELTA_W: usize = 11;

/// Draws the tree pane in `area` and records its inner area in the app.
pub fn draw(f: &mut Frame, area: Rect, app: &mut App) {
    let t = &app.theme;
    let title = format!(" {} ", app.tree.root_path().display());
    let metric = if app.apparent {
        app.s.apparent_size
    } else {
        app.s.disk_usage
    };
    let right = format!(" {}: {} · {} ", app.s.sort, app.sort_label(), metric);
    let block = Block::bordered()
        .border_set(app.icons.border())
        .border_style(t.fg(t.border))
        .style(t.text())
        .title(Span::styled(
            title,
            t.fg(t.accent).add_modifier(Modifier::BOLD),
        ))
        .title(Line::from(Span::styled(right, t.dimmed())).right_aligned());
    let inner = block.inner(area);
    f.render_widget(block, area);
    app.areas.tree = inner;
    app.fix_offset(usize::from(inner.height));

    let width = usize::from(inner.width);
    let end = (app.offset + usize::from(inner.height)).min(app.rows.len());
    let lines: Vec<Line> = (app.offset..end)
        .map(|i| row_line(app, app.rows[i], width, i == app.cursor))
        .collect();
    f.render_widget(Paragraph::new(lines), inner);
}

/// Bar width for a pane of `width` columns.
fn bar_width(width: usize) -> usize {
    if width < 50 {
        0
    } else {
        (width / 6).clamp(6, 20)
    }
}

fn row_line(app: &App, row: Row, width: usize, selected: bool) -> Line<'static> {
    let t = &app.theme;
    let id = row.id;
    let node = app.tree.node(id);
    let size = app.size_of(id);
    let parent_size = app.tree.parent(id).map_or(size, |p| app.size_of(p));
    let frac = format::fraction(size, parent_size);
    let dim = !app.matches_filter(id);

    let bar_w = bar_width(width);
    let pct_w = if width < 36 { 0 } else { PCT_W };
    let delta_w = if app.diff.is_some() { DELTA_W } else { 0 };
    let right_w = delta_w + SIZE_W + pct_w + if bar_w > 0 { bar_w + 1 } else { 0 };

    let mut spans: Vec<Span<'static>> = Vec::new();
    let prefix = connectors(app, row, width / 2);
    let prefix_w = prefix.width();
    spans.push(Span::styled(prefix, t.fg(t.connector)));

    let marked = app.marks.is_marked(id);
    let covered = marked || app.marks.covers(&app.tree, id);
    let icon = if marked {
        app.icons.mark()
    } else {
        app.icons
            .node(node.kind, app.is_open(id), node.has(flags::ERROR))
    };
    let icon_style = if covered {
        t.fg(t.marked).add_modifier(Modifier::BOLD)
    } else if node.has(flags::ERROR) {
        t.fg(t.error)
    } else if node.kind == Kind::Dir {
        t.fg(t.dir)
    } else {
        t.dimmed()
    };
    let lead = format!("{icon} ");
    let lead_w = lead.width();
    spans.push(Span::styled(lead, icon_style));

    let name_w = width.saturating_sub(prefix_w + lead_w + right_w + 1);
    let name = fit(&display_name(app, id), name_w);
    let name_pad = name_w.saturating_sub(name.width());
    let name_style = match node.kind {
        _ if covered => t.fg(t.marked).add_modifier(Modifier::BOLD),
        Kind::Dir => t.fg(t.dir).add_modifier(Modifier::BOLD),
        Kind::Symlink => t.fg(t.accent),
        _ => t.text(),
    };
    if app.is_hit(id) {
        spans.extend(highlight(app, id, &name, name_style));
    } else {
        spans.push(Span::styled(name, name_style));
    }
    spans.push(Span::raw(" ".repeat(name_pad + 1)));
    if let Some(diff) = &app.diff {
        let (text, color) = match diff.change(&app.tree, id, app.apparent) {
            Change::New => (app.s.diff_new.to_owned(), t.new),
            Change::Grew(d) => (format!("+{}", format::size(d)), t.grew),
            Change::Shrank(d) => (format!("-{}", format::size(d)), t.shrank),
            Change::Same => (String::new(), t.dim),
        };
        spans.push(Span::styled(
            format!("{:>w$}", fit(&text, DELTA_W - 1), w = DELTA_W),
            t.fg(color),
        ));
    }

    spans.push(Span::styled(
        format!("{:>w$}", format::size(size), w = SIZE_W),
        t.fg(t.size),
    ));
    if pct_w > 0 {
        spans.push(Span::styled(
            format!(" {}", format::percent(frac)),
            t.dimmed(),
        ));
    }
    if bar_w > 0 {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            app.icons.bar(frac, bar_w),
            t.fg(t.gradient(frac)),
        ));
    }

    let mut line = Line::from(spans);
    if dim {
        line = line.patch_style(t.dimmed());
    }
    if selected {
        line = line.patch_style(t.selected());
        pad_to(&mut line, width, t.selected());
    }
    line
}

/// Name spans with the fuzzy-matched characters emphasized.
fn highlight(app: &App, id: NodeId, shown: &str, base: Style) -> Vec<Span<'static>> {
    let Some(search) = &app.search else {
        return vec![Span::styled(shown.to_owned(), base)];
    };
    let raw = app.tree.name(id).to_string_lossy();
    let idx = crate::search::Finder::new(&search.query).indices(&raw);
    let hl = base
        .patch(app.theme.fg(app.theme.matched))
        .add_modifier(Modifier::UNDERLINED | Modifier::BOLD);
    let mut out: Vec<Span<'static>> = Vec::new();
    for (i, c) in shown.chars().enumerate() {
        let style = if idx.binary_search(&(i as u32)).is_ok() {
            hl
        } else {
            base
        };
        match out.last_mut() {
            Some(last) if last.style == style => last.content.to_mut().push(c),
            _ => out.push(Span::styled(c.to_string(), style)),
        }
    }
    out
}

/// Name shown for a node: the root shows its full path.
fn display_name(app: &App, id: NodeId) -> String {
    let name = app.tree.name(id).to_string_lossy();
    if app.tree.node(id).kind == Kind::Dir && id != NodeId::ROOT {
        format!("{name}/")
    } else {
        name.into_owned()
    }
}

/// Tree connector prefix for a row, at most `max` columns wide.
fn connectors(app: &App, row: Row, max: usize) -> String {
    if row.depth == 0 {
        return String::new();
    }
    let [branch, last, vert, blank] = app.icons.connectors();
    // Chain from the row's node up to depth 1.
    let mut chain = Vec::with_capacity(row.depth as usize);
    let mut cur = row.id;
    for _ in 0..row.depth {
        chain.push(cur);
        match app.tree.parent(cur) {
            Some(p) => cur = p,
            None => break,
        }
    }
    chain.reverse();
    let mut parts: Vec<&str> = chain
        .iter()
        .enumerate()
        .map(|(i, &n)| {
            let more = has_next_visible(app, n);
            match (i + 1 == chain.len(), more) {
                (true, true) => branch,
                (true, false) => last,
                (false, true) => vert,
                (false, false) => blank,
            }
        })
        .collect();
    let seg = 4;
    if parts.len() * seg > max && max > seg {
        let keep = (max - 1) / seg;
        parts.drain(..parts.len() - keep);
        return format!("…{}", parts.concat());
    }
    parts.concat()
}

fn has_next_visible(app: &App, id: NodeId) -> bool {
    std::iter::successors(app.tree.next_sibling(id), |&s| app.tree.next_sibling(s))
        .any(|s| app.is_shown(s))
}

/// Truncates `text` to `max` columns with a trailing ellipsis.
pub fn fit(text: &str, max: usize) -> String {
    if text.width() <= max {
        return text.to_owned();
    }
    if max == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut w = 0;
    for c in text.chars() {
        let cw = c.width().unwrap_or(0);
        if w + cw > max - 1 {
            break;
        }
        out.push(c);
        w += cw;
    }
    out.push('…');
    out
}

fn pad_to(line: &mut Line<'static>, width: usize, style: Style) {
    let w = line.width();
    if w < width {
        line.spans.push(Span::styled(" ".repeat(width - w), style));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::app::test_util::*;
    use crate::ui::render::test_util::render;
    use crossterm::event::KeyCode;

    #[test]
    fn fit_truncates_with_ellipsis() {
        assert_eq!(fit("abcdef", 4), "abc…");
        assert_eq!(fit("abc", 4), "abc");
    }

    #[test]
    fn renders_connectors_sizes_and_percent() {
        let mut app = app();
        press(&mut app, "jl"); // expand a
        let lines = render(&mut app, 100, 12);
        let body = lines.join("\n");
        assert!(lines[1].contains("/r"), "root row: {}", lines[1]);
        assert!(body.contains("├── ▾ a/"), "{body}");
        assert!(body.contains("│   ├── · y"), "{body}");
        assert!(body.contains("│   └── · x"), "{body}");
        assert!(body.contains("└── ▸ c/"), "{body}");
        assert!(body.contains("400 B"), "{body}");
        assert!(body.contains(" 88.9%"), "{body}");
        assert!(body.contains("sort: size · disk usage"), "{body}");
    }

    #[test]
    fn selection_moves_in_rendered_output() {
        let mut app = app();
        press(&mut app, "jj");
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 10)).unwrap();
        term.draw(|f| crate::ui::render::draw(f, &mut app)).unwrap();
        let buf = term.backend().buffer();
        // Row 0 border, row 1 root, row 2 a, row 3 b (selected).
        let sel = app.theme.selected_bg;
        assert_eq!(buf[(5, 3)].bg, sel);
        assert_ne!(buf[(5, 2)].bg, sel);
        app.on_key(key(KeyCode::Up));
        term.draw(|f| crate::ui::render::draw(f, &mut app)).unwrap();
        let buf = term.backend().buffer();
        assert_eq!(buf[(5, 2)].bg, sel);
    }

    #[test]
    fn scrolls_when_cursor_leaves_window() {
        let mut app = app();
        press(&mut app, "jl"); // 6 rows
        app.on_key(key(KeyCode::End));
        let lines = render(&mut app, 80, 7); // 3 inner rows + status + hint
        assert!(lines[3].contains("c/"), "{lines:?}");
        assert!(!lines.iter().any(|l| l.contains(" /r ") && l.contains("B ")));
    }

    #[test]
    fn narrow_terminal_drops_bar_and_percent() {
        let mut app = app();
        let lines = render(&mut app, 34, 6);
        assert!(!lines.join("").contains('%'));
    }
}
