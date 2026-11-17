//! Delete confirmation, typed confirmation, summary and job popups.

use ratatui::Frame;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use super::progress::shorten;
use super::tree::fit;
use super::widgets::{centered, popup};
use crate::i18n::fill;
use crate::ui::app::{App, Button, DeleteSummary};
use crate::ui::format;

/// Max paths listed in the confirmation dialog.
const MAX_LISTED: usize = 12;

/// Three-button delete confirmation.
pub fn confirm(f: &mut Frame, app: &App, focus: Button) {
    let s = app.s;
    let t = &app.theme;
    let ids = app.marks.ids();
    let listed = ids.len().min(MAX_LISTED);
    let extra = usize::from(ids.len() > MAX_LISTED);
    let w = f.area().width.saturating_sub(6).clamp(40, 100);
    let h = (listed + extra + 7) as u16;
    let area = centered(f.area(), w, h);
    let inner = popup(f, area, s.delete_title, t, app.icons);
    let width = usize::from(inner.width);
    let mut lines = Vec::new();
    let mut sorted = ids.clone();
    sorted.sort_by_key(|&id| std::cmp::Reverse(app.size_of(id)));
    for &id in sorted.iter().take(MAX_LISTED) {
        let size = format!(" {:>9}  ", format::size(app.size_of(id)));
        let path = app.tree.path(id).display().to_string();
        let path = shorten(&path, width.saturating_sub(size.len() + 1));
        lines.push(Line::from(vec![
            Span::styled(size, t.fg(t.size)),
            Span::styled(path, t.text()),
        ]));
    }
    if extra > 0 {
        lines.push(Line::from(Span::styled(
            format!(" … +{}", ids.len() - MAX_LISTED),
            t.dimmed(),
        )));
    }
    lines.push(Line::default());
    let (_, total) = app.marked_summary();
    lines.push(Line::from(vec![
        Span::styled(format!(" {}: ", s.delete_total), t.text()),
        Span::styled(
            format::size(total),
            t.fg(t.marked).add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::default());
    let button = |b: Button, label: &str| {
        let style = if b == focus { t.selected() } else { t.dimmed() };
        Span::styled(format!("[ {label} ]"), style)
    };
    lines.push(Line::from(vec![
        Span::raw(" "),
        button(Button::Trash, s.btn_trash),
        Span::raw("  "),
        button(Button::Permanent, s.btn_permanent),
        Span::raw("  "),
        button(Button::Cancel, s.btn_cancel),
    ]));
    f.render_widget(Paragraph::new(lines), inner);
}

/// Second confirmation: the user types the confirmation word.
pub fn type_confirm(f: &mut Frame, app: &App, input: &str) {
    let s = app.s;
    let t = &app.theme;
    let area = centered(f.area(), 60, 7);
    let inner = popup(f, area, s.confirm_title, t, app.icons);
    let lines = vec![
        Line::from(Span::styled(
            format!(" {}", fill(s.confirm_prompt, &[("word", s.confirm_word)])),
            t.fg(t.error).add_modifier(Modifier::BOLD),
        )),
        Line::default(),
        Line::from(vec![
            Span::styled(" > ", t.fg(t.accent)),
            Span::styled(input.to_owned(), t.text()),
            Span::styled("█", t.fg(t.accent)),
        ]),
        Line::default(),
        Line::from(Span::styled(format!(" Esc: {}", s.btn_cancel), t.dimmed())),
    ];
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

/// Result of a delete run.
pub fn summary(f: &mut Frame, app: &App, sum: &DeleteSummary) {
    let s = app.s;
    let t = &app.theme;
    let shown = sum.failed.len().min(MAX_LISTED);
    let w = f.area().width.saturating_sub(6).clamp(40, 100);
    let area = centered(f.area(), w, (shown + 7) as u16);
    let inner = popup(f, area, s.summary_title, t, app.icons);
    let width = usize::from(inner.width);
    let mut lines = vec![Line::from(Span::styled(
        format!(
            " {}",
            fill(
                s.summary_freed,
                &[
                    ("size", &format::size(sum.freed)),
                    ("n", &sum.removed.to_string())
                ]
            )
        ),
        t.fg(t.shrank).add_modifier(Modifier::BOLD),
    ))];
    if !sum.failed.is_empty() {
        lines.push(Line::from(Span::styled(
            format!(
                " {}",
                fill(s.summary_failed, &[("n", &sum.failed.len().to_string())])
            ),
            t.fg(t.error),
        )));
        for (path, why) in sum.failed.iter().take(MAX_LISTED) {
            let why_w = why.chars().count() + 5;
            let path = shorten(&path.display().to_string(), width.saturating_sub(why_w));
            let text = format!("  {path} — {why}");
            lines.push(Line::from(Span::styled(fit(&text, width), t.text())));
        }
    }
    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        format!(" {}", s.summary_close),
        t.dimmed(),
    )));
    f.render_widget(Paragraph::new(lines), inner);
}

/// Small "Deleting…" popup.
pub fn deleting(f: &mut Frame, app: &App) {
    let area = centered(f.area(), 30, 3);
    let inner = popup(f, area, app.s.deleting, &app.theme, app.icons);
    f.render_widget(
        Paragraph::new(Span::styled(
            format!(" {}", app.s.deleting),
            app.theme.text(),
        )),
        inner,
    );
}
