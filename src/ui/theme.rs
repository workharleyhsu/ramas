//! Color themes.

use ratatui::style::{Color, Modifier, Style};

/// Colors used by every widget. With `color == false` (NO_COLOR) styles
/// carry modifiers only.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub name: String,
    pub color: bool,
    pub fg: Color,
    pub bg: Color,
    pub dim: Color,
    pub accent: Color,
    pub border: Color,
    pub connector: Color,
    pub dir: Color,
    pub selected_fg: Color,
    pub selected_bg: Color,
    pub size: Color,
    pub marked: Color,
    pub error: Color,
    pub warn: Color,
    pub bar_low: Color,
    pub bar_mid: Color,
    pub bar_high: Color,
    pub status_fg: Color,
    pub status_bg: Color,
    pub grew: Color,
    pub shrank: Color,
    pub new: Color,
    pub matched: Color,
}

impl Theme {
    /// Default dark theme.
    pub fn dark() -> Self {
        Theme {
            name: "dark".into(),
            color: true,
            fg: Color::Rgb(0xd8, 0xde, 0xe9),
            bg: Color::Reset,
            dim: Color::Rgb(0x6b, 0x72, 0x80),
            accent: Color::Rgb(0x88, 0xc0, 0xd0),
            border: Color::Rgb(0x4c, 0x56, 0x6a),
            connector: Color::Rgb(0x4c, 0x56, 0x6a),
            dir: Color::Rgb(0x81, 0xa1, 0xc1),
            selected_fg: Color::Rgb(0xec, 0xef, 0xf4),
            selected_bg: Color::Rgb(0x3b, 0x42, 0x52),
            size: Color::Rgb(0xeb, 0xcb, 0x8b),
            marked: Color::Rgb(0xb4, 0x8e, 0xad),
            error: Color::Rgb(0xbf, 0x61, 0x6a),
            warn: Color::Rgb(0xd0, 0x87, 0x70),
            bar_low: Color::Rgb(0xa3, 0xbe, 0x8c),
            bar_mid: Color::Rgb(0xeb, 0xcb, 0x8b),
            bar_high: Color::Rgb(0xbf, 0x61, 0x6a),
            status_fg: Color::Rgb(0xe5, 0xe9, 0xf0),
            status_bg: Color::Rgb(0x2e, 0x34, 0x40),
            grew: Color::Rgb(0xbf, 0x61, 0x6a),
            shrank: Color::Rgb(0xa3, 0xbe, 0x8c),
            new: Color::Rgb(0x88, 0xc0, 0xd0),
            matched: Color::Rgb(0xeb, 0xcb, 0x8b),
        }
    }

    /// Light theme with an explicit light background.
    pub fn light() -> Self {
        Theme {
            name: "light".into(),
            color: true,
            fg: Color::Rgb(0x2e, 0x34, 0x40),
            bg: Color::Rgb(0xfa, 0xfa, 0xf7),
            dim: Color::Rgb(0x8a, 0x8f, 0x98),
            accent: Color::Rgb(0x1f, 0x6f, 0xb2),
            border: Color::Rgb(0xb8, 0xbe, 0xc8),
            connector: Color::Rgb(0xa0, 0xa6, 0xb0),
            dir: Color::Rgb(0x23, 0x5a, 0x97),
            selected_fg: Color::Rgb(0x10, 0x10, 0x10),
            selected_bg: Color::Rgb(0xd6, 0xe4, 0xf5),
            size: Color::Rgb(0x9a, 0x5b, 0x00),
            marked: Color::Rgb(0x8e, 0x3c, 0xa8),
            error: Color::Rgb(0xc0, 0x29, 0x2b),
            warn: Color::Rgb(0xb3, 0x5c, 0x00),
            bar_low: Color::Rgb(0x4c, 0x9a, 0x2a),
            bar_mid: Color::Rgb(0xd4, 0x9b, 0x00),
            bar_high: Color::Rgb(0xd0, 0x3a, 0x2f),
            status_fg: Color::Rgb(0xfa, 0xfa, 0xf7),
            status_bg: Color::Rgb(0x3b, 0x4a, 0x5e),
            grew: Color::Rgb(0xc0, 0x29, 0x2b),
            shrank: Color::Rgb(0x2f, 0x85, 0x2f),
            new: Color::Rgb(0x1f, 0x6f, 0xb2),
            matched: Color::Rgb(0xc2, 0x4f, 0x00),
        }
    }

