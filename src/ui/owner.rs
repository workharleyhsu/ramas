//! Owner of a path, resolved to a user name from `/etc/passwd`.

use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::sync::OnceLock;

/// `name (uid)` of the owner of `path` (not following symlinks), or `None`
/// when the path cannot be stat-ed.
pub fn of_path(path: &Path) -> Option<String> {
    let uid = std::fs::symlink_metadata(path).ok()?.uid();
    Some(describe(uid))
}

/// `root (0)`, `harley (1000)` or `uid 10059` for ids without a name
/// (for example Android app users inside a container).
pub fn describe(uid: u32) -> String {
    match users().get(&uid) {
        Some(name) => format!("{name} ({uid})"),
        None => format!("uid {uid}"),
    }
}

fn users() -> &'static HashMap<u32, String> {
    static USERS: OnceLock<HashMap<u32, String>> = OnceLock::new();
    USERS.get_or_init(|| {
        std::fs::read_to_string("/etc/passwd")
            .map(|text| parse_passwd(&text))
            .unwrap_or_default()
    })
}

/// Maps uid to user name from `passwd` text.
pub fn parse_passwd(text: &str) -> HashMap<u32, String> {
    text.lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let mut f = l.split(':');
            let name = f.next()?;
            let uid = f.nth(1)?.parse().ok()?;
            Some((uid, name.to_owned()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_passwd_lines() {
        let m = parse_passwd(
            "root:x:0:0:root:/root:/bin/bash\n# c\nana:x:1000:1000::/home/ana:/bin/fish\nbad\n",
        );
        assert_eq!(m.get(&0).map(String::as_str), Some("root"));
        assert_eq!(m.get(&1000).map(String::as_str), Some("ana"));
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn describes_root_and_unknown_ids() {
        assert_eq!(describe(0), "root (0)");
        assert_eq!(describe(4_000_000_000), "uid 4000000000");
        assert!(of_path(Path::new("/")).is_some_and(|o| o.starts_with("root")));
    }
}
