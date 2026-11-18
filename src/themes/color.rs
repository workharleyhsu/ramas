//! Color parsing, color-depth detection and conversion between depths.

use ratatui::style::Color;

/// Plain RGB triple.
pub type Rgb = (u8, u8, u8);

/// How many colors the terminal can show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorDepth {
    /// 24-bit color.
    TrueColor,
    /// The 256-color palette.
    Ansi256,
    /// The 16 basic colors.
    Ansi16,
    /// No colors at all (`NO_COLOR`): styles carry modifiers only.
    NoColor,
}

impl ColorDepth {
    /// Detects the depth from `NO_COLOR`, `COLORTERM` and `TERM`.
    pub fn detect() -> Self {
        let var = |name: &str| std::env::var(name).ok();
        Self::detect_from(
            var("NO_COLOR").as_deref(),
            var("COLORTERM").as_deref(),
            var("TERM").as_deref(),
        )
    }

    /// Same as [`ColorDepth::detect`], with the variables passed in.
    pub fn detect_from(
        no_color: Option<&str>,
        colorterm: Option<&str>,
        term: Option<&str>,
    ) -> Self {
        if no_color.is_some_and(|v| !v.is_empty()) {
            return ColorDepth::NoColor;
        }
        let term = term.unwrap_or("");
        if term == "dumb" {
            return ColorDepth::NoColor;
        }
        if colorterm.is_some_and(|v| v == "truecolor" || v == "24bit") || term.contains("direct") {
            return ColorDepth::TrueColor;
        }
        if term.contains("256") {
            return ColorDepth::Ansi256;
        }
        ColorDepth::Ansi16
    }
}

/// RGB values of the 16 basic colors (xterm defaults).
const ANSI16: [Rgb; 16] = [
    (0, 0, 0),
    (205, 0, 0),
    (0, 205, 0),
    (205, 205, 0),
    (0, 0, 238),
    (205, 0, 205),
    (0, 205, 205),
    (229, 229, 229),
    (127, 127, 127),
    (255, 0, 0),
    (0, 255, 0),
    (255, 255, 0),
    (92, 92, 255),
    (255, 0, 255),
    (0, 255, 255),
    (255, 255, 255),
];

/// Named ratatui colors in ANSI index order.
const NAMED: [Color; 16] = [
    Color::Black,
    Color::Red,
    Color::Green,
    Color::Yellow,
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
    Color::Gray,
    Color::DarkGray,
    Color::LightRed,
    Color::LightGreen,
    Color::LightYellow,
    Color::LightBlue,
    Color::LightMagenta,
    Color::LightCyan,
    Color::White,
];

const CUBE: [u8; 6] = [0, 95, 135, 175, 215, 255];

/// ANSI index (0..16) of a named color.
fn named_index(color: Color) -> Option<u8> {
    NAMED.iter().position(|c| *c == color).map(|i| i as u8)
}

/// Named color for an ANSI index (0..16).
pub fn named(index: u8) -> Color {
    NAMED[usize::from(index) % 16]
}

/// RGB value of a palette index.
fn indexed_rgb(i: u8) -> Rgb {
    match i {
        0..=15 => ANSI16[usize::from(i)],
        16..=231 => {
            let n = usize::from(i) - 16;
            (CUBE[n / 36], CUBE[n / 6 % 6], CUBE[n % 6])
        }
        _ => {
            let v = 8 + 10 * (i - 232);
            (v, v, v)
        }
    }
}

/// RGB value of a color; `None` for the terminal default.
pub fn to_rgb(color: Color) -> Option<Rgb> {
    match color {
        Color::Reset => None,
        Color::Rgb(r, g, b) => Some((r, g, b)),
        Color::Indexed(i) => Some(indexed_rgb(i)),
        other => named_index(other).map(|i| ANSI16[usize::from(i)]),
    }
}

fn dist(a: Rgb, b: Rgb) -> i32 {
    let d = |x: u8, y: u8| (i32::from(x) - i32::from(y)).pow(2);
    d(a.0, b.0) + d(a.1, b.1) + d(a.2, b.2)
}

/// Nearest index of the 256-color palette (6×6×6 cube or gray ramp).
pub fn to_ansi256(rgb: Rgb) -> u8 {
    let nearest = |v: u8| -> usize {
        let mut best = 0;
        for (i, level) in CUBE.iter().enumerate() {
            if v.abs_diff(*level) < v.abs_diff(CUBE[best]) {
                best = i;
            }
        }
        best
    };
    let (ri, gi, bi) = (nearest(rgb.0), nearest(rgb.1), nearest(rgb.2));
    let cube = (CUBE[ri], CUBE[gi], CUBE[bi]);
    let mean = (i32::from(rgb.0) + i32::from(rgb.1) + i32::from(rgb.2)) / 3;
    let gray_index = (((mean - 8).clamp(0, 230) + 5) / 10).min(23);
    let gray = (8 + 10 * gray_index) as u8;
    if dist((gray, gray, gray), rgb) < dist(cube, rgb) {
        (232 + gray_index) as u8
    } else {
        (16 + 36 * ri + 6 * gi + bi) as u8
    }
}

/// Nearest of the 16 basic colors, optionally skipping one index so a
/// foreground never lands on its own background.
pub fn to_ansi16(rgb: Rgb, exclude: Option<u8>) -> u8 {
    // A near-gray color only competes with the grays: plain distance would
    // send a very dark gray to dark red.
    let gray = rgb.0.max(rgb.1).max(rgb.2) - rgb.0.min(rgb.1).min(rgb.2) < 40;
    let mut best = 0u8;
    let mut best_dist = i32::MAX;
    for (i, candidate) in ANSI16.iter().enumerate() {
        let i = i as u8;
        if exclude == Some(i) || (gray && !matches!(i, 0 | 7 | 8 | 15)) {
            continue;
        }
        let d = dist(*candidate, rgb);
        if d < best_dist {
            best = i;
            best_dist = d;
        }
    }
    best
}

