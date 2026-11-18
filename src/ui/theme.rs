//! Theme selection for the UI: a thin layer over [`crate::themes`].
//!
//! No color is defined here. Themes are sets of semantic roles loaded by
//! `crate::themes`, which also adapts them to what the terminal can show.
//! This module adds what is specific to ramas' command line and config:
//! the theme names of ramas 0.1, `[custom_theme]` and the startup choice.

use std::collections::BTreeMap;

pub use crate::themes::{Catalog, ColorDepth, Theme};

/// Theme used when none is chosen: ramas' original dark look.
pub const DEFAULT: &str = "ramas";

/// Id of the theme built from the `[custom_theme]` config table.
pub const CUSTOM: &str = "custom";

/// Current id for a theme name, accepting the names used by ramas 0.1.
pub fn canonical(name: &str) -> &str {
    match name {
        "dark" => DEFAULT,
        "light" => "ramas-light",
        "high-contrast" => "high-contrast-dark",
        "gruvbox" => "gruvbox-dark",
        other => other,
    }
}

/// Every available theme and the color depth of this terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Themes {
    /// Embedded and user themes, plus `custom` when the config defines it.
    pub catalog: Catalog,
    pub depth: ColorDepth,
}

impl Themes {
    /// Theme by id, ready to draw with; `None` when it does not exist.
    pub fn get(&self, id: &str) -> Option<Theme> {
        self.catalog.get(id).map(|t| t.at(self.depth))
    }
}

impl Default for Themes {
    fn default() -> Self {
        Themes {
            catalog: Catalog::builtin(),
            depth: ColorDepth::TrueColor,
        }
    }
}

/// The default theme at full color depth.
pub fn default_theme() -> Theme {
    Catalog::builtin().get_or_plain(DEFAULT)
}

/// Theme from a `[custom_theme]` table: `base` picks the starting theme
/// (default dark), every other key overrides one color.
pub fn custom(catalog: &Catalog, spec: &BTreeMap<String, String>) -> Result<Theme, String> {
    let base = spec.get("base").map_or("dark", String::as_str);
    let base = catalog
        .get(canonical(base))
        .ok_or_else(|| format!("unknown base theme {base:?}"))?;
    let pairs = spec
        .iter()
        .filter(|(k, _)| k.as_str() != "base")
        .map(|(k, v)| (k.as_str(), v.as_str()));
    base.with_overrides(CUSTOM, pairs)
        .map_err(|e| format!("[custom_theme] {e}"))
}

/// What the user asked for, from the command line and the config file.
#[derive(Debug, Clone, Copy)]
pub struct Request<'a> {
    /// `--theme`: a wrong name is an error.
    pub cli: Option<&'a str>,
    /// `theme = "…"` in the config: a wrong name falls back to the default
    /// theme with a notice, so a deleted or broken theme file never keeps
    /// ramas from starting.
    pub config: Option<&'a str>,
    /// The `[custom_theme]` table (empty when absent).
    pub custom: &'a BTreeMap<String, String>,
}

/// Outcome of [`start`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Start {
    pub theme: Theme,
    pub themes: Themes,
    /// Something the user should know: an unknown configured theme, or
    /// theme files that failed to load.
    pub notice: Option<String>,
}

/// Picks the theme to start with. `failed` is the text for "N theme files
/// could not be loaded" and `unknown` the one for an unknown theme name.
pub fn start(
    catalog: Catalog,
    request: Request<'_>,
    depth: ColorDepth,
    unknown: &str,
    failed: &str,
) -> Result<Start, String> {
    let mut notice = match catalog.errors().len() {
        0 => None,
        n => Some(format!("{n} {failed}")),
    };
    if let Some(name) = request.cli {
        let (theme, themes) = select(catalog, Some(name), request.custom, depth)?;
        return Ok(Start {
            theme,
            themes,
            notice,
        });
    }
    let (theme, themes) = match select(catalog.clone(), request.config, request.custom, depth) {
        Ok(found) => found,
        Err(_)
            if request
                .config
                .is_some_and(|n| catalog.get(canonical(n)).is_none() && n != CUSTOM) =>
        {
            notice = Some(format!(
                "{unknown} {:?}",
                request.config.unwrap_or_default()
            ));
            select(catalog, None, request.custom, depth)?
        }
        Err(e) => return Err(e),
    };
    Ok(Start {
        theme,
        themes,
        notice,
    })
}

