//! The set of available themes: embedded ones plus the user's own files.

use std::path::{Path, PathBuf};

use crate::themes::theme::{Origin, Theme, ThemeError};

/// Embeds `themes/<dir>/<id>.toml` for every listed theme.
macro_rules! builtin {
    ($($dir:literal / $id:literal),* $(,)?) => {
        &[$(($id, include_str!(concat!("../../themes/", $dir, "/", $id, ".toml")))),*]
    };
}

/// Embedded themes as `(id, toml)`, in picker order.
const BUILTIN: &[(&str, &str)] = builtin![
    "default" / "ramas",
    "default" / "ramas-light",
    "popular" / "catppuccin-mocha",
    "popular" / "catppuccin-macchiato",
    "popular" / "catppuccin-frappe",
    "popular" / "catppuccin-latte",
    "popular" / "dracula",
    "popular" / "everforest",
    "popular" / "gruvbox-dark",
    "popular" / "gruvbox-light",
    "popular" / "kanagawa",
    "popular" / "monokai",
    "popular" / "nord",
    "popular" / "one-dark",
    "popular" / "rose-pine",
    "popular" / "solarized-dark",
    "popular" / "solarized-light",
    "popular" / "tokyo-night",
    "light" / "paper",
    "light" / "pastel-dawn",
    "light" / "mint-cream",
    "light" / "lavender-mist",
    "retro" / "phosphor-green",
    "retro" / "amber",
    "retro" / "mono-gray",
    "retro" / "cyberpunk",
    "retro" / "synthwave",
    "accessible" / "high-contrast-dark",
    "accessible" / "high-contrast-light",
    "accessible" / "okabe-ito-dark",
    "accessible" / "okabe-ito-light",
];

/// Family given to user themes that do not name one.
pub const USER_FAMILY: &str = "User";

/// Directory of user themes: `$XDG_CONFIG_HOME/ramas/themes`, or
/// `~/.config/ramas/themes`.
pub fn user_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|v| !v.is_empty())
                .map(|h| PathBuf::from(h).join(".config"))
        })?;
    Some(base.join("ramas").join("themes"))
}

/// Themes at true-color depth, plus the files that failed to load.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Catalog {
    themes: Vec<Theme>,
    errors: Vec<ThemeError>,
}

impl Catalog {
    /// Only the embedded themes.
    pub fn builtin() -> Self {
        let mut catalog = Catalog::default();
        for (id, text) in BUILTIN {
            match Theme::from_toml(id, text, Origin::Builtin) {
                Ok(theme) => catalog.themes.push(theme),
                Err(e) => catalog.errors.push(e),
            }
        }
        catalog
    }

    /// Embedded themes plus the ones in [`user_dir`].
    pub fn load() -> Self {
        match user_dir() {
            Some(dir) => Self::with_user_dir(&dir),
            None => Self::builtin(),
        }
    }

