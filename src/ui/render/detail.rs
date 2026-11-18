//! Side panel with the details of the selected node.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use unicode_width::UnicodeWidthStr;

use super::tree::fit;
use crate::tree::{Kind, flags};
use crate::ui::app::App;
use crate::ui::format;

/// Draws the detail panel.
pub fn draw(f: &mut Frame, area: Rect, app: &mut App) {
    let block = Block::bordered()
        .border_set(app.icons.border())
        .border_style(app.theme.fg(app.theme.border))
        .style(app.theme.text())
        .title(Span::styled(
            format!(" {} ", app.s.details),
            app.theme.fg(app.theme.accent).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);
    let lines = lines(app, usize::from(inner.width));
    f.render_widget(Paragraph::new(lines), inner);
}

/// Text of the panel for the selected node.
pub fn lines(app: &mut App, width: usize) -> Vec<Line<'static>> {
    let id = app.selected();
    let s = app.s;
    let t = app.theme.clone();
    let node = app.tree.node(id).clone();
    let labels = [
        s.path,
        s.kind,
        s.on_disk,
        s.apparent,
        s.files,
        s.subdirs,
        s.modified,
        s.permissions,
    ];
    let lw = labels.iter().map(|l| l.width()).max().unwrap_or(0) + 1;
    let vw = width.saturating_sub(lw + 1).max(1);
    let label = |l: &str| Span::styled(format!("{l:<lw$}"), t.dimmed());
    let mut out = Vec::new();

    // Path, wrapped over as many lines as needed.
    let path = app.tree.path(id).display().to_string();
    for (i, chunk) in wrap(&path, vw).into_iter().enumerate() {
        let head = if i == 0 {
            label(s.path)
        } else {
            Span::raw(" ".repeat(lw))
        };
        out.push(Line::from(vec![head, Span::styled(chunk, t.text())]));
    }
    let kind = match node.kind {
        Kind::File => s.kind_file,
        Kind::Dir => s.kind_dir,
        Kind::Symlink => s.kind_symlink,
        Kind::Other => s.kind_other,
    };
    let kv = |l: &str, v: String, style: Style| Line::from(vec![label(l), Span::styled(v, style)]);
    out.push(kv(s.kind, kind.to_owned(), t.text()));
    out.push(kv(s.on_disk, format::size(node.disk), t.fg(t.value)));
    out.push(kv(s.apparent, format::size(node.apparent), t.fg(t.value)));
    if node.kind == Kind::Dir {
        out.push(kv(s.files, node.files.to_string(), t.text()));
        out.push(kv(s.subdirs, node.dirs.to_string(), t.text()));
    }
    if let Some(diff) = &app.diff {
        use crate::export::diff::Change;
        let (text, color) = match diff.change(&app.tree, id, app.apparent) {
            Change::New => (s.diff_new.to_owned(), t.new),
            Change::Grew(d) => (format!("{} +{}", s.diff_grew, format::size(d)), t.grew),
            Change::Shrank(d) => (format!("{} -{}", s.diff_shrank, format::size(d)), t.shrank),
            Change::Same => ("=".to_owned(), t.dim),
        };
        let was = diff
            .before(id, app.apparent)
            .map(|b| format!(" ({})", format::size(b)))
            .unwrap_or_default();
        out.push(Line::from(vec![
            Span::styled(format!("{:<lw$}", "Δ"), t.dimmed()),
            Span::styled(format!("{text}{was}"), t.fg(color)),
        ]));
    }
    out.push(kv(s.modified, format::date(node.mtime), t.text()));
    out.push(kv(s.permissions, format::mode(node.mode), t.text()));
    if let Some(owner) = crate::ui::owner::of_path(&app.tree.path(id)) {
        out.push(kv(s.owner, owner, t.text()));
    }
    let note = |text: String| -> Vec<Line<'static>> {
        wrap_words(&text, width.saturating_sub(2))
            .into_iter()
            .enumerate()
            .map(|(i, l)| {
                let head = if i == 0 { "› " } else { "  " };
                Line::from(Span::styled(format!("{head}{l}"), t.fg(t.warn)))
            })
            .collect()
    };
    if node.has(flags::DENIED) {
        out.extend(note(s.flag_denied.to_owned()));
        let path = app.tree.path(id).display().to_string();
        out.extend(note(crate::i18n::fill(s.denied_hint, &[("path", &path)])));
    } else if node.has(flags::ERROR) {
        out.extend(note(s.flag_error.to_owned()));
    }
    for (flag, text, color) in [
        (flags::HARDLINK, s.flag_hardlink, t.warn),
        (flags::SKIP_PSEUDO, s.flag_pseudo, t.warn),
        (flags::SKIP_FS, s.flag_other_fs, t.warn),
    ] {
        if node.has(flag) {
            out.push(Line::from(Span::styled(format!("⚠ {text}"), t.fg(color))));
        }
    }
    let inner = node.errors - u32::from(node.has(flags::ERROR));
    if node.kind == Kind::Dir && inner > 0 {
        out.extend(note(crate::i18n::fill(
            s.inner_unreadable,
            &[("n", &inner.to_string())],
        )));
    }
    if node.kind != Kind::Dir {
        return out;
    }

    let apparent = app.apparent;
    let total = node.size(apparent);
    let stats = app.selected_stats().clone();
    // Largest files.
    out.push(Line::default());
    out.push(section(s.top_files, &t));
    for fid in &stats.top {
        let size = format!("{:>9} ", format::size(app.tree.node(*fid).size(apparent)));
        let rel = app.tree.path(*fid);
        let rel = rel
            .strip_prefix(app.tree.path(id))
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let name = fit(&rel, width.saturating_sub(size.width()));
        out.push(Line::from(vec![
            Span::styled(size, t.fg(t.value)),
            Span::styled(name, t.text()),
        ]));
    }
    // Breakdown by type.
    out.push(Line::default());
    out.push(section(s.by_type, &t));
    let name_w = 12;
    let bar_w = width.saturating_sub(name_w + 10 + 8).clamp(0, 12);
    for cat in stats.ranked() {
        let c = stats.cats[usize::from(cat)];
        let frac = format::fraction(c.size, total);
        let label = fit(s.category(app.classifier.name(cat)), name_w - 1);
        let mut spans = vec![Span::styled(format!("{label:<name_w$}"), t.text())];
        if bar_w > 0 {
            spans.push(Span::styled(
                app.icons.bar(frac, bar_w),
                t.fg(t.size_color(frac)),
            ));
        }
        spans.push(Span::styled(
            format!("{:>10}", format::size(c.size)),
            t.fg(t.value),
        ));
        spans.push(Span::styled(format!("{:>7}", c.count), t.dimmed()));
        out.push(Line::from(spans));
    }
    out
}

