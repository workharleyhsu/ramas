//! Combinable node filters: minimum size, extensions and age.

use crate::tree::{Kind, NodeId, Tree, flags};

/// Active filter criteria. Empty criteria match everything.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filter {
    /// Minimum size under the current metric.
    pub min_size: Option<u64>,
    /// Lowercase extensions without dot; empty = any.
    pub exts: Vec<String>,
    /// Only files not modified in this many days.
    pub older_than_days: Option<u64>,
    /// Hide non-matching nodes instead of dimming them.
    pub hide: bool,
}

impl Filter {
    /// Whether no criterion is set.
    pub fn is_empty(&self) -> bool {
        self.min_size.is_none() && self.exts.is_empty() && self.older_than_days.is_none()
    }

    /// Parses a comma/space separated extension list (`.mp4, MKV` → `mp4 mkv`).
    pub fn parse_exts(s: &str) -> Vec<String> {
        s.split([',', ' '])
            .map(|e| e.trim().trim_start_matches('.').to_ascii_lowercase())
            .filter(|e| !e.is_empty())
            .collect()
    }

    fn file_matches(&self, tree: &Tree, id: NodeId, apparent: bool, cutoff: Option<i64>) -> bool {
        let n = tree.node(id);
        if self.min_size.is_some_and(|m| n.size(apparent) < m) {
            return false;
        }
        if cutoff.is_some_and(|c| n.mtime > c) {
            return false;
        }
        if !self.exts.is_empty() {
            let name = tree.name(id).to_string_lossy();
            let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
            if !ext.is_some_and(|e| self.exts.contains(&e)) {
                return false;
            }
        }
        true
    }

    /// Computes the match flag of every node.
    ///
    /// A file matches when it meets every criterion. A directory matches
    /// when some descendant file matches, or, if only the size criterion is
    /// set, when its own total is large enough. The root always matches.
    pub fn evaluate(&self, tree: &Tree, apparent: bool, now: i64) -> Vec<bool> {
        let mut out = vec![false; tree.len()];
        if self.is_empty() {
            out.fill(true);
            return out;
        }
        let cutoff = self
            .older_than_days
            .map(|d| now - i64::try_from(d.saturating_mul(86_400)).unwrap_or(i64::MAX));
        let size_only = self.exts.is_empty() && self.older_than_days.is_none();
        // Children always have larger indices than parents: walk backwards so
        // a match can be propagated to the parent in the same pass.
        for i in (0..tree.len()).rev() {
            let id = NodeId(i as u32);
            let n = tree.node(id);
            if n.has(flags::REMOVED) {
                continue;
            }
            let own = match n.kind {
                Kind::Dir => size_only && self.min_size.is_some_and(|m| n.size(apparent) >= m),
                _ => self.file_matches(tree, id, apparent, cutoff),
            };
            out[i] |= own;
            if out[i]
                && let Some(p) = tree.parent(id)
            {
                out[p.index()] = true;
            }
        }
        out[0] = true;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::test_util::{dir, file};
    use std::ffi::OsStr;
    use std::path::Path;

    const DAY: i64 = 86_400;
    const NOW: i64 = 1000 * DAY;

    fn fixture() -> Tree {
        let mut t = Tree::new("/r", dir());
        let v = t.add_child(NodeId::ROOT, OsStr::new("videos"), &dir());
        let mut old = file(5_000_000);
        old.mtime = NOW - 400 * DAY;
        t.add_child(v, OsStr::new("old.mp4"), &old);
        let mut new = file(2_000_000);
        new.mtime = NOW - DAY;
        t.add_child(v, OsStr::new("new.MKV"), &new);
        let d = t.add_child(NodeId::ROOT, OsStr::new("docs"), &dir());
        let deep = t.add_child(d, OsStr::new("deep"), &dir());
        let mut pdf = file(300);
        pdf.mtime = NOW - 500 * DAY;
        t.add_child(deep, OsStr::new("a.pdf"), &pdf);
        t.add_child(NodeId::ROOT, OsStr::new("tiny"), &file(1));
        t.aggregate();
        t
    }

    fn matched(t: &Tree, f: &Filter) -> Vec<String> {
        let m = f.evaluate(t, false, NOW);
        (1..t.len())
            .filter(|&i| m[i])
            .map(|i| t.rel_path(NodeId(i as u32)).display().to_string())
            .collect()
    }

    #[test]
    fn empty_filter_matches_all() {
        let t = fixture();
        assert!(
            Filter::default()
                .evaluate(&t, false, NOW)
                .iter()
                .all(|&b| b)
        );
    }

    #[test]
    fn min_size_keeps_big_files_and_dirs() {
        let t = fixture();
        let f = Filter {
            min_size: Some(1_000_000),
            ..Filter::default()
        };
        assert_eq!(
            matched(&t, &f),
            ["videos", "videos/old.mp4", "videos/new.MKV"]
        );
    }

    #[test]
    fn extension_filter_marks_ancestors() {
        let t = fixture();
        let f = Filter {
            exts: Filter::parse_exts(".PDF"),
            ..Filter::default()
        };
        assert_eq!(matched(&t, &f), ["docs", "docs/deep", "docs/deep/a.pdf"]);
    }

    #[test]
    fn filters_combine() {
        let t = fixture();
        let f = Filter {
            exts: Filter::parse_exts("mp4, mkv"),
            older_than_days: Some(30),
            ..Filter::default()
        };
        assert_eq!(matched(&t, &f), ["videos", "videos/old.mp4"]);
        let f = Filter {
            min_size: Some(100),
            older_than_days: Some(450),
            ..Filter::default()
        };
        assert_eq!(matched(&t, &f), ["docs", "docs/deep", "docs/deep/a.pdf"]);
        assert!(t.find(Path::new("tiny")).is_some());
    }

    #[test]
    fn parse_exts_normalizes() {
        assert_eq!(
            Filter::parse_exts(" .Mp4,mkv  JPG,,"),
            ["mp4", "mkv", "jpg"]
        );
    }
}
