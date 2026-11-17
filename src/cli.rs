//! Command line interface definition.

use std::path::PathBuf;

use clap::{Parser, ValueEnum};

/// Icon set used in the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IconMode {
    /// Nerd Font glyphs (needs a patched font).
    Nerd,
    /// Plain Unicode symbols that render in any modern font.
    Unicode,
    /// Pure ASCII markers.
    Ascii,
}

/// UI language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    /// English.
    #[default]
    En,
    /// Spanish.
    Es,
}

/// Interactive tree-style disk usage analyzer.
///
/// Scans PATH (default: current directory) and shows its disk usage as an
/// expandable tree with a detail panel, batch deletion, search, filters,
/// per-type analysis and JSON export/diff.
#[derive(Debug, Parser)]
#[command(name = "ramas", version, about, long_about)]
pub struct Cli {
    /// Directory to scan.
    pub path: Option<PathBuf>,

    /// Do not cross filesystem boundaries.
    #[arg(short = 'x', long)]
    pub one_file_system: bool,

    /// Start showing apparent sizes instead of disk usage.
    #[arg(long)]
    pub apparent: bool,

    /// Disable every delete action.
    #[arg(long)]
    pub read_only: bool,

    /// Scan, write the result to FILE as JSON and exit without the TUI.
    #[arg(long, value_name = "FILE")]
    pub export: Option<PathBuf>,

    /// Open a previously exported scan instead of scanning (delete disabled).
    #[arg(long, value_name = "FILE", conflicts_with = "export")]
    pub import: Option<PathBuf>,

    /// Compare the scan against an earlier export and highlight changes.
    #[arg(long, value_name = "FILE")]
    pub diff: Option<PathBuf>,

    /// Icon set (overrides the config file).
    #[arg(long, value_enum, value_name = "MODE")]
    pub icons: Option<IconMode>,

    /// Use plain ASCII markers instead of icons (same as --icons ascii).
    #[arg(long, conflicts_with = "icons")]
    pub no_icons: bool,

    /// Color theme: dark, light, high-contrast, gruvbox or custom.
    #[arg(long, value_name = "NAME")]
    pub theme: Option<String>,

    /// UI language.
    #[arg(long, value_enum, value_name = "LANG")]
    pub lang: Option<Lang>,

    /// Config file path (default: ~/.config/ramas/config.toml).
    #[arg(long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Print the man page (roff) to stdout and exit.
    #[arg(long, hide = true)]
    pub man: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_flags() {
        let cli = Cli::parse_from(["ramas", "-x", "--read-only", "/tmp"]);
        assert!(cli.one_file_system);
        assert!(cli.read_only);
        assert_eq!(cli.path, Some(PathBuf::from("/tmp")));
    }
}
