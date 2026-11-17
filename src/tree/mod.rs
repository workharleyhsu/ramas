//! Arena-backed directory tree: storage, aggregation and ordering.
//!
//! Nodes live in one `Vec` and refer to each other through `u32` indices, and
//! every name lives in a single byte blob. A parent always has a smaller
//! index than its children, which lets aggregation run as one reverse pass.

mod edit;
mod sort;

pub use edit::Removed;
pub use sort::SortKey;

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

/// Sentinel for "no link".
const NONE: u32 = u32::MAX;

/// Index of a node in a [`Tree`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u32);

impl NodeId {
    /// The root node of every tree.
    pub const ROOT: NodeId = NodeId(0);

    /// Index as `usize`, for slices indexed by node.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// File type of a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Regular file.
    File,
    /// Directory.
    Dir,
    /// Symbolic link (never followed).
    Symlink,
    /// Socket, fifo, device or unknown.
    Other,
}

/// Per-node status bits.
pub mod flags {
    /// Reading metadata or listing the directory failed.
    pub const ERROR: u8 = 1;
    /// Extra hard link of an inode already counted elsewhere.
    pub const HARDLINK: u8 = 1 << 1;
    /// Not descended: pseudo filesystem such as `/proc`.
    pub const SKIP_PSEUDO: u8 = 1 << 2;
    /// Not descended: different filesystem and `-x` is active.
    pub const SKIP_FS: u8 = 1 << 3;
    /// Removed from the tree (deleted); the slot is dead.
    pub const REMOVED: u8 = 1 << 4;
}

/// Metadata of one filesystem entry, as recorded by the scanner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meta {
    /// File type.
    pub kind: Kind,
    /// Disk usage in bytes (`st_blocks * 512`).
    pub disk: u64,
    /// Apparent size in bytes (`st_size`).
    pub apparent: u64,
    /// Modification time, seconds since the epoch.
    pub mtime: i64,
    /// Raw `st_mode`.
    pub mode: u32,
    /// Device id.
    pub dev: u64,
    /// Inode number.
    pub ino: u64,
    /// Status bits from [`flags`].
    pub flags: u8,
}

impl Meta {
    /// Metadata for an entry whose `lstat` failed.
    pub fn error(kind: Kind) -> Self {
        Meta {
            kind,
            disk: 0,
            apparent: 0,
            mtime: 0,
            mode: 0,
            dev: 0,
            ino: 0,
            flags: flags::ERROR,
        }
    }
}

/// One stored node. Sizes and counts of a directory include all descendants.
#[derive(Debug, Clone)]
pub struct Node {
    /// Disk usage, aggregated for directories.
    pub disk: u64,
    /// Apparent size, aggregated for directories.
    pub apparent: u64,
    /// Inode number.
    pub ino: u64,
    /// Modification time (seconds since the epoch).
    pub mtime: i64,
    /// Raw `st_mode`.
    pub mode: u32,
    /// Number of non-directory descendants.
    pub files: u32,
    /// Number of directory descendants.
    pub dirs: u32,
    /// Number of descendants with [`flags::ERROR`], self included.
    pub errors: u32,
    name_off: u32,
    parent: u32,
    first_child: u32,
    last_child: u32,
    next_sibling: u32,
    prev_sibling: u32,
    name_len: u16,
    dev_idx: u16,
    /// File type.
    pub kind: Kind,
    /// Status bits from [`flags`].
    pub flags: u8,
}

impl Node {
    /// Size under the chosen metric.
    pub fn size(&self, apparent: bool) -> u64 {
        if apparent { self.apparent } else { self.disk }
    }

    /// Number of descendants (files and directories).
    pub fn items(&self) -> u64 {
        u64::from(self.files) + u64::from(self.dirs)
    }

    /// Whether the given flag bit is set.
    pub fn has(&self, flag: u8) -> bool {
        self.flags & flag != 0
    }

    /// Whether this node is a directory.
    pub fn is_dir(&self) -> bool {
        self.kind == Kind::Dir
    }
}

