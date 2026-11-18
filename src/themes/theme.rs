//! The theme model: semantic roles, TOML loading and validation.

use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

use ratatui::style::{Color, Modifier, Style};
use serde::Deserialize;

use crate::themes::color::{self, ColorDepth, Rgb, ansi16_index, degrade, named, parse_color};

/// Roles every theme file must define under `[colors]`.
pub const REQUIRED_ROLES: [&str; 10] = [
    "bg",
    "fg",
    "dim",
    "accent",
    "border",
    "selection_fg",
    "selection_bg",
    "ok",
    "warn",
    "danger",
];

/// Roles a theme may define; when missing they are derived from the required
/// ones (see `docs/THEMES.md`).
pub const OPTIONAL_ROLES: [&str; 13] = [
    "header",
    "connector",
    "dir",
    "marked",
    "matched",
    "status_fg",
    "status_bg",
    "grew",
    "shrank",
    "new",
    "ram",
    "value",
    "icon",
];

/// Gradients under `[gradients]`. Each is a list of two or more colors, low
/// to high. `size` colors the size bars.
pub const GRADIENTS: [&str; 2] = ["graph", "size"];

/// Whether a theme is meant for a dark or a light terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    Dark,
    Light,
}

/// Icon coloring mode. ramas has no app icons: the key is read so a theme
/// file that carries it still loads, and then ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconMode {
    /// Each icon keeps the dominant color of the real app icon.
    #[default]
    App,
    /// Each icon takes the palette color closest to its own.
    Tint,
    /// Every icon uses the theme's `icon` color.
    Mono,
}

impl IconMode {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "app" => Some(IconMode::App),
            "tint" => Some(IconMode::Tint),
            "mono" => Some(IconMode::Mono),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            IconMode::App => "app",
            IconMode::Tint => "tint",
            IconMode::Mono => "mono",
        }
    }
}

/// Where a theme came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// Embedded in the binary.
    Builtin,
    /// A file in the user's theme directory.
    User(PathBuf),
    /// Built from the `[custom_theme]` table of the config.
    Config,
}

/// A theme that failed to load: which file and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeError {
    /// File path, or the theme id for embedded themes.
    pub source: String,
    pub message: String,
}

impl fmt::Display for ThemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.source, self.message)
    }
}

impl std::error::Error for ThemeError {}

/// A set of semantic colors, resolved for one color depth.
///
/// Themes come out of the [`crate::themes::Catalog`] at true-color depth; call
/// [`Theme::at`] once with the terminal's depth before drawing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    /// Stable identifier: the file name without `.toml`.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Group shown in the picker.
    pub family: String,
    pub appearance: Appearance,
    /// Held to the stricter 7:1 contrast rule.
    pub high_contrast: bool,
    pub icon_mode: IconMode,
    pub origin: Origin,
    pub depth: ColorDepth,
    /// Background to assume for contrast math when `bg` is the terminal's.
    pub assumed_bg: Rgb,

    pub bg: Color,
    pub fg: Color,
    pub dim: Color,
    pub accent: Color,
    pub border: Color,
    pub selection_fg: Color,
    pub selection_bg: Color,
    pub ok: Color,
    pub warn: Color,
    pub danger: Color,

    pub header: Color,
    pub connector: Color,
    pub dir: Color,
    pub marked: Color,
    pub matched: Color,
    pub status_fg: Color,
    pub status_bg: Color,
    pub grew: Color,
    pub shrank: Color,
    pub new: Color,
    pub ram: Color,
    pub value: Color,
    pub icon: Color,

    /// Chart gradient, low to high.
    pub graph: Vec<Color>,
    /// Size-bar gradient, small to large.
    pub size: Vec<Color>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTheme {
    name: Option<String>,
    family: Option<String>,
    appearance: Option<String>,
    high_contrast: Option<bool>,
    icon_mode: Option<String>,
    assumed_bg: Option<String>,
    #[serde(default)]
    colors: BTreeMap<String, String>,
    #[serde(default)]
    gradients: BTreeMap<String, Vec<String>>,
}

/// `[custom_theme]` key names of ramas 0.1 and their current role.
fn alias(key: &str) -> &str {
    match key {
        "selected_fg" => "selection_fg",
        "selected_bg" => "selection_bg",
        "error" => "danger",
        "size" => "value",
        other => other,
    }
}