    /// Maximum contrast using the 16 basic terminal colors.
    pub fn high_contrast() -> Self {
        Theme {
            name: "high-contrast".into(),
            color: true,
            fg: Color::White,
            bg: Color::Black,
            dim: Color::Gray,
            accent: Color::LightCyan,
            border: Color::White,
            connector: Color::Gray,
            dir: Color::LightCyan,
            selected_fg: Color::Black,
            selected_bg: Color::LightYellow,
            size: Color::LightYellow,
            marked: Color::LightMagenta,
            error: Color::LightRed,
            warn: Color::LightYellow,
            bar_low: Color::LightGreen,
            bar_mid: Color::LightYellow,
            bar_high: Color::LightRed,
            status_fg: Color::Black,
            status_bg: Color::White,
            grew: Color::LightRed,
            shrank: Color::LightGreen,
            new: Color::LightCyan,
            matched: Color::LightYellow,
        }
    }

    /// Warm retro palette.
    pub fn gruvbox() -> Self {
        Theme {
            name: "gruvbox".into(),
            color: true,
            fg: Color::Rgb(0xeb, 0xdb, 0xb2),
            bg: Color::Rgb(0x28, 0x28, 0x28),
            dim: Color::Rgb(0x92, 0x83, 0x74),
            accent: Color::Rgb(0xfa, 0xbd, 0x2f),
            border: Color::Rgb(0x50, 0x49, 0x45),
            connector: Color::Rgb(0x66, 0x5c, 0x54),
            dir: Color::Rgb(0x83, 0xa5, 0x98),
            selected_fg: Color::Rgb(0xfb, 0xf1, 0xc7),
            selected_bg: Color::Rgb(0x50, 0x49, 0x45),
            size: Color::Rgb(0xfe, 0x80, 0x19),
            marked: Color::Rgb(0xd3, 0x86, 0x9b),
            error: Color::Rgb(0xfb, 0x49, 0x34),
            warn: Color::Rgb(0xfe, 0x80, 0x19),
            bar_low: Color::Rgb(0xb8, 0xbb, 0x26),
            bar_mid: Color::Rgb(0xfa, 0xbd, 0x2f),
            bar_high: Color::Rgb(0xfb, 0x49, 0x34),
            status_fg: Color::Rgb(0x28, 0x28, 0x28),
            status_bg: Color::Rgb(0xa8, 0x99, 0x84),
            grew: Color::Rgb(0xfb, 0x49, 0x34),
            shrank: Color::Rgb(0xb8, 0xbb, 0x26),
            new: Color::Rgb(0x83, 0xa5, 0x98),
            matched: Color::Rgb(0xfa, 0xbd, 0x2f),
        }
    }