/// Arena tree of a scanned directory.
#[derive(Debug, Clone)]
pub struct Tree {
    nodes: Vec<Node>,
    names: Vec<u8>,
    devs: Vec<u64>,
    root_path: PathBuf,
    removed: usize,
}

impl Tree {
    /// Creates a tree whose root is `root_path` with the given metadata.
    pub fn new(root_path: impl Into<PathBuf>, meta: Meta) -> Self {
        let root_path = root_path.into();
        let mut tree = Tree {
            nodes: Vec::new(),
            names: Vec::new(),
            devs: Vec::new(),
            removed: 0,
            root_path: PathBuf::new(),
        };
        let name = root_path.as_os_str().to_owned();
        tree.push(NONE, &name, &meta);
        tree.root_path = root_path;
        tree
    }

    /// Absolute path of the scan root.
    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    /// Number of slots, dead ones included. Valid ids are `0..len()`.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Never true: a tree always has a root.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Approximate heap footprint of the arena in bytes.
    pub fn mem_bytes(&self) -> usize {
        self.nodes.capacity() * std::mem::size_of::<Node>()
            + self.names.capacity()
            + self.devs.capacity() * 8
    }

    /// Number of live nodes.
    pub fn live(&self) -> usize {
        self.nodes.len() - self.removed
    }

