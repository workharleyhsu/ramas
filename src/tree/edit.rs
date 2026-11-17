//! Structural edits that keep ancestor totals consistent: removal of a
//! subtree (after a delete) and replacement of a subtree (after a rescan).

use super::{Kind, NONE, Node, NodeId, Tree, flags};

/// Totals of a removed subtree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Removed {
    /// Disk usage freed.
    pub disk: u64,
    /// Apparent size freed.
    pub apparent: u64,
    /// Number of nodes removed.
    pub nodes: u64,
}

#[derive(Clone, Copy)]
struct Totals {
    disk: u64,
    apparent: u64,
    files: u32,
    dirs: u32,
    errors: u32,
}

impl Totals {
    /// What a node contributes to its parent's totals.
    fn contribution(n: &Node) -> Self {
        let is_dir = n.kind == Kind::Dir;
        Totals {
            disk: n.disk,
            apparent: n.apparent,
            files: n.files + u32::from(!is_dir),
            dirs: n.dirs + u32::from(is_dir),
            errors: n.errors,
        }
    }
}

impl Tree {
    /// Removes `id` and its descendants, subtracting their totals from every
    /// ancestor. The root cannot be removed (returns `None`).
    pub fn remove_subtree(&mut self, id: NodeId) -> Option<Removed> {
        let parent = self.parent(id)?;
        if self.is_removed(id) {
            return None;
        }
        let t = Totals::contribution(self.node(id));
        self.unlink(id);
        self.sub_from_chain(parent, t);
        let nodes = self.mark_removed(id);
        Some(Removed {
            disk: t.disk,
            apparent: t.apparent,
            nodes,
        })
    }

    /// Replaces the contents of directory `id` with those of `sub`, whose root
    /// is the fresh scan of the same path. Ancestors are adjusted by the delta.
    pub fn replace_subtree(&mut self, id: NodeId, sub: &Tree) {
        let old = Totals::contribution(self.node(id));
        let kids: Vec<NodeId> = self.children(id).collect();
        for k in kids {
            self.unlink(k);
            self.mark_removed(k);
        }
        let mut map = vec![NONE; sub.nodes.len()];
        map[0] = id.0;
        for i in 1..sub.nodes.len() {
            let src = &sub.nodes[i];
            if src.has(flags::REMOVED) {
                continue;
            }
            let parent = map[src.parent as usize];
            if parent == NONE {
                continue;
            }
            let name = sub.name(NodeId(i as u32)).to_owned();
            let dev = sub.dev(NodeId(i as u32));
            let new_id = self.push_raw(src, &name, dev);
            self.link_last(NodeId(parent), new_id);
            map[i] = new_id.0;
        }
        let dev_idx = self.dev_index(sub.dev(NodeId::ROOT));
        let r = sub.root();
        let n = &mut self.nodes[id.index()];
        n.disk = r.disk;
        n.apparent = r.apparent;
        n.files = r.files;
        n.dirs = r.dirs;
        n.errors = r.errors;
        n.mtime = r.mtime;
        n.mode = r.mode;
        n.ino = r.ino;
        n.kind = r.kind;
        n.flags = r.flags;
        n.dev_idx = dev_idx;
        let new = Totals::contribution(self.node(id));
        if let Some(p) = self.parent(id) {
            self.sub_from_chain(p, old);
            self.add_to_chain(p, new);
        }
    }

    fn push_raw(&mut self, src: &Node, name: &std::ffi::OsStr, dev: u64) -> NodeId {
        use std::os::unix::ffi::OsStrExt;
        let bytes = name.as_bytes();
        let name_off = u32::try_from(self.names.len()).unwrap_or(u32::MAX);
        self.names.extend_from_slice(bytes);
        let dev_idx = self.dev_index(dev);
        let id = NodeId(u32::try_from(self.nodes.len()).unwrap_or(NONE));
        self.nodes.push(Node {
            name_off,
            name_len: src.name_len,
            dev_idx,
            parent: NONE,
            first_child: NONE,
            last_child: NONE,
            next_sibling: NONE,
            prev_sibling: NONE,
            ..src.clone()
        });
        id
    }

