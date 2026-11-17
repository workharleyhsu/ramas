//! JSON export, import and scan comparison.
//!
//! The format is flat so arbitrarily deep trees never hit JSON recursion
//! limits: a header plus one record per node in depth-first order, each
//! pointing at its parent's index. Sizes are a node's *own* sizes; directory
//! totals are recomputed on import.
//!
//! ```json
//! {"format":"ramas","version":1,"root":"/home/me","scanned_at":1700000000,
//!  "nodes":[{"p":null,"n":"/home/me","k":"d","d":4096,"a":4096,"m":0,"mode":16877,"dev":2049,"ino":2,"f":0}, ...]}
//! ```

pub mod diff;

use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::tree::{Kind, Meta, NodeId, Tree, flags};

/// Value of the `format` field.
pub const FORMAT: &str = "ramas";
/// Current format version.
pub const VERSION: u32 = 1;

/// Name that survives non-UTF-8 bytes: `n` when valid UTF-8, `nb` otherwise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Record {
    p: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    n: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    nb: Option<Vec<u8>>,
    k: char,
    d: u64,
    a: u64,
    m: i64,
    mode: u32,
    dev: u64,
    ino: u64,
    f: u8,
}

#[derive(Debug, Serialize, Deserialize)]
struct Header<'a> {
    format: &'a str,
    version: u32,
    root: String,
    scanned_at: i64,
}

#[derive(Debug, Deserialize)]
struct Document {
    format: String,
    version: u32,
    scanned_at: i64,
    nodes: Vec<Record>,
}

/// An imported scan.
#[derive(Debug)]
pub struct Imported {
    pub tree: Tree,
    pub scanned_at: i64,
}

fn kind_char(k: Kind) -> char {
    match k {
        Kind::File => 'f',
        Kind::Dir => 'd',
        Kind::Symlink => 'l',
        Kind::Other => 'o',
    }
}

fn char_kind(c: char) -> Result<Kind> {
    Ok(match c {
        'f' => Kind::File,
        'd' => Kind::Dir,
        'l' => Kind::Symlink,
        'o' => Kind::Other,
        other => bail!("unknown node kind {other:?}"),
    })
}

/// Current time in seconds since the epoch.
pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// Size of a node minus its live children: what it contributes by itself.
fn own_sizes(tree: &Tree, id: NodeId) -> (u64, u64) {
    let n = tree.node(id);
    let (mut d, mut a) = (n.disk, n.apparent);
    for c in tree.children(id) {
        let cn = tree.node(c);
        d = d.saturating_sub(cn.disk);
        a = a.saturating_sub(cn.apparent);
    }
    (d, a)
}

fn record(tree: &Tree, id: NodeId, parent: Option<u32>) -> Record {
    let n = tree.node(id);
    let name = tree.name(id);
    let (n_utf8, nb) = match name.to_str() {
        Some(s) => (Some(s.to_owned()), None),
        None => (None, Some(name.as_bytes().to_vec())),
    };
    let (d, a) = own_sizes(tree, id);
    Record {
        p: parent,
        n: n_utf8,
        nb,
        k: kind_char(n.kind),
        d,
        a,
        m: n.mtime,
        mode: n.mode,
        dev: tree.dev(id),
        ino: n.ino,
        f: n.flags & !flags::REMOVED,
    }
}

/// Writes `tree` as JSON, streaming one record at a time.
pub fn write(tree: &Tree, scanned_at: i64, out: &mut impl Write) -> Result<()> {
    let header = Header {
        format: FORMAT,
        version: VERSION,
        root: tree.root_path().to_string_lossy().into_owned(),
        scanned_at,
    };
    let head = serde_json::to_string(&header)?;
    // Reopen the header object to append the node array.
    out.write_all(&head.as_bytes()[..head.len() - 1])?;
    out.write_all(b",\"nodes\":[\n")?;
    // Depth-first; indices follow write order.
    let mut index: Vec<u32> = vec![u32::MAX; tree.len()];
    let mut next = 0u32;
    let mut stack = vec![NodeId::ROOT];
    while let Some(id) = stack.pop() {
        let parent = tree.parent(id).map(|p| index[p.index()]);
        if next > 0 {
            out.write_all(b",\n")?;
        }
        serde_json::to_writer(&mut *out, &record(tree, id, parent))?;
        index[id.index()] = next;
        next += 1;
        // Canonical order (by name bytes) so the output does not depend on
        // how the tree is sorted on screen.
        let start = stack.len();
        stack.extend(tree.children(id));
        stack[start..].sort_by(|a, b| tree.name(*b).as_bytes().cmp(tree.name(*a).as_bytes()));
    }
    out.write_all(b"\n]}\n")?;
    Ok(())
}

/// Exports to a file.
pub fn write_file(tree: &Tree, scanned_at: i64, path: &Path) -> Result<()> {
    let file = File::create(path).with_context(|| format!("cannot create {}", path.display()))?;
    let mut w = BufWriter::new(file);
    write(tree, scanned_at, &mut w)?;
    w.flush()?;
    Ok(())
}

/// Parses an export.
pub fn read(input: impl std::io::Read) -> Result<Imported> {
    let doc: Document = serde_json::from_reader(input).context("not a ramas export")?;
    if doc.format != FORMAT {
        bail!("not a ramas export (format {:?})", doc.format);
    }
    if doc.version != VERSION {
        bail!("unsupported export version {}", doc.version);
    }
    let mut records = doc.nodes.into_iter();
    let Some(root) = records.next() else {
        bail!("export has no nodes");
    };
    if root.p.is_some() {
        bail!("first node must be the root");
    }
    let root_path = PathBuf::from(name_of(&root));
    let mut tree = Tree::new(root_path, meta_of(&root)?);
    let mut ids = vec![NodeId::ROOT];
    for (i, r) in records.enumerate() {
        let Some(&parent) = r.p.and_then(|p| ids.get(p as usize)) else {
            bail!("node {} points to a missing parent", i + 1);
        };
        if tree.node(parent).kind != Kind::Dir {
            bail!("node {} has a non-directory parent", i + 1);
        }
        let id = tree.add_child(parent, &name_of(&r), &meta_of(&r)?);
        ids.push(id);
    }
    tree.aggregate();
    tree.shrink();
    Ok(Imported {
        tree,
        scanned_at: doc.scanned_at,
    })
}

/// Imports from a file.
pub fn read_file(path: &Path) -> Result<Imported> {
    let file = File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    read(BufReader::new(file)).with_context(|| format!("cannot import {}", path.display()))
}

fn name_of(r: &Record) -> std::ffi::OsString {
    match (&r.n, &r.nb) {
        (Some(s), _) => s.into(),
        (None, Some(b)) => OsStr::from_bytes(b).to_owned(),
        (None, None) => std::ffi::OsString::new(),
    }
}

fn meta_of(r: &Record) -> Result<Meta> {
    Ok(Meta {
        kind: char_kind(r.k)?,
        disk: r.d,
        apparent: r.a,
        mtime: r.m,
        mode: r.mode,
        dev: r.dev,
        ino: r.ino,
        flags: r.f & !flags::REMOVED,
    })
}

#[cfg(test)]
mod tests;
