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
    f.render_widget(
        ratatui::widgets::Block::new().style(app.theme.text()),
        f.area(),
    );
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
        Overlay::ThemePicker { .. } => {}
    }
    // The picker sits on the right so the tree shows the previewed theme.
    let theme = app.theme.clone();
    let labels = tui_themes::Labels {
        title: app.s.theme,
        filter: app.s.theme_filter,
        placeholder: app.s.theme_placeholder,
        hint: app.s.theme_keys,
        no_match: app.s.theme_no_match,
        invalid: app.s.theme_invalid,
        ascii: app.icons.mode == crate::cli::IconMode::Ascii,
    };
    if let Overlay::ThemePicker { picker, .. } = &mut app.overlay {
        picker.render(
            tui_themes::Picker::area(main),
            f.buffer_mut(),
            &theme,
            &labels,
        );
    }
    if let Some(job) = &app.job {
        let settings_title = app.s.rescanning;
        progress::draw_job(
            f,
            f.area(),
            job,
            settings_title,
            app.s,
            &app.theme,
            app.icons,
        );
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
                Overlay::ThemePicker { .. } => app.s.theme_hint,
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

#[cfg(test)]
mod theme_tests {
    use super::test_util::render;
    use crate::ui::app::Overlay;
    use crate::ui::app::test_util::*;
    use crate::ui::theme::{Catalog, ColorDepth, default_theme};
    use crossterm::event::KeyCode;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn every_theme_renders_every_screen() {
        let catalog = Catalog::builtin();
        for theme in catalog.themes().into_iter().cloned() {
            let name = theme.id.clone();
            for keys in ["", "?", "d", "t", "f", "/a"] {
                let mut a = app();
                a.theme = theme.clone();
                press(&mut a, "jl ");
                press(&mut a, keys);
                let body = render(&mut a, 100, 30).join("\n");
                assert!(body.contains("Total"), "{name} {keys}: {body}");
            }
            // The background color of the theme reaches the cells.
            let mut app = app();
            app.theme = theme;
            let mut term = Terminal::new(TestBackend::new(80, 20)).unwrap();
            term.draw(|f| super::draw(f, &mut app)).unwrap();
            let buf = term.backend().buffer();
            assert_eq!(buf[(79, 19)].bg, app.theme.bg, "{name}");
        }
    }

    #[test]
    fn shift_t_opens_the_picker_and_previews_live() {
        let mut app = app();
        assert_eq!(app.theme.id, "ramas");
        press(&mut app, "T");
        assert!(matches!(app.overlay, Overlay::ThemePicker { .. }));
        let body = render(&mut app, 100, 30).join("\n");
        for needle in ["Theme", "* ramas", "Gruvbox Dark", "Enter save"] {
            assert!(body.contains(needle), "{needle}: {body}");
        }
        assert!(body.contains("Total"), "the tree stays visible: {body}");
        // Moving previews at once; Esc brings the old theme back.
        app.on_key(key(KeyCode::Down));
        assert_eq!(app.theme.id, "ramas-light");
        let mut term = Terminal::new(TestBackend::new(100, 20)).unwrap();
        term.draw(|f| super::draw(f, &mut app)).unwrap();
        assert_eq!(term.backend().buffer()[(5, 12)].bg, app.theme.bg);
        assert_ne!(app.theme.bg, ratatui::style::Color::Reset);
        app.on_key(key(KeyCode::Esc));
        assert_eq!(app.overlay, Overlay::None);
        assert_eq!(app.theme.id, "ramas");
    }

    #[test]
    fn picker_filters_by_typing_and_enter_keeps_the_theme() {
        let mut app = app();
        press(&mut app, "T");
        // Letters filter instead of acting as shortcuts: `q` does not quit.
        press(&mut app, "gruvq");
        assert!(!app.quit);
        app.on_key(key(KeyCode::Backspace));
        assert_eq!(app.theme.id, "gruvbox-dark");
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.overlay, Overlay::None);
        assert_eq!(app.theme.id, "gruvbox-dark");
        assert_eq!(app.message.as_deref(), Some("Theme: Gruvbox Dark"));
    }

    #[test]
    fn enter_saves_the_theme_and_esc_does_not() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "# mine\nicons = \"ascii\"\n").unwrap();
        let mut app = app();
        app.config_path = Some(path.clone());
        press(&mut app, "T");
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Esc));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "# mine\nicons = \"ascii\"\n"
        );
        press(&mut app, "T");
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Enter));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "# mine\nicons = \"ascii\"\ntheme = \"ramas-light\"\n"
        );
        let saved = crate::config::Config::load(Some(&path)).unwrap();
        assert_eq!(saved.theme.as_deref(), Some("ramas-light"));
        // A config that cannot be written is reported, not fatal.
        app.config_path = Some("/proc/nope/config.toml".into());
        press(&mut app, "T");
        app.on_key(key(KeyCode::Enter));
        assert!(app.message.as_deref().unwrap().contains("/proc/nope"));
        assert_eq!(app.theme.id, "ramas-light");
    }

    #[test]
    fn picker_is_ascii_with_ascii_icons_and_translated() {
        let mut app = app();
        app.icons.mode = crate::cli::IconMode::Ascii;
        app.s = &crate::i18n::ES;
        press(&mut app, "T");
        let mut term = Terminal::new(TestBackend::new(100, 24)).unwrap();
        term.draw(|f| super::draw(f, &mut app)).unwrap();
        let buf = term.backend().buffer();
        let panel = tui_themes::Picker::area(ratatui::layout::Rect::new(0, 0, 100, 22));
        let mut text = String::new();
        for y in panel.top()..panel.bottom() {
            for x in panel.left()..panel.right() {
                text.push_str(buf[(x, y)].symbol());
            }
            text.push('\n');
        }
        assert!(text.is_ascii(), "{text}");
        assert!(
            text.contains("Tema") && text.contains("Enter guardar"),
            "{text}"
        );
    }

    #[test]
    fn no_color_renders_without_colors() {
        let mut app = app();
        app.theme = default_theme().at(ColorDepth::NoColor);
        press(&mut app, "j");
        let mut term = Terminal::new(TestBackend::new(100, 20)).unwrap();
        term.draw(|f| super::draw(f, &mut app)).unwrap();
        let buf = term.backend().buffer();
        for y in 0..20 {
            for x in 0..100 {
                let c = &buf[(x, y)];
                assert_eq!(c.fg, ratatui::style::Color::Reset, "fg at {x},{y}");
                assert_eq!(c.bg, ratatui::style::Color::Reset, "bg at {x},{y}");
            }
        }
    }
}

