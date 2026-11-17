//! Set of nodes marked for deletion. A marked directory implies all its
//! content, so a node and one of its ancestors are never marked together.

use std::collections::BTreeSet;

use crate::tree::{NodeId, Tree};

/// Effect of [`Marks::toggle`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toggle {
    /// Newly marked; `absorbed` descendant marks were dropped.
    Marked { absorbed: usize },
    /// Mark removed.
    Unmarked,
    /// Already covered by a marked ancestor: nothing changed.
    Covered,
    /// The scan root cannot be marked.
    Root,
}

/// Marked nodes, deduplicated against each other.
#[derive(Debug, Clone, Default)]
pub struct Marks {
    set: BTreeSet<NodeId>,
}

impl Marks {
    /// Marks or unmarks `id`.
    pub fn toggle(&mut self, tree: &Tree, id: NodeId) -> Toggle {
        if id == NodeId::ROOT {
            return Toggle::Root;
        }
        if self.set.remove(&id) {
            return Toggle::Unmarked;
        }
        if tree.ancestors(id).any(|a| self.set.contains(&a)) {
            return Toggle::Covered;
        }
        let before = self.set.len();
        self.set.retain(|&m| !tree.is_ancestor_or_self(id, m));
        let absorbed = before - self.set.len();
        self.set.insert(id);
        Toggle::Marked { absorbed }
    }

    /// Marks `id` unless already covered (used by bulk marking).
    pub fn mark(&mut self, tree: &Tree, id: NodeId) {
        if !self.covers(tree, id) {
            self.toggle(tree, id);
        }
    }

    /// Whether `id` itself is marked.
    pub fn is_marked(&self, id: NodeId) -> bool {
        self.set.contains(&id)
    }

    /// Whether `id` or one of its ancestors is marked.
    pub fn covers(&self, tree: &Tree, id: NodeId) -> bool {
        !self.set.is_empty()
            && (self.set.contains(&id) || tree.ancestors(id).any(|a| self.set.contains(&a)))
    }

    /// Number of marked nodes.
    pub fn len(&self) -> usize {
        self.set.len()
    }

    /// Whether nothing is marked.
    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    /// Bytes freed by deleting every marked node (no double counting).
    pub fn total(&self, tree: &Tree, apparent: bool) -> u64 {
        self.set
            .iter()
            .map(|&id| tree.node(id).size(apparent))
            .sum()
    }

    /// Marked nodes in id order.
    pub fn ids(&self) -> Vec<NodeId> {
        self.set.iter().copied().collect()
    }

    /// Drops marks on nodes that no longer exist in the tree.
    pub fn prune(&mut self, tree: &Tree) {
        self.set
            .retain(|&id| id.index() < tree.len() && !tree.is_removed(id));
    }

    /// Removes one mark.
    pub fn unmark(&mut self, id: NodeId) {
        self.set.remove(&id);
    }

    /// Removes every mark.
    pub fn clear(&mut self) {
        self.set.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::test_util::sample;
    use std::path::Path;

    #[test]
    fn toggle_marks_and_unmarks() {
        let t = sample();
        let b = t.find(Path::new("b")).unwrap();
        let mut m = Marks::default();
        assert_eq!(m.toggle(&t, b), Toggle::Marked { absorbed: 0 });
        assert!(m.is_marked(b));
        assert_eq!(m.total(&t, false), 50);
        assert_eq!(m.toggle(&t, b), Toggle::Unmarked);
        assert!(m.is_empty());
    }

    #[test]
    fn parent_absorbs_children_marks() {
        let t = sample();
        let a = t.find(Path::new("a")).unwrap();
        let x = t.find(Path::new("a/x")).unwrap();
        let y = t.find(Path::new("a/y")).unwrap();
        let mut m = Marks::default();
        m.toggle(&t, x);
        m.toggle(&t, y);
        assert_eq!(m.len(), 2);
        assert_eq!(m.toggle(&t, a), Toggle::Marked { absorbed: 2 });
        assert_eq!(m.ids(), vec![a]);
        assert_eq!(m.total(&t, false), 400, "no double counting");
        assert!(m.covers(&t, x));
        assert!(!m.is_marked(x));
    }

    #[test]
    fn child_of_marked_parent_is_covered() {
        let t = sample();
        let a = t.find(Path::new("a")).unwrap();
        let x = t.find(Path::new("a/x")).unwrap();
        let mut m = Marks::default();
        m.toggle(&t, a);
        assert_eq!(m.toggle(&t, x), Toggle::Covered);
        assert_eq!(m.ids(), vec![a]);
    }

    #[test]
    fn root_is_never_marked() {
        let t = sample();
        let mut m = Marks::default();
        assert_eq!(m.toggle(&t, NodeId::ROOT), Toggle::Root);
        assert!(m.is_empty());
    }

    #[test]
    fn prune_drops_removed_nodes() {
        let mut t = sample();
        let b = t.find(Path::new("b")).unwrap();
        let mut m = Marks::default();
        m.toggle(&t, b);
        t.remove_subtree(b);
        m.prune(&t);
        assert!(m.is_empty());
    }
}
