//! Node icons: Nerd Font glyphs, Unicode symbols or ASCII markers.

use ratatui::symbols::border;

use crate::cli::IconMode;
use crate::tree::Kind;

/// Icon set chosen at startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Icons {
    pub mode: IconMode,
}

impl Icons {
    /// Icon for a node. `open` tells whether a directory is expanded.
    pub fn node(&self, kind: Kind, open: bool, error: bool) -> &'static str {
        if error {
            return match self.mode {
                IconMode::Nerd => "\u{f071}",
                IconMode::Unicode => "⚠",
                IconMode::Ascii => "!",
            };
        }
        match (self.mode, kind, open) {
            (IconMode::Nerd, Kind::Dir, false) => "\u{f07b}",
            (IconMode::Nerd, Kind::Dir, true) => "\u{f07c}",
            (IconMode::Nerd, Kind::File, _) => "\u{f15b}",
            (IconMode::Nerd, Kind::Symlink, _) => "\u{f0c1}",
            (IconMode::Nerd, Kind::Other, _) => "\u{f128}",
            (IconMode::Unicode, Kind::Dir, false) => "▸",
            (IconMode::Unicode, Kind::Dir, true) => "▾",
            (IconMode::Unicode, Kind::File, _) => "·",
            (IconMode::Unicode, Kind::Symlink, _) => "↪",
            (IconMode::Unicode, Kind::Other, _) => "◦",
            (IconMode::Ascii, Kind::Dir, false) => "+",
            (IconMode::Ascii, Kind::Dir, true) => "-",
            (IconMode::Ascii, Kind::File, _) => " ",
            (IconMode::Ascii, Kind::Symlink, _) => "@",
            (IconMode::Ascii, Kind::Other, _) => "?",
        }
    }

    /// Tree connectors: (branch, last branch, vertical, blank), 4 columns each.
    pub fn connectors(&self) -> [&'static str; 4] {
        match self.mode {
            IconMode::Ascii => ["|-- ", "`-- ", "|   ", "    "],
            _ => ["├── ", "└── ", "│   ", "    "],
        }
    }

    /// Box border characters.
    pub fn border(&self) -> border::Set<'static> {
        match self.mode {
            IconMode::Ascii => border::Set {
                top_left: "+",
                top_right: "+",
                bottom_left: "+",
                bottom_right: "+",
                vertical_left: "|",
                vertical_right: "|",
                horizontal_top: "-",
                horizontal_bottom: "-",
            },
            _ => border::ROUNDED,
        }
    }

    /// Spinner frames of the progress screen.
    pub fn spinner(&self) -> &'static [&'static str] {
        match self.mode {
            IconMode::Ascii => &["|", "/", "-", "\\"],
            _ => &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"],
        }
    }

    /// Proportional bar of `width` columns (eighth-block resolution, or
    /// `#` in ASCII mode).
    pub fn bar(&self, frac: f64, width: usize) -> String {
        const EIGHTHS: [&str; 8] = ["", "▏", "▎", "▍", "▌", "▋", "▊", "▉"];
        if self.mode == IconMode::Ascii {
            let full = ((frac * width as f64).round() as usize).min(width);
            return format!("{}{}", "#".repeat(full), " ".repeat(width - full));
        }
        let eighths = (frac * width as f64 * 8.0).round() as usize;
        let full = (eighths / 8).min(width);
        let mut s = "█".repeat(full);
        if full < width {
            s.push_str(EIGHTHS[eighths % 8]);
        }
        let used = s.chars().count();
        s.push_str(&" ".repeat(width - used));
        s
    }

    /// Mark indicator shown before marked rows.
    pub fn mark(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "\u{f00c}",
            IconMode::Unicode => "✔",
            IconMode::Ascii => "*",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNICODE: Icons = Icons {
        mode: IconMode::Unicode,
    };
    const ASCII: Icons = Icons {
        mode: IconMode::Ascii,
    };

    #[test]
    fn bars_scale() {
        assert_eq!(UNICODE.bar(0.0, 4), "    ");
        assert_eq!(UNICODE.bar(1.0, 4), "████");
        assert_eq!(UNICODE.bar(0.5, 4), "██  ");
        assert_eq!(UNICODE.bar(0.5 + 1.0 / 16.0, 4).chars().count(), 4);
        assert_eq!(ASCII.bar(0.5, 4), "##  ");
    }

    #[test]
    fn ascii_mode_is_pure_ascii() {
        for kind in [Kind::File, Kind::Dir, Kind::Symlink, Kind::Other] {
            for open in [false, true] {
                assert!(ASCII.node(kind, open, false).is_ascii());
            }
        }
        assert!(ASCII.connectors().iter().all(|c| c.is_ascii()));
        assert!(ASCII.spinner().iter().all(|c| c.is_ascii()));
        assert!(ASCII.mark().is_ascii());
        assert!(ASCII.border().top_left.is_ascii());
    }
}