fn section(title: &str, t: &crate::ui::theme::Theme) -> Line<'static> {
    Line::from(Span::styled(
        title.to_owned(),
        t.fg(t.accent).add_modifier(Modifier::BOLD),
    ))
}

/// Word-wraps `text` to lines of at most `width` characters.
fn wrap_words(text: &str, width: usize) -> Vec<String> {
    let width = width.max(8);
    let mut lines = Vec::new();
    let mut cur = String::new();
    let pieces = text.split_whitespace().flat_map(|w| {
        // Words longer than a line (paths) are cut into line-sized chunks.
        let chars: Vec<char> = w.chars().collect();
        chars
            .chunks(width)
            .map(|c| c.iter().collect::<String>())
            .collect::<Vec<_>>()
    });
    for word in pieces {
        let word = word.as_str();
        let needed = cur.chars().count() + usize::from(!cur.is_empty()) + word.chars().count();
        if needed > width && !cur.is_empty() {
            lines.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

/// Splits `text` in chunks of at most `width` characters.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    chars
        .chunks(width.max(1))
        .map(|c| c.iter().collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::app::test_util::*;

    #[test]
    fn wrap_words_breaks_long_paths() {
        let lines = wrap_words("run: sudo ramas /very/long/path/that/does/not/fit", 12);
        assert!(lines.iter().all(|l| l.chars().count() <= 12), "{lines:?}");
        assert_eq!(
            lines.concat().replace(' ', ""),
            "run:sudoramas/very/long/path/that/does/not/fit"
        );
    }
    use crate::ui::render::test_util::render;

    fn text(app: &mut App) -> Vec<String> {
        lines(app, 50)
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
            .collect()
    }

    #[test]
    fn detail_matches_selected_directory() {
        let mut app = app();
        press(&mut app, "j"); // a
        let lines = text(&mut app);
        assert!(lines[0].contains("/r/a"), "{lines:?}");
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("Type") && l.contains("directory"))
        );
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("On disk") && l.contains("400 B"))
        );
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("Files") && l.ends_with('2'))
        );
        let top = lines.iter().position(|l| l == "Largest files").unwrap();
        assert!(lines[top + 1].contains("y"), "{lines:?}");
        assert!(lines[top + 2].contains("x"), "{lines:?}");
        assert!(lines.iter().any(|l| l == "By type"));
    }

    #[test]
    fn detail_follows_cursor_to_a_file() {
        let mut app = app();
        press(&mut app, "jj"); // b
        let lines = text(&mut app);
        assert!(lines[0].contains("/r/b"));
        assert!(lines.iter().any(|l| l.contains("file")));
        assert!(!lines.iter().any(|l| l.starts_with("Files")));
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("Mode") && l.contains("-rw-r--r--"))
        );
    }

    #[test]
    fn panel_renders_and_toggles() {
        let mut app = app();
        press(&mut app, "j");
        let body = render(&mut app, 120, 30).join("\n");
        assert!(body.contains("Details"), "{body}");
        assert!(body.contains("/r/a"));
        press(&mut app, "p");
        let body = render(&mut app, 120, 30).join("\n");
        assert!(!body.contains("Details"));
        press(&mut app, "p");
        let body = render(&mut app, 60, 30).join("\n");
        assert!(!body.contains("Details"), "hidden when narrow");
    }

    #[test]
    fn status_bar_shows_totals_marks_and_badges() {
        let mut app = app();
        app.read_only = true;
        let lines = render(&mut app, 120, 20);
        let status = &lines[18];
        assert!(status.contains("Total 450 B"), "{status}");
        assert!(status.contains("0 marked — 0 B to free"), "{status}");
        assert!(status.contains("[READ-ONLY]"), "{status}");
    }
}
