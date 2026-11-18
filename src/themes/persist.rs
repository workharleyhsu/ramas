//! Saving the chosen theme into an app's own TOML config file, keeping the
//! user's comments and layout.

use std::path::{Path, PathBuf};

use toml_edit::{DocumentMut, Item, Table, value};

/// Writes `id` under `key` (a path of table names ending in the key, e.g.
/// `["theme", "name"]`) in the TOML file at `path`.
///
/// The file and its directory are created when missing. Everything else in
/// the file is left as it was. A file that is not valid TOML is not touched.
/// The write is atomic: a temporary file in the same directory, then a
/// rename. A symlinked config keeps being a symlink.
pub fn save_theme(path: &Path, key: &[&str], id: &str) -> Result<(), String> {
    let fail = |what: &str, e: &dyn std::fmt::Display| format!("{}: {what}: {e}", path.display());
    let (last, tables) = key.split_last().ok_or("empty config key")?;

    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(fail("cannot read", &e)),
    };
    let mut doc: DocumentMut = text
        .parse()
        .map_err(|e| fail("not valid TOML, theme not saved", &e))?;

    let mut table: &mut Table = doc.as_table_mut();
    for name in tables {
        let item = table
            .entry(name)
            .or_insert_with(|| Item::Table(Table::new()));
        table = item
            .as_table_mut()
            .ok_or_else(|| format!("{}: `{name}` is not a table", path.display()))?;
    }
    // Keep a trailing comment on the line when the key already exists.
    match table.get_mut(last).and_then(Item::as_value_mut) {
        Some(existing) => {
            let decor = existing.decor().clone();
            *existing = id.into();
            *existing.decor_mut() = decor;
        }
        None => table[last] = value(id),
    }

    // Follow a symlink so the link itself survives the rename.
    let target: PathBuf = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let dir = target.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).map_err(|e| fail("cannot create directory", &e))?;
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "config.toml".to_string());
    let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    std::fs::write(&tmp, doc.to_string()).map_err(|e| fail("cannot write", &e))?;
    std::fs::rename(&tmp, &target).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        fail("cannot write", &e)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_the_file_and_its_directory() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app").join("config.toml");
        save_theme(&path, &["theme"], "nord").unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "theme = \"nord\"\n"
        );
        save_theme(&path, &["theme"], "dracula").unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "theme = \"dracula\"\n"
        );
        // No temporary file is left behind.
        assert_eq!(
            std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
            1
        );
    }

    #[test]
    fn keeps_comments_and_other_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let before = "# my settings\nrefresh = 2   # seconds\n\n[theme]\n# picked by hand\ncolor = \"auto\"\nname = \"old\"  # keep me\n\n[other]\nx = 1\n";
        std::fs::write(&path, before).unwrap();
        save_theme(&path, &["theme", "name"], "tokyo-night").unwrap();
        let after = std::fs::read_to_string(&path).unwrap();
        assert_eq!(after, before.replace("\"old\"", "\"tokyo-night\""));
    }

    #[test]
    fn adds_a_missing_table() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "sort = \"ram\"\n").unwrap();
        save_theme(&path, &["theme", "name"], "nord").unwrap();
        let after = std::fs::read_to_string(&path).unwrap();
        assert_eq!(after, "sort = \"ram\"\n\n[theme]\nname = \"nord\"\n");
        let parsed: toml::Table = toml::from_str(&after).unwrap();
        assert_eq!(parsed["theme"]["name"].as_str(), Some("nord"));
    }

    #[test]
    fn a_broken_file_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "this is = = not toml").unwrap();
        let err = save_theme(&path, &["theme"], "nord").unwrap_err();
        assert!(err.contains("not valid TOML"), "{err}");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "this is = = not toml"
        );
        // A key that is not a table cannot hold the theme name.
        std::fs::write(&path, "theme = 3\n").unwrap();
        let err = save_theme(&path, &["theme", "name"], "nord").unwrap_err();
        assert!(err.contains("`theme` is not a table"), "{err}");
        assert!(save_theme(&path, &[], "nord").is_err());
    }

    #[test]
    fn a_symlinked_config_stays_a_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("dotfiles.toml");
        let link = dir.path().join("config.toml");
        std::fs::write(&real, "theme = \"a\"\n").unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();
        save_theme(&link, &["theme"], "b").unwrap();
        assert!(
            std::fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(std::fs::read_to_string(&real).unwrap(), "theme = \"b\"\n");
    }

    #[test]
    fn an_unwritable_directory_is_an_error_not_a_panic() {
        let err = save_theme(Path::new("/proc/nope/config.toml"), &["theme"], "x").unwrap_err();
        assert!(err.contains("/proc/nope/config.toml"), "{err}");
    }
}
