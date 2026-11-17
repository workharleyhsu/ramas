//! Flattened list of the rows currently visible in the tree pane.

use crate::tree::{NodeId, Tree};

/// One visible line of the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    /// Node shown.
    pub id: NodeId,
    /// Depth below the root (root = 0).
    pub depth: u32,
}

/// Builds every visible row: the root, then the children of expanded
/// directories in tree order. `visible` hides nodes (filters).
pub fn build(tree: &Tree, expanded: &[bool], visible: &dyn Fn(NodeId) -> bool) -> Vec<Row> {
    let mut out = vec![Row {
        id: NodeId::ROOT,
        depth: 0,
    }];
    if is_open(expanded, NodeId::ROOT) {
        descendants(tree, NodeId::ROOT, 1, expanded, visible, &mut out);
    }
    out
}

/// Appends the visible descendants of `id` (which is shown at `depth - 1`).
pub fn descendants(
    tree: &Tree,
    id: NodeId,
    depth: u32,
    expanded: &[bool],
    visible: &dyn Fn(NodeId) -> bool,
    out: &mut Vec<Row>,
) {
    let mut stack = vec![(tree.children(id), depth)];
    while let Some(top) = stack.last_mut() {
        let d = top.1;
        let Some(child) = top.0.next() else {
            stack.pop();
            continue;
        };
        if !visible(child) {
            continue;
        }
        out.push(Row {
            id: child,
            depth: d,
        });
        if is_open(expanded, child) && tree.has_children(child) {
            stack.push((tree.children(child), d + 1));
        }
    }
}

/// Whether `id` is expanded.
pub fn is_open(expanded: &[bool], id: NodeId) -> bool {
    expanded.get(id.index()).copied().unwrap_or(false)
}

/// Index just past the rows that belong to the subtree of row `i`.
pub fn subtree_end(rows: &[Row], i: usize) -> usize {
    let depth = rows[i].depth;
    rows[i + 1..]
        .iter()
        .position(|r| r.depth <= depth)
        .map_or(rows.len(), |p| i + 1 + p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::test_util::sample;

    #[test]
    fn collapsed_root_shows_only_root() {
        let t = sample();
        let rows = build(&t, &vec![false; t.len()], &|_| true);
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn expanded_dirs_show_children_in_order() {
        let t = sample();
        let mut open = vec![false; t.len()];
        open[0] = true;
        let rows = build(&t, &open, &|_| true);
        assert_eq!(rows.len(), 4); // root a b c
        let a = t.find("a".as_ref()).unwrap();
        open[a.index()] = true;
        let rows = build(&t, &open, &|_| true);
        let names: Vec<_> = rows
            .iter()
            .map(|r| (t.name(r.id).to_owned(), r.depth))
            .collect();
        assert_eq!(names[1].1, 1);
        assert_eq!(names[2].1, 2);
        assert_eq!(rows.len(), 6);
        assert_eq!(subtree_end(&rows, 1), 4);
        assert_eq!(subtree_end(&rows, 0), 6);
    }

    #[test]
    fn visible_predicate_hides_rows() {
        let t = sample();
        let mut open = vec![true; t.len()];
        open[0] = true;
        let a = t.find("a".as_ref()).unwrap();
        let rows = build(&t, &open, &|id| id != a);
        assert_eq!(rows.len(), 3); // root b c
    }
}
