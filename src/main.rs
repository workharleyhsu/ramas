use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use ramas::cli::{Cli, IconMode, Lang};
use ramas::config::Config;
use ramas::export;
use ramas::scan::ScanOptions;
use ramas::ui::theme::{self, Catalog, ColorDepth, Theme};
use ramas::ui::{self, app::Settings, format, icons::Icons, job::ScanJob};

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.man {
        let mut out = std::io::stdout();
        clap_mangen::Man::new(Cli::command()).render(&mut out)?;
        return Ok(());
    }
    let config = Config::load(cli.config.as_deref())?;
    let mut settings = settings(&cli, &config)?;
    if let Some(old) = &cli.diff {
        let imported = export::read_file(old)?;
        settings.diff = Some((imported.tree, old.clone()));
    }

    if let Some(file) = &cli.import {
        let imported = export::read_file(file)?;
        settings.imported = true;
        settings.scanned_at = imported.scanned_at;
        return ui::run_with_tree(imported.tree, settings);
    }

    let root = cli.path.clone().unwrap_or_else(|| PathBuf::from("."));
    let root =
        std::fs::canonicalize(&root).with_context(|| format!("cannot open {}", root.display()))?;
    settings.scanned_at = export::now();
    match &cli.export {
        Some(file) => export_only(&root, &settings.scan_opts, settings.scanned_at, file),
        None => ui::run(&root, settings),
    }
}

/// `--export`: scans without the TUI and writes the JSON file.
fn export_only(root: &Path, opts: &ScanOptions, at: i64, file: &Path) -> Result<()> {
    let job = ScanJob::start(root.to_path_buf(), opts.clone(), None);
    let live = std::io::stderr().is_terminal();
    let tree = loop {
        if let Some(res) = job.try_result() {
            break res?;
        }
        if live {
            let p = job.snapshot();
            eprint!("\r\x1b[2K{} entries, {}", p.entries, format::size(p.bytes));
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    if live {
        eprint!("\r\x1b[2K");
    }
    export::write_file(&tree, at, file)?;
    eprintln!(
        "{}: {} entries, {} on disk, {} unreadable, {:.1}s -> {}",
        root.display(),
        tree.live(),
        format::size(tree.root().disk),
        tree.root().errors,
        job.elapsed().as_secs_f64(),
        file.display()
    );
    Ok(())
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
        diff: None,
        scanned_at: 0,
    })
}

/// Active theme and the `T` cycle, at the terminal's color depth
/// (`NO_COLOR` included).
fn themes(cli: &Cli, config: &Config) -> Result<(Theme, Vec<Theme>)> {
    let name = cli.theme.as_deref().or(config.theme.as_deref());
    theme::select(
        Catalog::builtin(),
        name,
        &config.custom_theme,
        ColorDepth::detect(),
    )
    .map_err(anyhow::Error::msg)
}
