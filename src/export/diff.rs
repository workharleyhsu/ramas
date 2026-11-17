//! Comparison of the current tree against an earlier export.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::PathBuf;

use crate::tree::{NodeId, Tree};

/// How a node changed since the earlier scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// Did not exist before.
    New,
    /// Larger by this many bytes.
    Grew(u64),
    /// Smaller by this many bytes.
    Shrank(u64),
    /// Same size.
    Same,
}

/// Result of matching the current tree against an old one by path.
#[derive(Debug, Clone)]
pub struct Diff {
    /// File the old scan came from.
    pub against: PathBuf,
    old: Tree,
    /// Old (disk, apparent) of each current node; `None` = new.
    before: Vec<Option<(u64, u64)>>,
    /// Old entries with no counterpart now (top-level of each gone subtree).
    pub removed: u64,
    /// Old disk usage of the removed entries.
    pub removed_disk: u64,
}

impl Diff {
    /// Compares `cur` with `old`; the two roots are matched to each other.
    pub fn new(cur: &Tree, old: Tree, against: PathBuf) -> Self {
        let mut d = Diff {
            against,
            old,
            before: Vec::new(),
            removed: 0,
            removed_disk: 0,
        };
        d.recompute(cur);
        d
    }

    /// Redoes the matching after the current tree changed.
    pub fn recompute(&mut self, cur: &Tree) {
        self.before = vec![None; cur.len()];
        self.removed = 0;
        self.removed_disk = 0;
        let mut stack = vec![(NodeId::ROOT, NodeId::ROOT)];
        while let Some((c, o)) = stack.pop() {
            let on = self.old.node(o);
            self.before[c.index()] = Some((on.disk, on.apparent));
            let olds: HashMap<&OsStr, NodeId> = self
                .old
                .children(o)
                .map(|k| (self.old.name(k), k))
                .collect();
            let mut matched = 0usize;
            for child in cur.children(c) {
                if let Some(&oc) = olds.get(cur.name(child)) {
                    matched += 1;
                    stack.push((child, oc));
                }
            }
            if matched < olds.len() {
                let present: std::collections::HashSet<&OsStr> =
                    cur.children(c).map(|k| cur.name(k)).collect();
                for (name, &oc) in &olds {
                    if !present.contains(name) {
                        self.removed += 1;
                        self.removed_disk += self.old.node(oc).disk;
                    }
                }
            }
        }
    }

    /// Change of a node under the chosen metric.
    pub fn change(&self, cur: &Tree, id: NodeId, apparent: bool) -> Change {
        let Some(Some((d, a))) = self.before.get(id.index()) else {
            return Change::New;
        };
        let was = if apparent { *a } else { *d };
        let now = cur.node(id).size(apparent);
        match now.cmp(&was) {
            std::cmp::Ordering::Greater => Change::Grew(now - was),
            std::cmp::Ordering::Less => Change::Shrank(was - now),
            std::cmp::Ordering::Equal => Change::Same,
        }
    }

    /// Old size of a node, when it existed.
    pub fn before(&self, id: NodeId, apparent: bool) -> Option<u64> {
        self.before
            .get(id.index())
            .copied()
            .flatten()
            .map(|(d, a)| if apparent { a } else { d })
    }

    /// Counts of new, grown and shrunk nodes in `cur`.
    pub fn counts(&self, cur: &Tree, apparent: bool) -> (u64, u64, u64) {
        let (mut new, mut grew, mut shrank) = (0, 0, 0);
        for id in std::iter::once(NodeId::ROOT).chain(cur.descendants(NodeId::ROOT)) {
            match self.change(cur, id, apparent) {
                Change::New => new += 1,
                Change::Grew(_) => grew += 1,
                Change::Shrank(_) => shrank += 1,
                Change::Same => {}
            }
        }
        (new, grew, shrank)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::test_util::{dir, file};
    use std::path::Path;

    fn old() -> Tree {
        let mut t = Tree::new("/r", dir());
        let a = t.add_child(NodeId::ROOT, OsStr::new("a"), &dir());
        t.add_child(a, OsStr::new("keep"), &file(100));
        t.add_child(a, OsStr::new("grow"), &file(100));
        t.add_child(a, OsStr::new("shrink"), &file(100));
        let gone = t.add_child(NodeId::ROOT, OsStr::new("gone"), &dir());
        t.add_child(gone, OsStr::new("x"), &file(70));
        t.aggregate();
        t
    }

    fn cur() -> Tree {
        let mut t = Tree::new("/elsewhere", dir());
        let a = t.add_child(NodeId::ROOT, OsStr::new("a"), &dir());
        t.add_child(a, OsStr::new("keep"), &file(100));
        t.add_child(a, OsStr::new("grow"), &file(350));
        t.add_child(a, OsStr::new("shrink"), &file(40));
        t.add_child(a, OsStr::new("fresh"), &file(5));
        t.aggregate();
        t
    }

    #[test]
    fn detects_new_grown_shrunk_and_removed() {
        let c = cur();
        let d = Diff::new(&c, old(), "old.json".into());
        let ch = |p: &str| d.change(&c, c.find(Path::new(p)).unwrap(), false);
        assert_eq!(ch("a/keep"), Change::Same);
        assert_eq!(ch("a/grow"), Change::Grew(250));
        assert_eq!(ch("a/shrink"), Change::Shrank(60));
        assert_eq!(ch("a/fresh"), Change::New);
        assert_eq!(ch("a"), Change::Grew(195));
        assert_eq!(d.removed, 1);
        assert_eq!(d.removed_disk, 70);
        assert_eq!(d.counts(&c, false), (1, 3, 1)); // root, a, grow grew
        assert_eq!(
            d.before(c.find(Path::new("a/grow")).unwrap(), false),
            Some(100)
        );
    }

    #[test]
    fn identical_trees_have_no_changes() {
        let c = old();
        let d = Diff::new(&c, old(), "same.json".into());
        assert_eq!(d.counts(&c, false), (0, 0, 0));
        assert_eq!(d.removed, 0);
    }
}