    fn sub_from_chain(&mut self, start: NodeId, t: Totals) {
        let mut cur = Some(start);
        while let Some(c) = cur {
            let n = &mut self.nodes[c.index()];
            n.disk = n.disk.saturating_sub(t.disk);
            n.apparent = n.apparent.saturating_sub(t.apparent);
            n.files = n.files.saturating_sub(t.files);
            n.dirs = n.dirs.saturating_sub(t.dirs);
            n.errors = n.errors.saturating_sub(t.errors);
            cur = self.parent(c);
        }
    }

    fn add_to_chain(&mut self, start: NodeId, t: Totals) {
        let mut cur = Some(start);
        while let Some(c) = cur {
            let n = &mut self.nodes[c.index()];
            n.disk += t.disk;
            n.apparent += t.apparent;
            n.files += t.files;
            n.dirs += t.dirs;
            n.errors += t.errors;
            cur = self.parent(c);
        }
    }

    fn mark_removed(&mut self, id: NodeId) -> u64 {
        let mut stack = vec![id];
        let mut count = 0;
        while let Some(cur) = stack.pop() {
            stack.extend(self.children(cur));
            let n = &mut self.nodes[cur.index()];
            if !n.has(flags::REMOVED) {
                n.flags |= flags::REMOVED;
                count += 1;
            }
        }
        self.removed += count as usize;
        count
    }
}

#[cfg(test)]
mod tests {
    use crate::tree::test_util::*;
    use crate::tree::*;
    use std::ffi::OsStr;

    #[test]
    fn remove_updates_ancestors() {
        let mut t = sample();
        let x = t.find(Path::new("a/x")).unwrap();
        let r = t.remove_subtree(x).unwrap();
        assert_eq!(r.disk, 100);
        assert_eq!(r.nodes, 1);
        let a = t.find(Path::new("a")).unwrap();
        assert_eq!(t.node(a).disk, 300);
        assert_eq!(t.node(a).files, 1);
        assert_eq!(t.root().disk, 350);
        assert_eq!(t.root().files, 2);
        assert!(t.find(Path::new("a/x")).is_none());
        assert!(t.is_removed(x));
        assert_eq!(t.live(), t.len() - 1);
    }

    #[test]
    fn remove_directory_subtree() {
        let mut t = sample();
        let a = t.find(Path::new("a")).unwrap();
        let r = t.remove_subtree(a).unwrap();
        assert_eq!(r.nodes, 3);
        assert_eq!(t.root().disk, 50);
        assert_eq!(t.root().files, 1);
        assert_eq!(t.root().dirs, 1);
        assert!(t.remove_subtree(a).is_none(), "double remove");
    }

    #[test]
    fn root_cannot_be_removed() {
        let mut t = sample();
        assert!(t.remove_subtree(NodeId::ROOT).is_none());
    }

    #[test]
    fn replace_subtree_applies_delta() {
        let mut t = sample();
        let a = t.find(Path::new("a")).unwrap();
        let mut sub = Tree::new("/r/a", dir());
        let z = sub.add_child(NodeId::ROOT, OsStr::new("z"), &dir());
        sub.add_child(z, OsStr::new("big"), &file(1000));
        sub.aggregate();
        t.replace_subtree(a, &sub);
        assert_eq!(t.node(a).disk, 1000);
        assert_eq!(t.root().disk, 1050);
        assert_eq!(t.root().files, 2);
        assert_eq!(t.root().dirs, 3);
        let big = t.find(Path::new("a/z/big")).unwrap();
        assert_eq!(t.path(big), PathBuf::from("/r/a/z/big"));
        assert!(t.find(Path::new("a/x")).is_none());
    }
}
