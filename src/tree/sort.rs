//! Ordering of siblings.

use std::cmp::Ordering;

use super::{NONE, NodeId, Tree};

/// Order of children within each directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortKey {
    /// Largest first.
    #[default]
    Size,
    /// Alphabetical, case-insensitive.
    Name,
    /// Most recently modified first.
    Mtime,
    /// Most descendants first.
    Items,
}

impl SortKey {
    /// Next key in the `s` cycle.
    pub fn next(self) -> Self {
        match self {
            SortKey::Size => SortKey::Name,
            SortKey::Name => SortKey::Mtime,
            SortKey::Mtime => SortKey::Items,
            SortKey::Items => SortKey::Size,
        }
    }
}

impl Tree {
    /// Re-orders the children of every directory.
    pub fn sort(&mut self, key: SortKey, apparent: bool) {
        let mut buf = Vec::new();
        for i in 0..self.nodes.len() {
            if self.nodes[i].first_child != NONE {
                self.sort_children_with(NodeId(i as u32), key, apparent, &mut buf);
            }
        }
    }

    /// Re-orders the children of `id` and of all its descendants.
    pub fn sort_subtree(&mut self, id: NodeId, key: SortKey, apparent: bool) {
        let mut buf = Vec::new();
        let mut stack = vec![id];
        while let Some(cur) = stack.pop() {
            self.sort_children_with(cur, key, apparent, &mut buf);
            stack.extend(self.children(cur).filter(|&c| self.has_children(c)));
        }
    }

    fn sort_children_with(&mut self, id: NodeId, key: SortKey, apparent: bool, buf: &mut Vec<u32>) {
        buf.clear();
        buf.extend(self.children(id).map(|c| c.0));
        if buf.len() < 2 {
            return;
        }
        buf.sort_by(|&a, &b| self.compare(NodeId(a), NodeId(b), key, apparent));
        self.relink(id, buf);
    }

    fn compare(&self, a: NodeId, b: NodeId, key: SortKey, apparent: bool) -> Ordering {
        let (na, nb) = (self.node(a), self.node(b));
        let primary = match key {
            SortKey::Size => nb.size(apparent).cmp(&na.size(apparent)),
            SortKey::Name => Ordering::Equal,
            SortKey::Mtime => nb.mtime.cmp(&na.mtime),
            SortKey::Items => nb.items().cmp(&na.items()),
        };
        primary.then_with(|| name_cmp(self, a, b))
    }

    fn relink(&mut self, parent: NodeId, order: &[u32]) {
        let mut prev = NONE;
        for &c in order {
            let n = &mut self.nodes[c as usize];
            n.prev_sibling = prev;
            n.next_sibling = NONE;
            if prev != NONE {
                self.nodes[prev as usize].next_sibling = c;
            }
            prev = c;
        }
        let p = &mut self.nodes[parent.index()];
        p.first_child = order.first().copied().unwrap_or(NONE);
        p.last_child = prev;
    }
}

fn name_cmp(tree: &Tree, a: NodeId, b: NodeId) -> Ordering {
    use std::os::unix::ffi::OsStrExt;
    let (x, y) = (tree.name(a).as_bytes(), tree.name(b).as_bytes());
    let lower = |c: &u8| c.to_ascii_lowercase();
    x.iter()
        .map(lower)
        .cmp(y.iter().map(lower))
        .then_with(|| x.cmp(y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::test_util::*;
    use std::ffi::OsStr;

    fn names(t: &Tree, id: NodeId) -> Vec<String> {
        t.children(id)
            .map(|c| t.name(c).to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn sorts_by_each_key() {
        let mut t = sample();
        t.sort(SortKey::Size, false);
        assert_eq!(names(&t, NodeId::ROOT), ["a", "b", "c"]);
        let a = t.find("a".as_ref()).unwrap();
        assert_eq!(names(&t, a), ["y", "x"]);

        t.sort(SortKey::Name, false);
        assert_eq!(names(&t, a), ["x", "y"]);

        t.sort(SortKey::Items, false);
        assert_eq!(names(&t, NodeId::ROOT), ["a", "b", "c"]);
    }

    #[test]
    fn name_sort_is_case_insensitive() {
        let mut t = Tree::new("/r", dir());
        for n in ["b", "A", "a", "C"] {
            t.add_child(NodeId::ROOT, OsStr::new(n), &file(1));
        }
        t.sort(SortKey::Name, false);
        assert_eq!(names(&t, NodeId::ROOT), ["A", "a", "b", "C"]);
    }

    #[test]
    fn mtime_sort_newest_first() {
        let mut t = Tree::new("/r", dir());
        for (n, m) in [("old", 1), ("new", 9), ("mid", 5)] {
            let mut f = file(1);
            f.mtime = m;
            t.add_child(NodeId::ROOT, OsStr::new(n), &f);
        }
        t.sort(SortKey::Mtime, false);
        assert_eq!(names(&t, NodeId::ROOT), ["new", "mid", "old"]);
    }

    #[test]
    fn key_cycle_returns_to_size() {
        let mut k = SortKey::Size;
        for _ in 0..4 {
            k = k.next();
        }
        assert_eq!(k, SortKey::Size);
    }
}