/// The active theme, converted to `depth`, and the set it was picked from.
///
/// `name` is the `--theme` or `theme = "…"` value; `spec` is the
/// `[custom_theme]` table (empty when absent).
pub fn select(
    mut catalog: Catalog,
    name: Option<&str>,
    spec: &BTreeMap<String, String>,
    depth: ColorDepth,
) -> Result<(Theme, Themes), String> {
    if !spec.is_empty() {
        let theme = custom(&catalog, spec)?;
        catalog.push(theme);
    }
    let name = name.unwrap_or(DEFAULT);
    if name == CUSTOM && spec.is_empty() {
        return Err("theme \"custom\" needs a [custom_theme] table in the config".into());
    }
    let active = catalog
        .get(canonical(name))
        .ok_or_else(|| {
            format!(
                "unknown theme {name:?} (available: {})",
                catalog.ids().join(", ")
            )
        })?
        .at(depth);
    Ok((active, Themes { catalog, depth }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::{Color, Style};

    fn spec(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn pick(name: Option<&str>, pairs: &[(&str, &str)]) -> Result<Theme, String> {
        select(
            Catalog::builtin(),
            name,
            &spec(pairs),
            ColorDepth::TrueColor,
        )
        .map(|(t, _)| t)
    }

    #[test]
    fn default_is_the_original_dark_theme() {
        let t = pick(None, &[]).unwrap();
        assert_eq!(t.id, DEFAULT);
        assert_eq!(t.bg, Color::Reset);
        assert_eq!(t.fg, Color::Rgb(0xd8, 0xde, 0xe9));
        assert_eq!(t.size_color(0.0), Color::Rgb(0xa3, 0xbe, 0x8c));
        assert_eq!(t.size_color(0.5), Color::Rgb(0xeb, 0xcb, 0x8b));
        assert_eq!(t.size_color(1.0), Color::Rgb(0xbf, 0x61, 0x6a));
        assert_eq!(t, default_theme());
    }

    #[test]
    fn old_theme_names_still_work() {
        for (old, id) in [
            ("dark", "ramas"),
            ("light", "ramas-light"),
            ("high-contrast", "high-contrast-dark"),
            ("gruvbox", "gruvbox-dark"),
        ] {
            assert_eq!(pick(Some(old), &[]).unwrap().id, id);
            assert_eq!(pick(Some(id), &[]).unwrap().id, id);
        }
        let err = pick(Some("nope"), &[]).unwrap_err();
        assert!(
            err.starts_with("unknown theme \"nope\" (available: "),
            "{err}"
        );
        assert!(err.contains("gruvbox-dark"), "{err}");
    }

    #[test]
    fn custom_theme_overrides_colors() {
        let pairs = [
            ("base", "light"),
            ("fg", "#112233"),
            ("selected_bg", "blue"),
        ];
        let t = pick(Some("custom"), &pairs).unwrap();
        assert_eq!(t.id, CUSTOM);
        assert_eq!(t.fg, Color::Rgb(0x11, 0x22, 0x33));
        assert_eq!(t.selection_bg, Color::Blue);
        assert_eq!(t.bg, pick(Some("light"), &[]).unwrap().bg);
        // The custom theme is listed with the others even when not active.
        let (active, all) = select(
            Catalog::builtin(),
            None,
            &spec(&pairs),
            ColorDepth::TrueColor,
        )
        .unwrap();
        assert_eq!(active.id, DEFAULT);
        assert!(all.get(CUSTOM).is_some());
    }

    #[test]
    fn custom_theme_errors_are_clear() {
        let err = |pairs: &[(&str, &str)]| pick(Some("custom"), pairs).unwrap_err();
        assert!(err(&[]).contains("needs a [custom_theme] table"));
        assert!(err(&[("fg", "notacolor")]).starts_with("[custom_theme] fg: invalid color"));
        assert!(err(&[("background", "#000000")]).contains("unknown theme key \"background\""));
        assert_eq!(err(&[("base", "nope")]), "unknown base theme \"nope\"");
    }

    #[test]
    fn a_wrong_config_theme_falls_back_but_a_wrong_cli_theme_fails() {
        let none = spec(&[]);
        let go = |cli, config| {
            start(
                Catalog::builtin(),
                Request {
                    cli,
                    config,
                    custom: &none,
                },
                ColorDepth::TrueColor,
                "Unknown theme",
                "theme file(s) failed",
            )
        };
        let ok = go(None, Some("gruvbox")).unwrap();
        assert_eq!(ok.theme.id, "gruvbox-dark");
        assert_eq!(ok.notice, None);
        let fallback = go(None, Some("gone")).unwrap();
        assert_eq!(fallback.theme.id, DEFAULT);
        assert_eq!(fallback.notice.as_deref(), Some("Unknown theme \"gone\""));
        assert!(
            go(Some("gone"), None)
                .unwrap_err()
                .starts_with("unknown theme")
        );
        // The command line wins over the config.
        assert_eq!(
            go(Some("light"), Some("gone")).unwrap().theme.id,
            "ramas-light"
        );
        // `custom` without its table stays an error: the config is wrong.
        assert!(go(None, Some("custom")).is_err());
    }

    #[test]
    fn broken_user_theme_files_only_produce_a_notice() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("bad.toml"), "nope").unwrap();
        let none = spec(&[]);
        let started = start(
            Catalog::with_user_dir(dir.path()),
            Request {
                cli: None,
                config: Some("bad"),
                custom: &none,
            },
            ColorDepth::TrueColor,
            "Unknown theme",
            "theme file(s) failed",
        )
        .unwrap();
        assert_eq!(started.theme.id, DEFAULT);
        assert_eq!(started.notice.as_deref(), Some("Unknown theme \"bad\""));
        assert_eq!(started.themes.catalog.errors().len(), 1);
    }

    #[test]
    fn themes_follow_the_color_depth() {
        let at = |depth| select(Catalog::builtin(), None, &spec(&[]), depth).unwrap();
        let (t, all) = at(ColorDepth::Ansi256);
        assert!(matches!(t.fg, Color::Indexed(_)));
        assert_eq!(all.depth, ColorDepth::Ansi256);
        assert!(matches!(
            all.get("gruvbox").or(all.get("gruvbox-dark")).unwrap().bg,
            Color::Indexed(_)
        ));
        let (plain, _) = at(ColorDepth::NoColor);
        assert_eq!(plain.fg(plain.value), Style::new());
        assert_eq!(plain.text(), Style::new());
    }
}
