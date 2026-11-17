//! Per-subtree statistics: heaviest files and size by category.

use crate::classify::{CatId, Classifier};
use crate::tree::{Kind, NodeId, Tree};

/// Totals of one category.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CatStat {
    /// Bytes under the chosen metric.
    pub size: u64,
    /// Number of files.
    pub count: u64,
}

/// Statistics of a subtree.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stats {
    /// Heaviest files, largest first.
    pub top: Vec<NodeId>,
    /// Totals indexed by [`CatId`].
    pub cats: Vec<CatStat>,
}

impl Stats {
    /// Category ids with at least one file, largest first.
    pub fn ranked(&self) -> Vec<CatId> {
        let mut ids: Vec<CatId> = (0..self.cats.len())
            .filter(|&i| self.cats[i].count > 0)
            .map(|i| i as CatId)
            .collect();
        ids.sort_by(|&a, &b| {
            self.cats[usize::from(b)]
                .size
                .cmp(&self.cats[usize::from(a)].size)
                .then(a.cmp(&b))
        });
        ids
    }
}

/// Walks the subtree of `id` once (`id` included when it is a file).
pub fn stats(tree: &Tree, id: NodeId, apparent: bool, cls: &Classifier, top_n: usize) -> Stats {
    let mut cats = vec![CatStat::default(); cls.len()];
    let mut top = TopN::new(top_n);
    for n in std::iter::once(id).chain(tree.descendants(id)) {
        let node = tree.node(n);
        if node.kind == Kind::Dir {
            continue;
        }
        let size = node.size(apparent);
        let c = &mut cats[usize::from(cls.of_node(tree, n))];
        c.size += size;
        c.count += 1;
        top.offer(n, size);
    }
    Stats {
        top: top.into_ids(),
        cats,
    }
}

/// Heaviest `n` files of one category inside the subtree of `id`.
pub fn top_in_category(
    tree: &Tree,
    id: NodeId,
    apparent: bool,
    cls: &Classifier,
    cat: CatId,
    n: usize,
) -> Vec<NodeId> {
    let mut top = TopN::new(n);
    for d in std::iter::once(id).chain(tree.descendants(id)) {
        let node = tree.node(d);
        if node.kind != Kind::Dir && cls.of_node(tree, d) == cat {
            top.offer(d, node.size(apparent));
        }
    }
    top.into_ids()
}

/// Keeps the `n` largest items seen, sorted descending.
struct TopN {
    n: usize,
    items: Vec<(u64, NodeId)>,
}

impl TopN {
    fn new(n: usize) -> Self {
        TopN {
            n,
            items: Vec::with_capacity(n + 1),
        }
    }

    fn offer(&mut self, id: NodeId, size: u64) {
        if self.n == 0 {
            return;
        }
        if self.items.len() == self.n && self.items.last().is_some_and(|&(s, _)| size <= s) {
            return;
        }
        let pos = self
            .items
            .partition_point(|&(s, i)| s > size || (s == size && i < id));
        self.items.insert(pos, (size, id));
        self.items.truncate(self.n);
    }

    fn into_ids(self) -> Vec<NodeId> {
        self.items.into_iter().map(|(_, id)| id).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::test_util::{dir, file};
    use std::ffi::OsStr;

    fn fixture() -> Tree {
        let mut t = Tree::new("/r", dir());
        let m = t.add_child(NodeId::ROOT, OsStr::new("media"), &dir());
        t.add_child(m, OsStr::new("a.mp4"), &file(5000));
        t.add_child(m, OsStr::new("b.MKV"), &file(3000));
        t.add_child(m, OsStr::new("c.jpg"), &file(200));
        t.add_child(NodeId::ROOT, OsStr::new("notes.txt"), &file(10));
        t.add_child(NodeId::ROOT, OsStr::new("x.tar.gz"), &file(900));
        t.add_child(NodeId::ROOT, OsStr::new("Makefile"), &file(1));
        t.aggregate();
        t
    }

    #[test]
    fn top_files_ordered() {
        let t = fixture();
        let cls = Classifier::default();
        let s = stats(&t, NodeId::ROOT, false, &cls, 3);
        let names: Vec<_> = s.top.iter().map(|&i| t.name(i).to_owned()).collect();
        assert_eq!(names, ["a.mp4", "b.MKV", "x.tar.gz"]);
    }

    #[test]
    fn categories_sum_correctly() {
        let t = fixture();
        let cls = Classifier::default();
        let s = stats(&t, NodeId::ROOT, false, &cls, 10);
        let by = |name: &str| {
            let id = (0..cls.len())
                .find(|&i| cls.name(i as CatId) == name)
                .unwrap();
            s.cats[id]
        };
        assert_eq!(
            by("video"),
            CatStat {
                size: 8000,
                count: 2
            }
        );
        assert_eq!(
            by("image"),
            CatStat {
                size: 200,
                count: 1
            }
        );
        assert_eq!(by("documents"), CatStat { size: 10, count: 1 });
        assert_eq!(
            by("archives"),
            CatStat {
                size: 900,
                count: 1
            }
        );
        assert_eq!(by("other"), CatStat { size: 1, count: 1 });
        let ranked: Vec<_> = s.ranked().iter().map(|&c| cls.name(c).to_owned()).collect();
        assert_eq!(ranked, ["video", "archives", "image", "documents", "other"]);
    }

    #[test]
    fn subtree_only_and_category_top() {
        let t = fixture();
        let cls = Classifier::default();
        let media = t.find("media".as_ref()).unwrap();
        let s = stats(&t, media, false, &cls, 10);
        assert_eq!(s.top.len(), 3);
        let video = cls.of_name(b"x.mp4");
        let v = top_in_category(&t, NodeId::ROOT, false, &cls, video, 1);
        assert_eq!(t.name(v[0]), "a.mp4");
    }
}
