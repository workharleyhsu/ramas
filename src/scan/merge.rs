//! Copies the scanned entry tree into the arena.

use std::collections::HashSet;
use std::path::PathBuf;

use super::walker::Entry;
use crate::tree::{Kind, Meta, NodeId, Tree, flags};

/// Builds and aggregates the tree. Extra hard links of an inode already seen
/// get [`flags::HARDLINK`] so their size counts once.
pub(super) fn build(root: PathBuf, meta: Meta, children: Vec<Entry>) -> Tree {
    let mut tree = Tree::new(root, meta);
    let mut seen: HashSet<(u64, u64)> = HashSet::new();
    let mut stack: Vec<(NodeId, Vec<Entry>)> = vec![(NodeId::ROOT, children)];
    while let Some((parent, entries)) = stack.pop() {
        for mut e in entries {
            if e.meta.kind != Kind::Dir && e.nlink > 1 && !seen.insert((e.meta.dev, e.meta.ino)) {
                e.meta.flags |= flags::HARDLINK;
            }
            let id = tree.add_child(parent, &e.name, &e.meta);
            if !e.children.is_empty() {
                stack.push((id, std::mem::take(&mut e.children)));
            }
        }
    }
    tree.aggregate();
    tree
}
