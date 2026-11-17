//! Recursive parallel directory walk.

use std::ffi::OsString;
use std::fs::{self, Metadata};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use rayon::prelude::*;

use super::{Progress, ScanOptions, pseudo};
use crate::tree::{Kind, Meta, flags};

pub(super) struct Ctx<'a> {
    pub opts: &'a ScanOptions,
    pub progress: &'a Progress,
    pub cancel: &'a AtomicBool,
    pub root_dev: u64,
}

/// One scanned entry with its (already scanned) children.
pub(super) struct Entry {
    pub name: OsString,
    pub meta: Meta,
    pub nlink: u64,
    pub children: Vec<Entry>,
}

/// Scans the root itself and, when it is a directory, its contents.
pub(super) fn walk_root(ctx: &Ctx, root: &Path, md: &Metadata) -> (Meta, Vec<Entry>) {
    let mut meta = meta_of(md);
    ctx.progress.add_entry(meta.disk);
    if meta.kind != Kind::Dir {
        return (meta, Vec::new());
    }
    let (children, fail) = walk_dir(ctx, root, meta.dev);
    meta.flags |= fail;
    (meta, children)
}

/// Lists `dir` and descends into its subdirectories in parallel.
/// Returns the entries and the error flags listing produced (0 if none).
fn walk_dir(ctx: &Ctx, dir: &Path, dev: u64) -> (Vec<Entry>, u8) {
    if ctx.cancel.load(Ordering::Relaxed) {
        return (Vec::new(), 0);
    }
    ctx.progress.set_current(dir);
    let (mut entries, fail) = list(ctx, dir, dev);
    entries
        .par_iter_mut()
        .filter(|e| e.meta.kind == Kind::Dir && e.meta.flags == 0)
        .for_each(|e| {
            let path = dir.join(&e.name);
            let (children, fail) = walk_dir(ctx, &path, e.meta.dev);
            e.meta.flags |= fail;
            e.children = children;
        });
    (entries, fail)
}

/// Reads one directory level without descending.
fn list(ctx: &Ctx, dir: &Path, dev: u64) -> (Vec<Entry>, u8) {
    let rd = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) => {
            ctx.progress.add_error();
            return (Vec::new(), fail_flags(&e));
        }
    };
    let mut fail = 0;
    let mut out = Vec::new();
    for item in rd {
        let item = match item {
            Ok(item) => item,
            Err(e) => {
                ctx.progress.add_error();
                fail |= fail_flags(&e);
                continue;
            }
        };
        let name = item.file_name();
        // DirEntry::metadata does not follow symlinks on Unix.
        let entry = match item.metadata() {
            Ok(md) => {
                let mut meta = meta_of(&md);
                if meta.kind == Kind::Dir {
                    meta.flags |= skip_flag(ctx, &dir.join(&name), meta.dev, dev);
                }
                Entry {
                    name,
                    meta,
                    nlink: md.nlink(),
                    children: Vec::new(),
                }
            }
            Err(e) => {
                ctx.progress.add_error();
                let kind = item.file_type().map(kind_of).unwrap_or(Kind::Other);
                let mut meta = Meta::error(kind);
                meta.flags |= fail_flags(&e);
                Entry {
                    name,
                    meta,
                    nlink: 1,
                    children: Vec::new(),
                }
            }
        };
        ctx.progress.add_entry(entry.meta.disk);
        out.push(entry);
    }
    (out, fail)
}

/// Flags for a read failure: always [`flags::ERROR`], plus
/// [`flags::DENIED`] when it is only a matter of permissions.
fn fail_flags(e: &std::io::Error) -> u8 {
    if e.kind() == std::io::ErrorKind::PermissionDenied {
        flags::ERROR | flags::DENIED
    } else {
        flags::ERROR
    }
}

/// Decides whether a subdirectory must not be descended.
fn skip_flag(ctx: &Ctx, path: &Path, dev: u64, parent_dev: u64) -> u8 {
    if ctx.opts.one_file_system && dev != ctx.root_dev {
        return flags::SKIP_FS;
    }
    if pseudo::is_pseudo(path, dev != parent_dev) {
        return flags::SKIP_PSEUDO;
    }
    0
}

pub(super) fn meta_of(md: &Metadata) -> Meta {
    Meta {
        kind: kind_of(md.file_type()),
        disk: md.blocks().saturating_mul(512),
        apparent: md.size(),
        mtime: md.mtime(),
        mode: md.mode(),
        dev: md.dev(),
        ino: md.ino(),
        flags: 0,
    }
}

fn kind_of(ft: fs::FileType) -> Kind {
    if ft.is_symlink() {
        Kind::Symlink
    } else if ft.is_dir() {
        Kind::Dir
    } else if ft.is_file() {
        Kind::File
    } else {
        // fifo, socket, block or char device
        Kind::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_file_system_skips_other_devices() {
        let opts = ScanOptions {
            one_file_system: true,
        };
        let progress = Progress::default();
        let cancel = AtomicBool::new(false);
        let ctx = Ctx {
            opts: &opts,
            progress: &progress,
            cancel: &cancel,
            root_dev: 1,
        };
        assert_eq!(skip_flag(&ctx, Path::new("/x/mnt"), 2, 1), flags::SKIP_FS);
        assert_eq!(skip_flag(&ctx, Path::new("/x/sub"), 1, 1), 0);
        let opts = ScanOptions::default();
        let ctx = Ctx { opts: &opts, ..ctx };
        assert_eq!(skip_flag(&ctx, Path::new("/x/mnt"), 2, 1), 0);
    }
}