impl Theme {
    /// Parses a theme file. `id` is the file name without extension.
    pub fn from_toml(id: &str, text: &str, origin: Origin) -> Result<Theme, ThemeError> {
        let source = match &origin {
            Origin::User(path) => path.display().to_string(),
            _ => id.to_string(),
        };
        Self::parse(id, text, origin).map_err(|message| ThemeError { source, message })
    }

    fn parse(id: &str, text: &str, origin: Origin) -> Result<Theme, String> {
        // One line: where and what, without toml's multi-line excerpt.
        let raw: RawTheme = toml::from_str(text).map_err(|e| match e.span() {
            Some(span) => {
                let line = text[..span.start.min(text.len())].matches('\n').count() + 1;
                format!("line {line}: {}", e.message())
            }
            None => e.message().to_string(),
        })?;

        for key in raw.colors.keys() {
            if !REQUIRED_ROLES.contains(&key.as_str()) && !OPTIONAL_ROLES.contains(&key.as_str()) {
                return Err(format!("colors.{key}: unknown role"));
            }
        }
        for key in raw.gradients.keys() {
            if !GRADIENTS.contains(&key.as_str()) {
                return Err(format!("gradients.{key}: unknown gradient"));
            }
        }
        let role = |key: &str| -> Result<Option<Color>, String> {
            raw.colors
                .get(key)
                .map(|v| parse_color(v).map_err(|e| format!("colors.{key}: {e}")))
                .transpose()
        };
        let required = |key: &str| -> Result<Color, String> {
            role(key)?.ok_or_else(|| format!("colors.{key}: missing required role"))
        };
        let gradient = |key: &str, fallback: [Color; 3]| -> Result<Vec<Color>, String> {
            let Some(stops) = raw.gradients.get(key) else {
                return Ok(fallback.to_vec());
            };
            if stops.len() < 2 {
                return Err(format!("gradients.{key}: needs at least 2 colors"));
            }
            stops
                .iter()
                .map(|v| match parse_color(v) {
                    Ok(Color::Reset) => {
                        Err(format!("gradients.{key}: \"terminal\" is not allowed"))
                    }
                    Ok(c) => Ok(c),
                    Err(e) => Err(format!("gradients.{key}: {e}")),
                })
                .collect()
        };

        let bg = required("bg")?;
        let fg = required("fg")?;
        let dim = required("dim")?;
        let accent = required("accent")?;
        let border = required("border")?;
        let selection_fg = required("selection_fg")?;
        let selection_bg = required("selection_bg")?;
        let ok = required("ok")?;
        let warn = required("warn")?;
        let danger = required("danger")?;

        let assumed = match &raw.assumed_bg {
            Some(v) => {
                let c = parse_color(v).map_err(|e| format!("assumed_bg: {e}"))?;
                Some(color::to_rgb(c).ok_or("assumed_bg: \"terminal\" is not allowed")?)
            }
            None => None,
        };
        let known_bg = color::to_rgb(bg).or(assumed);
        let appearance = match raw.appearance.as_deref() {
            Some("dark") => Appearance::Dark,
            Some("light") => Appearance::Light,
            Some(other) => {
                return Err(format!(
                    "appearance: {other:?} is not \"dark\" or \"light\""
                ));
            }
            None => match known_bg {
                Some(rgb) if color::luminance(rgb) > 0.4 => Appearance::Light,
                _ => Appearance::Dark,
            },
        };
        let assumed_bg = known_bg.unwrap_or(match appearance {
            Appearance::Dark => (0, 0, 0),
            Appearance::Light => (255, 255, 255),
        });
        let icon_mode = match raw.icon_mode.as_deref() {
            None => IconMode::default(),
            Some(v) => IconMode::parse(v)
                .ok_or_else(|| format!("icon_mode: {v:?} is not \"app\", \"tint\" or \"mono\""))?,
        };

        Ok(Theme {
            id: id.to_string(),
            name: raw.name.clone().unwrap_or_else(|| id.to_string()),
            family: raw.family.clone().unwrap_or_else(|| "User".to_string()),
            appearance,
            high_contrast: raw.high_contrast.unwrap_or(false),
            icon_mode,
            origin,
            depth: ColorDepth::TrueColor,
            assumed_bg,
            header: role("header")?.unwrap_or(fg),
            connector: role("connector")?.unwrap_or(border),
            dir: role("dir")?.unwrap_or(accent),
            marked: role("marked")?.unwrap_or(accent),
            matched: role("matched")?.unwrap_or(warn),
            status_fg: role("status_fg")?.unwrap_or(selection_fg),
            status_bg: role("status_bg")?.unwrap_or(selection_bg),
            grew: role("grew")?.unwrap_or(danger),
            shrank: role("shrank")?.unwrap_or(ok),
            new: role("new")?.unwrap_or(accent),
            ram: role("ram")?.unwrap_or(accent),
            value: role("value")?.unwrap_or(warn),
            icon: role("icon")?.unwrap_or(accent),
            graph: gradient("graph", [ok, warn, danger])?,
            size: gradient("size", [ok, warn, danger])?,
            bg,
            fg,
            dim,
            accent,
            border,
            selection_fg,
            selection_bg,
            ok,
            warn,
            danger,
        })
    }

