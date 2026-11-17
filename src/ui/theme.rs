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
    fn no_color_styles_are_plain() {
        let t = Theme::dark().without_color();
        assert_eq!(t.fg(t.size), Style::new());
        assert_eq!(t.text(), Style::new());
    }
}