    /// Names of the built-in themes, in `T` cycle order.
    pub const BUILTIN: [&'static str; 4] = ["dark", "light", "high-contrast", "gruvbox"];

    /// Built-in theme by name.
    pub fn builtin(name: &str) -> Option<Self> {
        match name {
            "dark" => Some(Self::dark()),
            "light" => Some(Self::light()),
            "high-contrast" => Some(Self::high_contrast()),
            "gruvbox" => Some(Self::gruvbox()),
            _ => None,
        }
    }

    /// Custom theme: `base` key picks the starting theme (default dark), every
    /// other key overrides one color (`#rrggbb`, a color name or an index).
    pub fn custom(spec: &std::collections::BTreeMap<String, String>) -> Result<Self, String> {
        let base = spec.get("base").map_or("dark", String::as_str);
        let mut t = Self::builtin(base).ok_or_else(|| format!("unknown base theme {base:?}"))?;
        t.name = "custom".into();
        for (key, value) in spec {
            if key == "base" {
                continue;
            }
            let color: Color = value
                .parse()
                .map_err(|_| format!("invalid color {value:?} for {key:?}"))?;
            let slot = match key.as_str() {
                "fg" => &mut t.fg,
                "bg" => &mut t.bg,
                "dim" => &mut t.dim,
                "accent" => &mut t.accent,
                "border" => &mut t.border,
                "connector" => &mut t.connector,
                "dir" => &mut t.dir,
                "selected_fg" => &mut t.selected_fg,
                "selected_bg" => &mut t.selected_bg,
                "size" => &mut t.size,
                "marked" => &mut t.marked,
                "error" => &mut t.error,
                "warn" => &mut t.warn,
                "bar_low" => &mut t.bar_low,
                "bar_mid" => &mut t.bar_mid,
                "bar_high" => &mut t.bar_high,
                "status_fg" => &mut t.status_fg,
                "status_bg" => &mut t.status_bg,
                "grew" => &mut t.grew,
                "shrank" => &mut t.shrank,
                "new" => &mut t.new,
                "matched" => &mut t.matched,
                other => return Err(format!("unknown theme key {other:?}")),
            };
            *slot = color;
        }
        Ok(t)
    }

    /// Strips colors (for `NO_COLOR`), keeping the name.
    pub fn without_color(mut self) -> Self {
        self.color = false;
        self
    }

    /// Foreground style, or plain when colors are off.
    pub fn fg(&self, c: Color) -> Style {
        if self.color {
            Style::new().fg(c)
        } else {
            Style::new()
        }
    }

    /// Base text style.
    pub fn text(&self) -> Style {
        if self.color {
            Style::new().fg(self.fg).bg(self.bg)
        } else {
            Style::new()
        }
    }

    /// Style of the selected row.
    pub fn selected(&self) -> Style {
        if self.color {
            Style::new()
                .fg(self.selected_fg)
                .bg(self.selected_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::new().add_modifier(Modifier::REVERSED)
        }
    }

    /// Style of the status bar.
    pub fn status(&self) -> Style {
        if self.color {
            Style::new().fg(self.status_fg).bg(self.status_bg)
        } else {
            Style::new().add_modifier(Modifier::REVERSED)
        }
    }

    /// Dimmed style (filtered-out rows, secondary text).
    pub fn dimmed(&self) -> Style {
        if self.color {
            Style::new().fg(self.dim)
        } else {
            Style::new().add_modifier(Modifier::DIM)
        }
    }

    /// Bar color for a weight in `0..=1`: low, mid, high blended.
    pub fn gradient(&self, f: f64) -> Color {
        let f = f.clamp(0.0, 1.0);
        let (a, b, t) = if f < 0.5 {
            (self.bar_low, self.bar_mid, f * 2.0)
        } else {
            (self.bar_mid, self.bar_high, (f - 0.5) * 2.0)
        };
        match (a, b) {
            (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
                let mix = |x: u8, y: u8| (f64::from(x) + (f64::from(y) - f64::from(x)) * t) as u8;
                Color::Rgb(mix(r1, r2), mix(g1, g2), mix(b1, b2))
            }
            _ => {
                if t < 0.5 {
                    a
                } else {
                    b
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_endpoints() {
        let t = Theme::dark();
        assert_eq!(t.gradient(0.0), t.bar_low);
        assert_eq!(t.gradient(0.5), t.bar_mid);
        assert_eq!(t.gradient(1.0), t.bar_high);
    }

    #[test]
    fn builtins_exist_and_differ() {
        let themes: Vec<Theme> = Theme::BUILTIN
            .iter()
            .map(|n| Theme::builtin(n).unwrap())
            .collect();
        for (i, a) in themes.iter().enumerate() {
            assert_eq!(a.name, Theme::BUILTIN[i]);
            for b in &themes[i + 1..] {
                assert_ne!(a, b);
            }
        }
        assert!(Theme::builtin("nope").is_none());
    }

    #[test]
    fn custom_theme_overrides_colors() {
        let mut spec = std::collections::BTreeMap::new();
        spec.insert("base".to_string(), "light".to_string());
        spec.insert("fg".to_string(), "#112233".to_string());
        spec.insert("selected_bg".to_string(), "blue".to_string());
        let t = Theme::custom(&spec).unwrap();
        assert_eq!(t.name, "custom");
        assert_eq!(t.fg, Color::Rgb(0x11, 0x22, 0x33));
        assert_eq!(t.selected_bg, Color::Blue);
        assert_eq!(t.bg, Theme::light().bg);
        spec.insert("fg".to_string(), "notacolor".to_string());
        assert!(Theme::custom(&spec).is_err());
        let mut bad = std::collections::BTreeMap::new();
        bad.insert("background".to_string(), "#000000".to_string());
        assert!(Theme::custom(&bad).is_err());
    }

    #[test]
    fn named_colors_gradient_steps() {
        let t = Theme::high_contrast();
        assert_eq!(t.gradient(0.1), t.bar_low);
        assert_eq!(t.gradient(0.9), t.bar_high);
    }

    #[test]
    fn no_color_styles_are_plain() {
        let t = Theme::dark().without_color();
        assert_eq!(t.fg(t.size), Style::new());
        assert_eq!(t.text(), Style::new());
    }
}