    /// A theme with no colors of its own: everything is the terminal
    /// default. Used when even the embedded themes cannot be read.
    pub fn plain() -> Theme {
        let r = Color::Reset;
        Theme {
            id: "plain".to_string(),
            name: "plain".to_string(),
            family: "Default".to_string(),
            appearance: Appearance::Dark,
            high_contrast: false,
            icon_mode: IconMode::App,
            origin: Origin::Builtin,
            depth: ColorDepth::TrueColor,
            assumed_bg: (0, 0, 0),
            bg: r,
            fg: r,
            dim: r,
            accent: r,
            border: r,
            selection_fg: r,
            selection_bg: r,
            ok: r,
            warn: r,
            danger: r,
            header: r,
            connector: r,
            dir: r,
            marked: r,
            matched: r,
            status_fg: r,
            status_bg: r,
            grew: r,
            shrank: r,
            new: r,
            ram: r,
            value: r,
            icon: r,
            graph: vec![r, r],
            size: vec![r, r],
        }
    }

    fn role_mut(&mut self, key: &str) -> Option<&mut Color> {
        Some(match key {
            "bg" => &mut self.bg,
            "fg" => &mut self.fg,
            "dim" => &mut self.dim,
            "accent" => &mut self.accent,
            "border" => &mut self.border,
            "selection_fg" => &mut self.selection_fg,
            "selection_bg" => &mut self.selection_bg,
            "ok" => &mut self.ok,
            "warn" => &mut self.warn,
            "danger" => &mut self.danger,
            "header" => &mut self.header,
            "connector" => &mut self.connector,
            "dir" => &mut self.dir,
            "marked" => &mut self.marked,
            "matched" => &mut self.matched,
            "status_fg" => &mut self.status_fg,
            "status_bg" => &mut self.status_bg,
            "grew" => &mut self.grew,
            "shrank" => &mut self.shrank,
            "new" => &mut self.new,
            "ram" => &mut self.ram,
            "value" => &mut self.value,
            "icon" => &mut self.icon,
            _ => return None,
        })
    }

    /// Color of a role by name, or `None` for an unknown role.
    pub fn role(&self, key: &str) -> Option<Color> {
        self.clone().role_mut(key).map(|c| *c)
    }

    /// Copy of this theme with some roles replaced, as `(role, color)` text
    /// pairs. Also accepts the key names of ramas 0.1 `[custom_theme]`
    /// (`selected_fg`, `error`, `size`, `bar_low`, `bar_mid`, `bar_high`).
    pub fn with_overrides<'a>(
        &self,
        id: &str,
        pairs: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Result<Theme, String> {
        let mut t = self.clone();
        t.id = id.to_string();
        t.name = id.to_string();
        t.origin = Origin::Config;
        t.family = crate::themes::catalog::USER_FAMILY.to_string();
        t.high_contrast = false;
        for (key, value) in pairs {
            let color = parse_color(value).map_err(|e| format!("{key}: {e}"))?;
            let bar = match key {
                "bar_low" => Some(0),
                "bar_mid" => Some(1),
                "bar_high" => Some(2),
                _ => None,
            };
            if let Some(i) = bar {
                if t.size.len() != 3 {
                    t.size = vec![
                        t.gradient(&t.size, 0.0),
                        t.gradient(&t.size, 0.5),
                        t.gradient(&t.size, 1.0),
                    ];
                }
                t.size[i] = color;
                continue;
            }
            match t.role_mut(alias(key)) {
                Some(slot) => *slot = color,
                None => return Err(format!("unknown theme key {key:?}")),
            }
        }
        Ok(t)
    }