    /// Node data. Panics on an id that does not belong to this tree.
    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id.index()]
    }

    /// The root node.
    pub fn root(&self) -> &Node {
        self.node(NodeId::ROOT)
    }

    /// Device id of a node.
    pub fn dev(&self, id: NodeId) -> u64 {
        self.devs
            .get(usize::from(self.node(id).dev_idx))
            .copied()
            .unwrap_or(0)
    }

    /// Name of a node. The root's name is its full path.
    pub fn name(&self, id: NodeId) -> &OsStr {
        let n = self.node(id);
        let start = n.name_off as usize;
        OsStr::from_bytes(&self.names[start..start + usize::from(n.name_len)])
    }

    /// Parent of a node, `None` for the root.
    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        link(self.node(id).parent)
    }

    /// Whether `id` has at least one child.
    pub fn has_children(&self, id: NodeId) -> bool {
        self.node(id).first_child != NONE
    }

    /// Iterates over the children of a node in their current order.
    pub fn children(&self, id: NodeId) -> Children<'_> {
        Children {
            tree: self,
            next: self.node(id).first_child,
        }
    }

    /// Iterates from the parent of `id` up to the root.
    pub fn ancestors(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        std::iter::successors(self.parent(id), |&p| self.parent(p))
    }

    /// Depth of a node; the root has depth 0.
    pub fn depth(&self, id: NodeId) -> usize {
        self.ancestors(id).count()
    }

    /// Whether `anc` is `id` or one of its ancestors.
    pub fn is_ancestor_or_self(&self, anc: NodeId, id: NodeId) -> bool {
        anc == id || self.ancestors(id).any(|a| a == anc)
    }

    /// Whether the node has been removed.
    pub fn is_removed(&self, id: NodeId) -> bool {
        self.node(id).has(flags::REMOVED)
    }

    /// Full filesystem path of a node.
    pub fn path(&self, id: NodeId) -> PathBuf {
        let mut parts: Vec<&OsStr> = Vec::new();
        let mut cur = id;
        while let Some(p) = self.parent(cur) {
            parts.push(self.name(cur));
            cur = p;
        }
        let mut path = self.root_path.clone();
        for part in parts.iter().rev() {
            path.push(part);
        }
        path
    }

    /// Path of a node relative to the root (empty for the root).
    pub fn rel_path(&self, id: NodeId) -> PathBuf {
        let full = self.path(id);
        full.strip_prefix(&self.root_path)
            .map(Path::to_path_buf)
            .unwrap_or(full)
    }

    /// Finds a node by its path relative to the root.
    pub fn find(&self, rel: &Path) -> Option<NodeId> {
        let mut cur = NodeId::ROOT;
        for comp in rel.components() {
            let name = comp.as_os_str();
            cur = self.children(cur).find(|&c| self.name(c) == name)?;
        }
        Some(cur)
    }

    /// Appends a child under `parent` (last position) and returns its id.
    ///
    /// Sizes of ancestors are not touched; call [`Tree::aggregate`] once
    /// after building.
    pub fn add_child(&mut self, parent: NodeId, name: &OsStr, meta: &Meta) -> NodeId {
        let id = self.push(parent.0, name, meta);
        self.link_last(parent, id);
        id
    }

    /// Adds every node's totals into its ancestors, bottom-up.
    ///
    /// Call exactly once, after building a fresh tree with
    /// [`Tree::add_child`]; later edits keep totals up to date themselves.
    pub fn aggregate(&mut self) {
        for i in (1..self.nodes.len()).rev() {
            let n = &self.nodes[i];
            if n.has(flags::REMOVED) {
                continue;
            }
            let (disk, apparent) = (n.disk, n.apparent);
            let is_dir = n.kind == Kind::Dir;
            let (files, dirs, errors) = (n.files, n.dirs, n.errors);
            let p = n.parent as usize;
            let parent = &mut self.nodes[p];
            parent.disk += disk;
            parent.apparent += apparent;
            parent.files += files + u32::from(!is_dir);
            parent.dirs += dirs + u32::from(is_dir);
            parent.errors += errors;
        }
    }

    fn push(&mut self, parent: u32, name: &OsStr, meta: &Meta) -> NodeId {
        let bytes = name.as_bytes();
        let len = bytes.len().min(usize::from(u16::MAX));
        let name_off = u32::try_from(self.names.len()).unwrap_or(u32::MAX);
        self.names.extend_from_slice(&bytes[..len]);
        let dev_idx = self.dev_index(meta.dev);
        let id = u32::try_from(self.nodes.len()).unwrap_or(NONE);
        let hardlink = meta.flags & flags::HARDLINK != 0;
        self.nodes.push(Node {
            disk: if hardlink { 0 } else { meta.disk },
            apparent: if hardlink { 0 } else { meta.apparent },
            ino: meta.ino,
            mtime: meta.mtime,
            mode: meta.mode,
            files: 0,
            dirs: 0,
            errors: u32::from(meta.flags & flags::ERROR != 0),
            name_off,
            parent,
            first_child: NONE,
            last_child: NONE,
            next_sibling: NONE,
            prev_sibling: NONE,
            name_len: len as u16,
            dev_idx,
            kind: meta.kind,
            flags: meta.flags,
        });
        NodeId(id)
    }

    fn dev_index(&mut self, dev: u64) -> u16 {
        if let Some(i) = self.devs.iter().position(|&d| d == dev) {
            return i as u16;
        }
        if self.devs.len() >= usize::from(u16::MAX) {
            return 0;
        }
        self.devs.push(dev);
        (self.devs.len() - 1) as u16
    }

    fn link_last(&mut self, parent: NodeId, id: NodeId) {
        let last = self.nodes[parent.index()].last_child;
        {
            let n = &mut self.nodes[id.index()];
            n.parent = parent.0;
            n.prev_sibling = last;
            n.next_sibling = NONE;
        }
        if last == NONE {
            self.nodes[parent.index()].first_child = id.0;
        } else {
            self.nodes[last as usize].next_sibling = id.0;
        }
        self.nodes[parent.index()].last_child = id.0;
    }

    fn unlink(&mut self, id: NodeId) {
        let (parent, prev, next) = {
            let n = &self.nodes[id.index()];
            (n.parent, n.prev_sibling, n.next_sibling)
        };
        if parent == NONE {
            return;
        }
        if prev == NONE {
            self.nodes[parent as usize].first_child = next;
        } else {
            self.nodes[prev as usize].next_sibling = next;
        }
        if next == NONE {
            self.nodes[parent as usize].last_child = prev;
        } else {
            self.nodes[next as usize].prev_sibling = prev;
        }
        let n = &mut self.nodes[id.index()];
        n.prev_sibling = NONE;
        n.next_sibling = NONE;
    }
}

