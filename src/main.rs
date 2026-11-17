use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use ramas::cli::{Cli, IconMode, Lang};
use ramas::config::Config;
use ramas::scan::ScanOptions;
use ramas::ui::{self, app::Settings, icons::Icons, theme::Theme};

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.man {
        let mut out = std::io::stdout();
        clap_mangen::Man::new(Cli::command()).render(&mut out)?;
        return Ok(());
    }
    let root = cli.path.clone().unwrap_or_else(|| PathBuf::from("."));
    let root =
        std::fs::canonicalize(&root).with_context(|| format!("cannot open {}", root.display()))?;
    let config = Config::load(cli.config.as_deref())?;
    let settings = settings(&cli, &config);
    ui::run(&root, settings)
}

fn settings(cli: &Cli, config: &Config) -> Settings {
    let icons = if cli.no_icons {
        IconMode::Ascii
    } else {
        cli.icons.unwrap_or(IconMode::Unicode)
    };
    let mut theme = Theme::dark();
    if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
        theme = theme.without_color();
    }
    Settings {
        apparent: cli.apparent,
        read_only: cli.read_only,
        imported: false,
        strings: ramas::i18n::strings(cli.lang.unwrap_or(Lang::En)),
        theme,
        icons: Icons { mode: icons },
        scan_opts: ScanOptions {
            one_file_system: cli.one_file_system,
        },
        classifier: config.classifier(),
    }
}