    /// This theme converted to what `depth` can show. Call it on a
    /// true-color theme (as the catalog returns them).
    pub fn at(&self, depth: ColorDepth) -> Theme {
        let mut t = self.clone();
        t.depth = depth;
        match depth {
            ColorDepth::TrueColor | ColorDepth::NoColor => {}
            ColorDepth::Ansi256 => t.map_colors(|c| degrade(c, depth)),
            ColorDepth::Ansi16 => t.reduce_to_16(),
        }
        t
    }

    fn map_colors(&mut self, f: impl Fn(Color) -> Color) {
        for key in REQUIRED_ROLES.iter().chain(OPTIONAL_ROLES.iter()) {
            if let Some(slot) = self.role_mut(key) {
                *slot = f(*slot);
            }
        }
        for c in self.graph.iter_mut().chain(self.size.iter_mut()) {
            *c = f(*c);
        }
    }

    /// 16 colors: nearest basic color, never letting a foreground land on the
    /// background it is drawn over.
    fn reduce_to_16(&mut self) {
        let page = ansi16_index(self.bg, None).or(match self.appearance {
            Appearance::Dark => Some(0),
            Appearance::Light => Some(15),
        });
        let pick =
            |c: Color, exclude: Option<u8>| ansi16_index(c, exclude).map_or(Color::Reset, named);
        let selection_bg = pick(self.selection_bg, page);
        let status_bg = pick(self.status_bg, page);
        let selection_fg = pick(self.selection_fg, ansi16_index(selection_bg, None).or(page));
        let status_fg = pick(self.status_fg, ansi16_index(status_bg, None).or(page));
        let bg = pick(self.bg, None);
        self.map_colors(|c| pick(c, page));
        self.bg = bg;
        self.selection_bg = selection_bg;
        self.selection_fg = selection_fg;
        self.status_bg = status_bg;
        self.status_fg = status_fg;
    }

    /// False under `NO_COLOR`: styles then carry modifiers only.
    pub fn colored(&self) -> bool {
        self.depth != ColorDepth::NoColor
    }

    fn style(&self, fg: Color, bg: Color) -> Style {
        let mut s = Style::new();
        if self.colored() {
            if fg != Color::Reset {
                s = s.fg(fg);
            }
            if bg != Color::Reset {
                s = s.bg(bg);
            }
        }
        s
    }

    /// Foreground-only style for one of this theme's colors.
    pub fn fg(&self, color: Color) -> Style {
        self.style(color, Color::Reset)
    }

    /// Base text style. Fill the whole frame with it before drawing.
    pub fn text(&self) -> Style {
        self.style(self.fg, self.bg)
    }

    /// Secondary text.
    pub fn dimmed(&self) -> Style {
        if self.colored() {
            self.fg(self.dim)
        } else {
            Style::new().add_modifier(Modifier::DIM)
        }
    }

    /// Bold section headers.
    pub fn heading(&self) -> Style {
        self.fg(self.header).add_modifier(Modifier::BOLD)
    }

