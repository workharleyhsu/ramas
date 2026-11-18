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

const AFTER_HELP: &str = "\
KEYS:
  Up/Down, k/j        move              Right/l, Left/h   expand / collapse
  Enter               toggle            PgUp PgDn Home End  page / first / last
  Space               mark / unmark     d                 delete marked (confirm)
  u                   clear marks       r                 rescan selected
  / n N               search, next/prev f                 filters
  t                   type analysis     s                 cycle sort
  a                   apparent/disk     p                 toggle side panel
  Shift+T             choose theme      e                 export JSON
  ?                   help              q                 quit
  Mouse: click selects, double click expands, wheel scrolls.

DELETING:
  Marked items go to the trash or are removed permanently; the dialog starts
  on Cancel and permanent deletion asks you to type a confirmation word.
  Paths outside the scanned root, /, $HOME (and its parents), the scan root
  and entries changed since the scan are always refused. Symlinks are
  removed, never followed. --read-only and imported scans disable deletion.

CONFIGURATION (~/.config/ramas/config.toml, all keys optional):
  language = \"es\"              # en | es
  icons = \"nerd\"               # nerd | unicode | ascii
  theme = \"gruvbox-dark\"       # any theme id (Shift+T browses them), or custom
  [categories]                  # replace a built-in list or add a category
  models = [\"gguf\", \"safetensors\"]
  [custom_theme]                # base = \"dark\", then fg, bg, accent, ... = \"#rrggbb\"
  base = \"dark\"
  accent = \"#ff8800\"
  NO_COLOR disables colors. Your own themes: ~/.config/ramas/themes/*.toml

EXAMPLES:
  ramas                         scan the current directory
  ramas -x /                    whole root filesystem, staying on it
  ramas --export home.json ~    scan without the TUI and save the result
  ramas --import home.json      browse a saved scan (deletion disabled)
  ramas --diff home.json ~      highlight what grew, shrank or is new
";

/// Interactive tree-style disk usage analyzer.
///
/// Scans PATH (default: current directory) and shows its disk usage as an
/// expandable tree with a detail panel, batch deletion, search, filters,
/// per-type analysis and JSON export/diff.
#[derive(Debug, Parser)]
#[command(
    name = "ramas",
    version,
    about,
    long_about,
    after_long_help = AFTER_HELP,
    after_help = "Run `ramas --help` for keys, configuration and examples."
)]
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

    /// Color theme id, e.g. ramas, gruvbox-dark, high-contrast-dark or custom (Shift+T browses them).
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
    fn man_page_in_repo_is_up_to_date() {
        let mut out = Vec::new();
        clap_mangen::Man::new(Cli::command())
            .render(&mut out)
            .unwrap();
        let committed = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/man/ramas.1"))
            .expect("man/ramas.1 missing: run `cargo run -- --man > man/ramas.1`");
        assert!(
            committed == out,
            "man/ramas.1 is stale: run `cargo run -- --man > man/ramas.1`"
        );
    }

    #[test]
    fn parses_flags() {
        let cli = Cli::parse_from(["ramas", "-x", "--read-only", "/tmp"]);
        assert!(cli.one_file_system);
        assert!(cli.read_only);
        assert_eq!(cli.path, Some(PathBuf::from("/tmp")));
    }
}
