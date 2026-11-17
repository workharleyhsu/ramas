//! Drawing of the whole screen from an [`App`].

pub mod detail;
pub mod dialogs;
pub mod filters;
pub mod progress;
pub mod status;
pub mod tree;
pub mod types;
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
    let [main, status_area, hint] = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(f.area());
    if panel_visible(app, main.width) {
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
                .areas(main);
        tree::draw(f, left, app);
        detail::draw(f, right, app);
    } else {
        tree::draw(f, main, app);
    }
    status::draw(f, status_area, app);
    draw_hint(f, hint, app);
    match &app.overlay.clone() {
        Overlay::None => {}
        Overlay::Help => widgets::help(f, app),
        Overlay::Confirm { focus } => dialogs::confirm(f, app, *focus),
        Overlay::TypeConfirm { input } => dialogs::type_confirm(f, app, input),
        Overlay::Summary(sum) => dialogs::summary(f, app, sum),
        Overlay::Search { .. } => {}
        Overlay::Filters(form) => filters::draw(f, app, form),
        Overlay::Types(view) => {
            let view = view.clone();
            types::draw(f, app, &view);
        }
    }
    if let Some(job) = &app.job {
        let settings_title = app.s.rescanning;
        progress::draw_job(f, f.area(), job, settings_title, app.s, &app.theme);
    }
    if app.delete_job.is_some() {
        dialogs::deleting(f, app);
    }
}

/// Whether the side panel fits and is enabled.
pub fn panel_visible(app: &App, width: u16) -> bool {
    app.show_panel && width >= PANEL_MIN_WIDTH
}

fn draw_hint(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let line = match &app.overlay {
        Overlay::Search { input, .. } => {
            let count = match &app.search {
                Some(s) if !s.hits.is_empty() => crate::i18n::fill(
                    app.s.search_count,
                    &[
                        ("i", &(s.idx + 1).to_string()),
                        ("n", &s.hits.len().to_string()),
                    ],
                ),
                _ if input.is_empty() => String::new(),
                _ => app.s.search_none.to_owned(),
            };
            Line::from(vec![
                Span::styled(format!(" {}", app.s.search_prompt), t.fg(t.accent)),
                Span::styled(input.clone(), t.text()),
                Span::styled("█", t.fg(t.accent)),
                Span::styled(format!("  {count}  "), t.fg(t.matched)),
                Span::styled(app.s.hint_search, t.dimmed()),
            ])
        }
        overlay => {
            let text = match overlay {
                _ if app.message.is_some() => app.message.as_deref().unwrap_or_default(),
                Overlay::Confirm { .. } => app.s.hint_confirm,
                Overlay::Filters(_) => app.s.filters_hint,
                Overlay::Summary(_) => app.s.summary_close,
                Overlay::Help => app.s.help_close,
                Overlay::Types(v) if v.list.is_some() => app.s.category_files_hint,
                Overlay::Types(_) => app.s.types_hint,
                _ => app.s.hint_browse,
            };
            let style = if app.message.is_some() {
                t.fg(t.warn)
            } else {
                t.dimmed()
            };
            Line::from(Span::styled(format!(" {text}"), style))
        }
    };
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