    /// The selected row or cell.
    pub fn selected(&self) -> Style {
        if self.colored() {
            self.style(self.selection_fg, self.selection_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::new().add_modifier(Modifier::REVERSED)
        }
    }

    /// The status bar.
    pub fn status(&self) -> Style {
        if self.colored() {
            self.style(self.status_fg, self.status_bg)
        } else {
            Style::new().add_modifier(Modifier::REVERSED)
        }
    }

    /// An arbitrary RGB color (e.g. an app icon's) at this theme's depth.
    pub fn rgb(&self, rgb: Rgb) -> Color {
        degrade(Color::Rgb(rgb.0, rgb.1, rgb.2), self.depth)
    }

    /// Color at position `f` (`0..=1`) of a gradient. Blends in RGB when the
    /// terminal can show it; otherwise steps to the nearest stop.
    pub fn gradient(&self, stops: &[Color], f: f64) -> Color {
        let Some(last) = stops.last() else {
            return Color::Reset;
        };
        let f = if f.is_nan() { 0.0 } else { f.clamp(0.0, 1.0) };
        let pos = f * (stops.len() - 1) as f64;
        let i = (pos.floor() as usize).min(stops.len() - 1);
        let t = pos - i as f64;
        if i + 1 >= stops.len() {
            return *last;
        }
        if t == 0.0 {
            return stops[i];
        }
        let (a, b) = (stops[i], stops[i + 1]);
        let blend = matches!(self.depth, ColorDepth::TrueColor | ColorDepth::Ansi256)
            && stops
                .iter()
                .all(|c| matches!(c, Color::Rgb(..) | Color::Indexed(16..)));
        match (blend, color::to_rgb(a), color::to_rgb(b)) {
            (true, Some(x), Some(y)) => self.rgb(color::mix(x, y, t)),
            _ if t < 0.5 => a,
            _ => b,
        }
    }

    /// Chart color for a load in `0..=1`.
    pub fn graph_color(&self, f: f64) -> Color {
        self.gradient(&self.graph, f)
    }

    /// Bar color for a relative size in `0..=1`.
    pub fn size_color(&self, f: f64) -> Color {
        self.gradient(&self.size, f)
    }

    /// Contrast ratio of the base text over the background, using
    /// `assumed_bg` and a plain black or white for terminal defaults.
    pub fn text_contrast(&self) -> f64 {
        self.contrast(self.fg, self.bg)
    }

    /// Contrast ratio of `fg` over `bg` under this theme's assumptions.
    pub fn contrast(&self, fg: Color, bg: Color) -> f64 {
        let back = color::to_rgb(bg).unwrap_or(self.assumed_bg);
        let front = color::to_rgb(fg).unwrap_or(match self.appearance {
            Appearance::Dark => (229, 229, 229),
            Appearance::Light => (0, 0, 0),
        });
        color::contrast_ratio(front, back)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r##"
        name = "Minimal"
        [colors]
        bg = "#101010"
        fg = "#f0f0f0"
        dim = "#808080"
        accent = "#00aaff"
        border = "#404040"
        selection_fg = "#ffffff"
        selection_bg = "#303060"
        ok = "#00cc66"
        warn = "#ffcc00"
        danger = "#ff3355"
    "##;

    fn minimal() -> Theme {
        Theme::from_toml("minimal", MINIMAL, Origin::Builtin).unwrap()
    }

    #[test]
    fn optional_roles_are_derived() {
        let t = minimal();
        assert_eq!(t.name, "Minimal");
        assert_eq!(t.family, "User");
        assert_eq!(t.appearance, Appearance::Dark);
        assert_eq!(t.icon_mode, IconMode::App);
        assert_eq!(t.header, t.fg);
        assert_eq!(t.connector, t.border);
        assert_eq!(t.dir, t.accent);
        assert_eq!(t.status_bg, t.selection_bg);
        assert_eq!(t.grew, t.danger);
        assert_eq!(t.shrank, t.ok);
        assert_eq!(t.graph, vec![t.ok, t.warn, t.danger]);
        assert_eq!(t.assumed_bg, (0x10, 0x10, 0x10));
    }

    #[test]
    fn errors_name_the_key() {
        let err = |text: &str| {
            Theme::from_toml("x", text, Origin::Builtin)
                .unwrap_err()
                .to_string()
        };
        let missing = MINIMAL.replace("warn = \"#ffcc00\"", "");
        assert_eq!(err(&missing), "x: colors.warn: missing required role");
        let bad = MINIMAL.replace("#00aaff", "azul");
        assert!(err(&bad).starts_with("x: colors.accent: invalid color \"azul\""));
        let unknown = format!("{MINIMAL}\nbackground = \"#000000\"");
        assert_eq!(err(&unknown), "x: colors.background: unknown role");
        let mode = format!("icon_mode = \"rainbow\"\n{MINIMAL}");
        assert!(err(&mode).contains("icon_mode"));
        let short = format!("{MINIMAL}\n[gradients]\ngraph = [\"#ffffff\"]");
        assert!(err(&short).contains("gradients.graph: needs at least 2 colors"));
        let broken = err("ok = 1\nthis is not toml");
        assert!(broken.starts_with("x: line 2: "), "{broken}");
        assert!(!broken.contains('\n'), "{broken}");
        let user = Theme::from_toml("x", "nope", Origin::User("/tmp/x.toml".into())).unwrap_err();
        assert_eq!(user.source, "/tmp/x.toml");
    }

    #[test]
    fn terminal_background_infers_from_appearance() {
        let text = MINIMAL.replace("bg = \"#101010\"", "bg = \"terminal\"");
        let t = Theme::from_toml(
            "x",
            &format!("appearance = \"light\"\n{text}"),
            Origin::Builtin,
        )
        .unwrap();
        assert_eq!(t.bg, Color::Reset);
        assert_eq!(t.appearance, Appearance::Light);
        assert_eq!(t.assumed_bg, (255, 255, 255));
        assert_eq!(t.text().bg, None);
    }

    #[test]
    fn depth_256_has_no_rgb() {
        let t = minimal().at(ColorDepth::Ansi256);
        for key in REQUIRED_ROLES.iter().chain(OPTIONAL_ROLES.iter()) {
            assert!(matches!(t.role(key), Some(Color::Indexed(_))), "{key}");
        }
        assert!(matches!(t.graph_color(0.3), Color::Indexed(_)));
        assert!(matches!(t.rgb((1, 2, 3)), Color::Indexed(_)));
    }

    #[test]
    fn depth_16_keeps_text_off_the_background() {
        let t = minimal().at(ColorDepth::Ansi16);
        assert_eq!(t.bg, Color::Black);
        for key in REQUIRED_ROLES.iter().chain(OPTIONAL_ROLES.iter()) {
            let c = t.role(key).unwrap();
            assert!(!matches!(c, Color::Rgb(..) | Color::Indexed(_)), "{key}");
            if *key != "bg" {
                assert_ne!(c, t.bg, "{key}");
            }
        }
        assert_ne!(t.selection_fg, t.selection_bg);
        assert_ne!(t.status_fg, t.status_bg);
    }

    #[test]
    fn no_color_styles_carry_modifiers_only() {
        let t = minimal().at(ColorDepth::NoColor);
        assert_eq!(t.text(), Style::new());
        assert_eq!(t.fg(t.accent), Style::new());
        assert_eq!(t.selected(), Style::new().add_modifier(Modifier::REVERSED));
        assert_eq!(t.status(), Style::new().add_modifier(Modifier::REVERSED));
        assert_eq!(t.dimmed(), Style::new().add_modifier(Modifier::DIM));
    }

    #[test]
    fn gradient_blends_and_hits_the_stops() {
        let t = minimal();
        assert_eq!(t.size_color(0.0), t.ok);
        assert_eq!(t.size_color(0.5), t.warn);
        assert_eq!(t.size_color(1.0), t.danger);
        assert_eq!(t.size_color(7.0), t.danger);
        assert_eq!(t.size_color(f64::NAN), t.ok);
        let mid = t.size_color(0.25);
        assert!(matches!(mid, Color::Rgb(..)));
        assert_ne!(mid, t.ok);
        assert_ne!(mid, t.warn);
        // Named colors step instead of blending.
        let stops = [Color::LightGreen, Color::LightYellow, Color::LightRed];
        assert_eq!(t.gradient(&stops, 0.1), Color::LightGreen);
        assert_eq!(t.gradient(&stops, 0.9), Color::LightRed);
        assert_eq!(t.gradient(&[], 0.5), Color::Reset);
    }

    #[test]
    fn overrides_accept_legacy_ramas_keys() {
        let base = minimal();
        let t = base
            .with_overrides(
                "custom",
                [
                    ("fg", "#112233"),
                    ("selected_bg", "blue"),
                    ("error", "#ff0000"),
                    ("size", "#abcdef"),
                    ("bar_high", "#010203"),
                ],
            )
            .unwrap();
        assert_eq!(t.id, "custom");
        assert_eq!(t.origin, Origin::Config);
        assert_eq!(t.fg, Color::Rgb(0x11, 0x22, 0x33));
        assert_eq!(t.selection_bg, Color::Blue);
        assert_eq!(t.danger, Color::Rgb(255, 0, 0));
        assert_eq!(t.value, Color::Rgb(0xab, 0xcd, 0xef));
        assert_eq!(t.size[2], Color::Rgb(1, 2, 3));
        assert_eq!(t.size[0], base.size[0]);
        assert_eq!(t.bg, base.bg);
        assert!(base.with_overrides("c", [("fg", "notacolor")]).is_err());
        assert!(
            base.with_overrides("c", [("background", "#000000")])
                .is_err()
        );
    }

    #[test]
    fn contrast_uses_assumed_background() {
        let t = minimal();
        assert!(t.text_contrast() > 15.0);
        assert!(t.contrast(Color::Reset, Color::Reset) > 10.0);
    }
}
