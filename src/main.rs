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
    let settings = settings(&cli, &config)?;
    ui::run(&root, settings)
}

fn settings(cli: &Cli, config: &Config) -> Result<Settings> {
    let icons = if cli.no_icons {
        IconMode::Ascii
    } else {
        cli.icons.or(config.icons).unwrap_or(IconMode::Unicode)
    };
    let (theme, themes) = themes(cli, config)?;
    Ok(Settings {
        apparent: cli.apparent,
        read_only: cli.read_only,
        imported: false,
        strings: ramas::i18n::strings(cli.lang.or(config.language).unwrap_or(Lang::En)),
        theme,
        themes,
        icons: Icons { mode: icons },
        scan_opts: ScanOptions {
            one_file_system: cli.one_file_system,
        },
        classifier: config.classifier(),
    })
}

/// Active theme and the `T` cycle, honoring `NO_COLOR`.
fn themes(cli: &Cli, config: &Config) -> Result<(Theme, Vec<Theme>)> {
    let mut all: Vec<Theme> = Theme::BUILTIN
        .iter()
        .filter_map(|n| Theme::builtin(n))
        .collect();
    if !config.custom_theme.is_empty() {
        all.push(Theme::custom(&config.custom_theme).map_err(anyhow::Error::msg)?);
    }
    let name = cli
        .theme
        .as_deref()
        .or(config.theme.as_deref())
        .unwrap_or("dark");
    let theme = match name {
        "custom" if config.custom_theme.is_empty() => {
            anyhow::bail!("theme \"custom\" needs a [custom_theme] table in the config")
        }
        _ => all
            .iter()
            .find(|t| t.name == name)
            .cloned()
            .with_context(|| {
                format!("unknown theme {name:?} (dark, light, high-contrast, gruvbox, custom)")
            })?,
    };
    if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
        let plain = |t: Theme| t.without_color();
        return Ok((plain(theme), all.into_iter().map(plain).collect()));
    }
    Ok((theme, all))
}
