//! User configuration loaded from `~/.config/ramas/config.toml`.
//!
//! Every key is optional:
//!
//! ```toml
//! language = "es"          # en | es
//! icons = "nerd"           # nerd | unicode | ascii
//! theme = "gruvbox"        # dark | light | high-contrast | gruvbox | custom
//!
//! [categories]             # a built-in key replaces its list, others add one
//! video = ["mp4", "mkv"]
//! models = ["gguf", "safetensors"]
//!
//! [custom_theme]           # used by theme = "custom"; "#rrggbb" or color names
//! base = "dark"            # theme whose colors are the defaults
//! fg = "#e0e0e0"
//! selected_bg = "blue"
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::classify::Classifier;
use crate::cli::{IconMode, Lang};

/// Parsed configuration file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub language: Option<Lang>,
    pub icons: Option<IconMode>,
    pub theme: Option<String>,
    #[serde(default)]
    pub categories: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub custom_theme: BTreeMap<String, String>,
}

impl Config {
    /// Default location: `$XDG_CONFIG_HOME/ramas/config.toml`.
    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("ramas").join("config.toml"))
    }

    /// Loads `path`, or the default location when `None`. A missing file at
    /// the default location yields the default config; an explicitly given
    /// path must exist.
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let (path, explicit) = match path {
            Some(p) => (p.to_path_buf(), true),
            None => match Self::default_path() {
                Some(p) => (p, false),
                None => return Ok(Config::default()),
            },
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                Self::parse(&text).with_context(|| format!("invalid config {}", path.display()))
            }
            Err(e) if !explicit && e.kind() == std::io::ErrorKind::NotFound => {
                Ok(Config::default())
            }
            Err(e) => Err(e).with_context(|| format!("cannot read config {}", path.display())),
        }
    }

    /// Parses TOML text.
    pub fn parse(text: &str) -> Result<Self> {
        Ok(toml::from_str(text)?)
    }

    /// Classifier with the configured category overrides.
    pub fn classifier(&self) -> Classifier {
        let overrides: Vec<(String, Vec<String>)> = self
            .categories
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Classifier::new(&overrides)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_example() {
        let c = Config::parse(
            r##"
            language = "es"
            icons = "nerd"
            theme = "custom"
            [categories]
            video = ["mp4"]
            models = ["gguf"]
            [custom_theme]
            fg = "#ffffff"
            "##,
        )
        .unwrap();
        assert_eq!(c.language, Some(Lang::Es));
        assert_eq!(c.icons, Some(IconMode::Nerd));
        assert_eq!(c.theme.as_deref(), Some("custom"));
        assert_eq!(c.custom_theme["fg"], "#ffffff");
        let cls = c.classifier();
        assert_eq!(cls.name(cls.of_name(b"x.gguf")), "models");
        assert_eq!(cls.name(cls.of_name(b"x.mkv")), "other");
    }

    #[test]
    fn empty_and_unknown_keys() {
        assert_eq!(Config::parse("").unwrap(), Config::default());
        assert!(Config::parse("colour = 1").is_err(), "typos are reported");
        assert!(Config::parse("icons = \"emoji\"").is_err());
    }

    #[test]
    fn load_from_file_and_missing_explicit_path() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("config.toml");
        std::fs::write(&p, "theme = \"light\"\n").unwrap();
        assert_eq!(
            Config::load(Some(&p)).unwrap().theme.as_deref(),
            Some("light")
        );
        assert!(Config::load(Some(&tmp.path().join("nope.toml"))).is_err());
    }
}