#[cfg(test)]
mod icon_tests {
    use super::test_util::render;
    use crate::cli::IconMode;
    use crate::ui::app::test_util::*;
    use crate::ui::icons::Icons;

    fn has_private_use(text: &str) -> bool {
        text.chars().any(|c| ('\u{e000}'..='\u{f8ff}').contains(&c))
    }

    fn screen(mode: IconMode) -> String {
        let mut app = app();
        app.icons = Icons { mode };
        press(&mut app, "jl"); // expand a
        press(&mut app, "j "); // mark y
        render(&mut app, 120, 24).join("\n")
    }

    #[test]
    fn nerd_mode_uses_nerd_glyphs() {
        assert!(has_private_use(&screen(IconMode::Nerd)));
    }

    #[test]
    fn fallbacks_need_no_nerd_font() {
        let unicode = screen(IconMode::Unicode);
        assert!(!has_private_use(&unicode));
        assert!(
            unicode.contains("├── ") && unicode.contains('▾'),
            "{unicode}"
        );
        let ascii = screen(IconMode::Ascii);
        assert!(!has_private_use(&ascii));
        assert!(ascii.contains("|-- ") && ascii.contains("`-- "), "{ascii}");
        assert!(ascii.contains("+--"), "ascii borders: {ascii}");
        assert!(!ascii.contains('█') && !ascii.contains('├'), "{ascii}");
    }
}

#[cfg(test)]
mod i18n_tests {
    use super::test_util::render;
    use crate::ui::app::test_util::*;

    #[test]
    fn spanish_ui_end_to_end() {
        let mut app = app();
        app.s = &crate::i18n::ES;
        press(&mut app, "j");
        let body = render(&mut app, 130, 30).join("\n");
        assert!(body.contains("Detalle"), "{body}");
        assert!(body.contains("orden: tamaño · uso en disco"), "{body}");
        assert!(
            body.contains("0 elementos marcados — 0 B a liberar"),
            "{body}"
        );
        assert!(body.contains("q salir"), "{body}");
        assert!(body.contains("Archivos más pesados"), "{body}");
        press(&mut app, "?");
        let body = render(&mut app, 130, 30).join("\n");
        assert!(
            body.contains("Teclas") && body.contains("ciclar orden"),
            "{body}"
        );
        press(&mut app, "x");
        press(&mut app, "s");
        let body = render(&mut app, 130, 30).join("\n");
        assert!(body.contains("orden: nombre"), "{body}");
    }

    #[test]
    fn english_is_default() {
        let mut app = app();
        let body = render(&mut app, 130, 30).join("\n");
        assert!(
            body.contains("Details") && body.contains("q quit"),
            "{body}"
        );
    }
}
