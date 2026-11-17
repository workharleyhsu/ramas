//! Node icons: Nerd Font glyphs, Unicode symbols or ASCII markers.

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

    /// Mark indicator shown before marked rows.
    pub fn mark(&self) -> &'static str {
        match self.mode {
            IconMode::Nerd => "\u{f00c}",
            IconMode::Unicode => "✔",
            IconMode::Ascii => "*",
        }
    }
}