    /// Embedded themes plus every `*.toml` in `dir`. A missing directory is
    /// not an error; a broken file is recorded in [`Catalog::errors`] and
    /// skipped. A user theme replaces an embedded one with the same id.
    pub fn with_user_dir(dir: &Path) -> Self {
        let mut catalog = Self::builtin();
        let Ok(entries) = std::fs::read_dir(dir) else {
            return catalog;
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect();
        paths.sort();
        for path in paths {
            let Some(id) = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(str::to_lowercase)
            else {
                continue;
            };
            let loaded = std::fs::read_to_string(&path)
                .map_err(|e| ThemeError {
                    source: path.display().to_string(),
                    message: e.to_string(),
                })
                .and_then(|text| Theme::from_toml(&id, &text, Origin::User(path.clone())));
            match loaded {
                Ok(theme) => catalog.push(theme),
                Err(e) => catalog.errors.push(e),
            }
        }
        catalog
    }

    /// Adds a theme, replacing any with the same id.
    pub fn push(&mut self, theme: Theme) {
        match self.themes.iter_mut().find(|t| t.id == theme.id) {
            Some(slot) => *slot = theme,
            None => self.themes.push(theme),
        }
    }

    /// Theme by id, ignoring case.
    pub fn get(&self, id: &str) -> Option<&Theme> {
        self.themes.iter().find(|t| t.id.eq_ignore_ascii_case(id))
    }

    /// Theme by id, or [`Theme::plain`] when it does not exist.
    pub fn get_or_plain(&self, id: &str) -> Theme {
        self.get(id).cloned().unwrap_or_else(Theme::plain)
    }

    /// Every theme, grouped by family in first-seen order.
    pub fn themes(&self) -> Vec<&Theme> {
        let mut families: Vec<&str> = Vec::new();
        for t in &self.themes {
            if !families.contains(&t.family.as_str()) {
                families.push(&t.family);
            }
        }
        families
            .iter()
            .flat_map(|f| self.themes.iter().filter(move |t| t.family == *f))
            .collect()
    }

    /// Ids in picker order.
    pub fn ids(&self) -> Vec<&str> {
        self.themes().iter().map(|t| t.id.as_str()).collect()
    }

    /// Files that could not be loaded.
    pub fn errors(&self) -> &[ThemeError] {
        &self.errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::themes::ColorDepth;
    use crate::themes::theme::{GRADIENTS, REQUIRED_ROLES};
    use ratatui::style::Color;

    #[test]
    fn every_builtin_loads() {
        let catalog = Catalog::builtin();
        assert_eq!(catalog.errors(), &[]);
        assert_eq!(catalog.themes().len(), BUILTIN.len());
    }

    #[test]
    fn every_file_in_themes_is_embedded() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("themes");
        let mut on_disk = Vec::new();
        for family in std::fs::read_dir(&root).unwrap() {
            for file in std::fs::read_dir(family.unwrap().path()).unwrap() {
                let path = file.unwrap().path();
                on_disk.push(path.file_stem().unwrap().to_str().unwrap().to_string());
            }
        }
        on_disk.sort();
        let mut embedded: Vec<String> = BUILTIN.iter().map(|(id, _)| id.to_string()).collect();
        embedded.sort();
        assert_eq!(on_disk, embedded);
    }

    /// Embedded themes spell out every required role and both gradients, and
    /// give themselves a name and a family.
    #[test]
    fn every_builtin_defines_all_roles() {
        for (id, text) in BUILTIN {
            let raw: toml::Table = toml::from_str(text).unwrap();
            let colors = raw["colors"].as_table().unwrap();
            for role in REQUIRED_ROLES {
                assert!(colors.contains_key(role), "{id}: colors.{role}");
            }
            let gradients = raw["gradients"].as_table().unwrap();
            for g in GRADIENTS {
                assert!(gradients.contains_key(g), "{id}: gradients.{g}");
            }
            for key in ["name", "family", "appearance", "icon_mode"] {
                assert!(raw.contains_key(key), "{id}: {key}");
            }
        }
    }

    #[test]
    fn ids_are_unique_lowercase_and_names_differ() {
        let catalog = Catalog::builtin();
        let themes = catalog.themes();
        for (i, a) in themes.iter().enumerate() {
            assert_eq!(a.id, a.id.to_lowercase());
            for b in &themes[i + 1..] {
                assert_ne!(a.id, b.id);
                assert_ne!(a.name, b.name);
            }
        }
    }

    /// Text over background: at least 4.5:1, and 7:1 for high-contrast
    /// themes. Secondary pairs must stay readable too.
    #[test]
    fn every_builtin_meets_contrast() {
        for t in Catalog::builtin().themes() {
            let floor = if t.high_contrast { 7.0 } else { 4.5 };
            let check = |what: &str, fg: Color, bg: Color, min: f64| {
                let ratio = t.contrast(fg, bg);
                assert!(ratio >= min, "{}: {what} contrast {ratio:.2} < {min}", t.id);
            };
            check("text", t.fg, t.bg, floor);
            check("selection", t.selection_fg, t.selection_bg, 4.5);
            check("status", t.status_fg, t.status_bg, 4.5);
            for (what, c) in [
                ("accent", t.accent),
                ("ok", t.ok),
                ("warn", t.warn),
                ("danger", t.danger),
                ("header", t.header),
                ("dir", t.dir),
                ("marked", t.marked),
                ("matched", t.matched),
                ("value", t.value),
                ("ram", t.ram),
                ("icon", t.icon),
                ("dim", t.dim),
            ] {
                check(what, c, t.bg, 3.0);
            }
        }
    }

    #[test]
    fn the_catalog_has_every_family() {
        let catalog = Catalog::builtin();
        let count = |family: &str| {
            catalog
                .themes()
                .iter()
                .filter(|t| t.family == family)
                .count()
        };
        assert_eq!(count("Default"), 2);
        assert_eq!(count("Popular"), 16);
        assert_eq!(count("Light & pastel"), 4);
        assert_eq!(count("Retro & neon"), 5);
        assert_eq!(count("Accessible"), 4);
        assert!(catalog.themes().len() >= 28);
        let high: Vec<&str> = catalog
            .themes()
            .iter()
            .filter(|t| t.high_contrast)
            .map(|t| t.id.as_str())
            .collect();
        assert_eq!(high, ["high-contrast-dark", "high-contrast-light"]);
        let light = catalog
            .themes()
            .iter()
            .filter(|t| t.appearance == crate::themes::Appearance::Light)
            .count();
        assert!(light >= 4 + 4, "{light} light themes");
    }

    /// The color-blind safe themes keep ok, warn and danger apart for
    /// people with red-green color blindness (simulated with the Machado
    /// et al. 2009 matrices for full protanopia and deuteranopia).
    #[test]
    fn okabe_ito_states_survive_red_green_blindness() {
        const PROTAN: [[f64; 3]; 3] = [
            [0.152286, 1.052583, -0.204868],
            [0.114503, 0.786281, 0.099216],
            [-0.003882, -0.048116, 1.051998],
        ];
        const DEUTAN: [[f64; 3]; 3] = [
            [0.367322, 0.860646, -0.227968],
            [0.280085, 0.672501, 0.047413],
            [-0.011820, 0.042940, 0.968881],
        ];
        let seen = |m: &[[f64; 3]; 3], c: Color| {
            let (r, g, b) = crate::themes::color::to_rgb(c).unwrap();
            let v = [f64::from(r), f64::from(g), f64::from(b)];
            m.map(|row| (row[0] * v[0] + row[1] * v[1] + row[2] * v[2]).clamp(0.0, 255.0))
        };
        let apart = |a: [f64; 3], b: [f64; 3]| {
            ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
        };
        let catalog = Catalog::builtin();
        for id in ["okabe-ito-dark", "okabe-ito-light"] {
            let t = catalog.get(id).unwrap();
            for (name, matrix) in [("protanopia", &PROTAN), ("deuteranopia", &DEUTAN)] {
                for (x, y) in [(t.ok, t.warn), (t.ok, t.danger), (t.warn, t.danger)] {
                    let d = apart(seen(matrix, x), seen(matrix, y));
                    assert!(d >= 50.0, "{id} under {name}: {x:?} vs {y:?} = {d:.0}");
                }
            }
        }
    }

    #[test]
    fn every_builtin_degrades_cleanly() {
        for t in Catalog::builtin().themes() {
            let all = |t: &Theme| {
                let mut v: Vec<Color> = REQUIRED_ROLES
                    .iter()
                    .chain(crate::themes::theme::OPTIONAL_ROLES.iter())
                    .filter_map(|k| t.role(k))
                    .collect();
                v.extend(t.graph.iter().chain(t.size.iter()));
                v.extend((0..=10).map(|i| t.graph_color(f64::from(i) / 10.0)));
                v.extend((0..=10).map(|i| t.size_color(f64::from(i) / 10.0)));
                v
            };
            for c in all(&t.at(ColorDepth::Ansi256)) {
                assert!(!matches!(c, Color::Rgb(..)), "{}: rgb at 256", t.id);
            }
            let low = t.at(ColorDepth::Ansi16);
            for c in all(&low) {
                assert!(
                    !matches!(c, Color::Rgb(..) | Color::Indexed(_)),
                    "{}: {c:?} at 16",
                    t.id
                );
            }
            assert_ne!(low.selection_fg, low.selection_bg, "{}", t.id);
            assert_ne!(low.status_fg, low.status_bg, "{}", t.id);
            let plain = t.at(ColorDepth::NoColor);
            assert_eq!(plain.text(), ratatui::style::Style::new(), "{}", t.id);
        }
    }

    #[test]
    fn user_themes_load_and_broken_ones_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        let good = include_str!("../../themes/popular/nord.toml").replace("\"Nord\"", "\"Mine\"");
        std::fs::write(dir.path().join("Mine.toml"), good).unwrap();
        std::fs::write(
            dir.path().join("broken.toml"),
            "[colors]\nbg = \"#000000\"\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("notes.txt"), "ignored").unwrap();
        let catalog = Catalog::with_user_dir(dir.path());
        let mine = catalog.get("mine").unwrap();
        assert_eq!(mine.name, "Mine");
        assert!(matches!(mine.origin, Origin::User(_)));
        assert_eq!(catalog.errors().len(), 1);
        let e = catalog.errors()[0].to_string();
        assert!(e.contains("broken.toml"), "{e}");
        assert!(e.contains("colors.fg: missing required role"), "{e}");
        assert_eq!(catalog.themes().len(), BUILTIN.len() + 1);
    }

    #[test]
    fn a_user_theme_replaces_a_builtin_with_the_same_id() {
        let dir = tempfile::tempdir().unwrap();
        let text = include_str!("../../themes/popular/nord.toml").replace("#88c0d0", "#123456");
        std::fs::write(dir.path().join("nord.toml"), text).unwrap();
        let catalog = Catalog::with_user_dir(dir.path());
        assert_eq!(catalog.themes().len(), BUILTIN.len());
        assert_eq!(
            catalog.get("NORD").unwrap().accent,
            Color::Rgb(0x12, 0x34, 0x56)
        );
    }

    /// The commented example in `examples/` is a valid, readable theme, and
    /// it mentions every role so nobody has to guess a name.
    #[test]
    fn the_documented_example_is_a_valid_theme() {
        let text = include_str!("../../docs/example-theme.toml");
        let theme = Theme::from_toml("example", text, Origin::Builtin).unwrap();
        assert_eq!(theme.name, "Example");
        assert_eq!(theme.family, USER_FAMILY);
        assert!(theme.text_contrast() >= 4.5);
        assert!(theme.contrast(theme.selection_fg, theme.selection_bg) >= 4.5);
        for role in REQUIRED_ROLES
            .iter()
            .chain(crate::themes::theme::OPTIONAL_ROLES.iter())
        {
            assert!(text.contains(&format!("\n{role} = ")), "{role}");
        }
        for gradient in GRADIENTS {
            assert!(text.contains(&format!("\n{gradient} = ")), "{gradient}");
        }
    }

    /// The guides list every embedded theme and every role.
    #[test]
    fn the_guides_list_every_theme_and_role() {
        for doc in [
            include_str!("../../docs/THEMES.md"),
            include_str!("../../docs/THEMES.es.md"),
        ] {
            for (id, _) in BUILTIN {
                assert!(doc.contains(&format!("`{id}`")), "{id}");
            }
            for role in REQUIRED_ROLES
                .iter()
                .chain(crate::themes::theme::OPTIONAL_ROLES.iter())
            {
                assert!(doc.contains(&format!("`{role}`")), "{role}");
            }
        }
    }

    #[test]
    fn a_missing_user_dir_is_fine() {
        let catalog = Catalog::with_user_dir(Path::new("/nonexistent/themes"));
        assert_eq!(catalog, Catalog::builtin());
    }
}
