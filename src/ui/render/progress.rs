//! Scan progress screen (initial scan and rescans).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::widgets::{centered, popup};
use crate::i18n::Strings;
use crate::ui::app::Settings;
use crate::ui::format;
use crate::ui::icons::Icons;
use crate::ui::job::ScanJob;
use crate::ui::theme::Theme;

/// Draws the progress box centered in `area`.
pub fn draw(f: &mut Frame, area: Rect, job: &ScanJob, title: &str, settings: &Settings) {
    draw_job(
        f,
        area,
        job,
        title,
        settings.strings,
        &settings.theme,
        settings.icons,
    );
}

/// Same as [`draw`] with explicit strings and theme.
pub fn draw_job(
    f: &mut Frame,
    area: Rect,
    job: &ScanJob,
    title: &str,
    s: &Strings,
    t: &Theme,
    icons: Icons,
) {
    let snap = job.snapshot();
    let elapsed = job.elapsed();
    let w = area.width.saturating_sub(4).clamp(30, 76);
    let rect = centered(area, w, 9);
    let frames = icons.spinner();
    let spin = frames[(elapsed.as_millis() / 100) as usize % frames.len()];
    let title = format!("{spin} {title} {}", job.root.display());
    let inner = popup(f, rect, &title, t, icons);
    let label_w = [s.entries, s.size, s.errors, s.elapsed, s.current]
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);
    let row = |label: &str, value: String| {
        Line::from(vec![
            Span::styled(format!(" {label:<label_w$}  "), t.dimmed()),
            Span::styled(value, t.text()),
        ])
    };
    let avail = usize::from(inner.width).saturating_sub(label_w + 4);
    let current = shorten(&snap.current.display().to_string(), avail);
    let lines = vec![
        row(s.entries, snap.entries.to_string()),
        row(s.size, format::size(snap.bytes)),
        row(s.errors, snap.errors.to_string()),
        row(s.elapsed, format!("{:.1}s", elapsed.as_secs_f64())),
        row(s.current, current),
        Line::default(),
        Line::from(Span::styled(format!(" {}", s.cancel_hint), t.dimmed())),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}

/// Keeps the tail of a long path: `…/deep/dir`.
pub fn shorten(text: &str, max: usize) -> String {
    let n = text.chars().count();
    if n <= max || max == 0 {
        return text.to_owned();
    }
    let tail: String = text.chars().skip(n - max + 1).collect();
    format!("…{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shorten_keeps_tail() {
        assert_eq!(shorten("/a/b/c/d", 5), "…/c/d");
        assert_eq!(shorten("/a", 5), "/a");
    }
}