/// Iterator over the children of a node.
pub struct Children<'a> {
    tree: &'a Tree,
    next: u32,
}

impl Iterator for Children<'_> {
    type Item = NodeId;

    fn next(&mut self) -> Option<NodeId> {
        let cur = link(self.next)?;
        self.next = self.tree.node(cur).next_sibling;
        Some(cur)
    }
}

fn link(raw: u32) -> Option<NodeId> {
    (raw != NONE).then_some(NodeId(raw))
}

#[cfg(test)]
pub(crate) mod test_util {
    use super::*;

    /// Metadata of a regular file with equal disk and apparent size.
    pub fn file(size: u64) -> Meta {
        Meta {
            kind: Kind::File,
            disk: size,
            apparent: size,
            mtime: 0,
            mode: 0o100644,
            dev: 1,
            ino: 0,
            flags: 0,
        }
    }

    /// Metadata of an empty directory.
    pub fn dir() -> Meta {
        Meta {
            kind: Kind::Dir,
            mode: 0o040755,
            ..file(0)
        }
    }

    /// root/{a/{x:100, y:300}, b:50, c/}
    pub fn sample() -> Tree {
        let mut t = Tree::new("/r", dir());
        let a = t.add_child(NodeId::ROOT, OsStr::new("a"), &dir());
        t.add_child(a, OsStr::new("x"), &file(100));
        t.add_child(a, OsStr::new("y"), &file(300));
        t.add_child(NodeId::ROOT, OsStr::new("b"), &file(50));
        t.add_child(NodeId::ROOT, OsStr::new("c"), &dir());
        t.aggregate();
        t
    }
}

#[cfg(test)]
mod tests {
    use super::test_util::*;
    use super::*;

    fn names(t: &Tree, id: NodeId) -> Vec<String> {
        t.children(id)
            .map(|c| t.name(c).to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn aggregates_sizes_and_counts() {
        let t = sample();
        let root = t.root();
        assert_eq!(root.disk, 450);
        assert_eq!(root.files, 3);
        assert_eq!(root.dirs, 2);
        let a = t.find(Path::new("a")).unwrap();
        assert_eq!(t.node(a).disk, 400);
        assert_eq!(t.node(a).files, 2);
    }

    #[test]
    fn children_keep_insertion_order() {
        let t = sample();
        assert_eq!(names(&t, NodeId::ROOT), ["a", "b", "c"]);
    }

    #[test]
    fn paths_and_lookup() {
        let t = sample();
        let y = t.find(Path::new("a/y")).unwrap();
        assert_eq!(t.path(y), PathBuf::from("/r/a/y"));
        assert_eq!(t.rel_path(y), PathBuf::from("a/y"));
        assert_eq!(t.depth(y), 2);
        assert_eq!(t.path(NodeId::ROOT), PathBuf::from("/r"));
        assert!(t.find(Path::new("nope")).is_none());
    }

    #[test]
    fn hardlink_contributes_nothing() {
        let mut t = Tree::new("/r", dir());
        t.add_child(NodeId::ROOT, OsStr::new("a"), &file(10));
        let mut dup = file(10);
        dup.flags = flags::HARDLINK;
        t.add_child(NodeId::ROOT, OsStr::new("b"), &dup);
        t.aggregate();
        assert_eq!(t.root().disk, 10);
        assert_eq!(t.root().files, 2);
    }

    #[test]
    fn errors_propagate() {
        let mut t = Tree::new("/r", dir());
        let a = t.add_child(NodeId::ROOT, OsStr::new("a"), &dir());
        t.add_child(a, OsStr::new("bad"), &Meta::error(Kind::Dir));
        t.aggregate();
        assert_eq!(t.root().errors, 1);
    }

    #[test]
    fn node_stays_compact() {
        assert!(std::mem::size_of::<Node>() <= 80);
    }
}