/// ANSI index a color takes at 16-color depth; `None` for the terminal default.
pub(crate) fn ansi16_index(color: Color, exclude: Option<u8>) -> Option<u8> {
    if let Some(i) = named_index(color)
        && exclude != Some(i)
    {
        return Some(i);
    }
    to_rgb(color).map(|rgb| to_ansi16(rgb, exclude))
}

/// Converts a color to the best one the given depth can show.
pub fn degrade(color: Color, depth: ColorDepth) -> Color {
    match (depth, color) {
        (ColorDepth::TrueColor | ColorDepth::NoColor, c) => c,
        (ColorDepth::Ansi256, Color::Rgb(r, g, b)) => Color::Indexed(to_ansi256((r, g, b))),
        (ColorDepth::Ansi256, c) => c,
        (ColorDepth::Ansi16, c) => ansi16_index(c, None).map_or(Color::Reset, named),
    }
}

/// Parses `"terminal"` (the terminal default), `#rrggbb`, a color name or a
/// palette index.
pub fn parse_color(text: &str) -> Result<Color, String> {
    let t = text.trim();
    if t.eq_ignore_ascii_case("terminal") || t.eq_ignore_ascii_case("default") {
        return Ok(Color::Reset);
    }
    if let Some(hex) = t.strip_prefix('#')
        && (hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()))
    {
        return Err(format!("invalid color {text:?}: expected #rrggbb"));
    }
    t.parse::<Color>().map_err(|_| {
        format!("invalid color {text:?}: expected #rrggbb, a color name, an index or \"terminal\"")
    })
}

/// Linear blend of two RGB colors, `t` in `0..=1`.
pub fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (f64::from(x) + (f64::from(y) - f64::from(x)) * t).round() as u8;
    (m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

/// WCAG relative luminance, `0..=1`.
pub fn luminance(rgb: Rgb) -> f64 {
    let channel = |v: u8| {
        let c = f64::from(v) / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(rgb.0) + 0.7152 * channel(rgb.1) + 0.0722 * channel(rgb.2)
}

/// WCAG contrast ratio between two colors, `1..=21`.
pub fn contrast_ratio(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_depth_from_environment() {
        use ColorDepth::*;
        let d = ColorDepth::detect_from;
        assert_eq!(
            d(Some("1"), Some("truecolor"), Some("xterm-256color")),
            NoColor
        );
        assert_eq!(d(Some(""), Some("truecolor"), Some("xterm")), TrueColor);
        assert_eq!(d(None, Some("24bit"), None), TrueColor);
        assert_eq!(d(None, None, Some("xterm-direct")), TrueColor);
        assert_eq!(d(None, None, Some("xterm-256color")), Ansi256);
        assert_eq!(d(None, None, Some("linux")), Ansi16);
        assert_eq!(d(None, None, None), Ansi16);
        assert_eq!(d(None, Some("truecolor"), Some("dumb")), NoColor);
    }

    #[test]
    fn maps_to_the_256_color_palette() {
        assert_eq!(to_ansi256((255, 0, 0)), 196);
        assert_eq!(to_ansi256((0, 0, 255)), 21);
        assert_eq!(to_ansi256((255, 255, 255)), 231);
        assert_eq!(to_ansi256((128, 128, 128)), 244);
        assert_eq!(to_ansi256((95, 135, 175)), 67);
    }

    #[test]
    fn maps_to_the_16_color_palette() {
        assert_eq!(to_ansi16((250, 10, 10), None), 9);
        assert_eq!(to_ansi16((5, 5, 5), None), 0);
        assert_eq!(to_ansi16((5, 5, 5), Some(0)), 8);
        assert_eq!(to_ansi16((250, 250, 250), None), 15);
    }

    #[test]
    fn degrades_by_depth() {
        let c = Color::Rgb(255, 0, 0);
        assert_eq!(degrade(c, ColorDepth::TrueColor), c);
        assert_eq!(degrade(c, ColorDepth::Ansi256), Color::Indexed(196));
        assert_eq!(degrade(c, ColorDepth::Ansi16), Color::LightRed);
        assert_eq!(degrade(Color::Reset, ColorDepth::Ansi16), Color::Reset);
        assert_eq!(
            degrade(Color::Indexed(196), ColorDepth::Ansi16),
            Color::LightRed
        );
        assert_eq!(
            degrade(Color::LightCyan, ColorDepth::Ansi256),
            Color::LightCyan
        );
    }

    #[test]
    fn parses_colors() {
        assert_eq!(parse_color("#ff8800"), Ok(Color::Rgb(255, 136, 0)));
        assert_eq!(parse_color("terminal"), Ok(Color::Reset));
        assert_eq!(parse_color("blue"), Ok(Color::Blue));
        assert_eq!(parse_color("42"), Ok(Color::Indexed(42)));
        assert!(parse_color("#ff88").is_err());
        assert!(parse_color("notacolor").is_err());
    }

    #[test]
    fn contrast_of_black_and_white_is_21() {
        assert!((contrast_ratio((0, 0, 0), (255, 255, 255)) - 21.0).abs() < 0.01);
        assert!((contrast_ratio((10, 10, 10), (10, 10, 10)) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn palette_indices_round_trip() {
        assert_eq!(to_rgb(Color::Indexed(196)), Some((255, 0, 0)));
        assert_eq!(to_rgb(Color::Indexed(244)), Some((128, 128, 128)));
        assert_eq!(to_rgb(Color::White), Some((255, 255, 255)));
        assert_eq!(to_rgb(Color::Reset), None);
    }
}
