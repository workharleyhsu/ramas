//! Safe deletion of scanned paths.
//!
//! Every target is validated right before removal:
//! 1. the path is rebuilt from its canonical *parent* plus its own name, so a
//!    symlink is judged as itself and never as its target;
//! 2. it must lie strictly inside the canonical scan root;
//! 3. `/`, `$HOME` (and any ancestor of it) and the scan root are refused;
//! 4. it must still exist with the same device and inode as when scanned.
//!
//! Removal never follows symlinks: links and files go through
//! `remove_file`, real directories through `remove_dir_all` (which does not
//! follow symlinks either), and the trash backend moves the entry itself.

use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

use crate::tree::{NodeId, Tree};

/// How marked items are removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// Move to the freedesktop trash.
    Trash,
    /// Unlink for good.
    Permanent,
}

/// Why a path was not deleted.
#[derive(Debug, thiserror::Error)]
pub enum DeleteError {
    #[error("refusing to delete the filesystem root")]
    FilesystemRoot,
    #[error("refusing to delete the home directory or one of its parents")]
    Home,
    #[error("refusing to delete the scan root")]
    ScanRoot,
    #[error("path is outside the scanned directory")]
    OutsideRoot,
    #[error("path has no usable file name")]
    BadPath,
    #[error("no longer exists: {0}")]
    Missing(io::Error),
    #[error("changed since the scan (different inode)")]
    Changed,
    #[error("trash failed: {0}")]
    Trash(String),
    #[error("{0}")]
    Io(#[from] io::Error),
}

/// One item to delete, with the identity recorded by the scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub node: NodeId,
    pub path: PathBuf,
    pub dev: u64,
    pub ino: u64,
    pub size: u64,
}

impl Target {
    /// Target for a node of `tree`.
    pub fn of(tree: &Tree, id: NodeId, apparent: bool) -> Self {
        let n = tree.node(id);
        Target {
            node: id,
            path: tree.path(id),
            dev: tree.dev(id),
            ino: n.ino,
            size: n.size(apparent),
        }
    }
}

/// Fixed context of a delete run.
#[derive(Debug, Clone)]
pub struct Guard {
    root: PathBuf,
    home: Option<PathBuf>,
}

impl Guard {
    /// Guard for a scan rooted at `root`. Fails if the root cannot be
    /// canonicalized (then nothing may be deleted).
    pub fn new(root: &Path) -> io::Result<Self> {
        let home = dirs::home_dir().and_then(|h| h.canonicalize().ok());
        Self::with_home(root, home)
    }

    /// Same as [`Guard::new`] with an explicit home directory (tests).
    pub fn with_home(root: &Path, home: Option<PathBuf>) -> io::Result<Self> {
        Ok(Guard {
            root: root.canonicalize()?,
            home,
        })
    }

    /// Checks `t` and returns the exact path to remove.
    pub fn validate(&self, t: &Target) -> Result<PathBuf, DeleteError> {
        let path = &t.path;
        if !path.is_absolute() || path.components().any(|c| c == Component::ParentDir) {
            return Err(DeleteError::BadPath);
        }
        let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
            return Err(DeleteError::FilesystemRoot);
        };
        let parent = parent.canonicalize().map_err(DeleteError::Missing)?;
        let full = parent.join(name);
        if full == Path::new("/") {
            return Err(DeleteError::FilesystemRoot);
        }
        if self.home.as_deref().is_some_and(|h| h.starts_with(&full)) {
            return Err(DeleteError::Home);
        }
        if full == self.root {
            return Err(DeleteError::ScanRoot);
        }
        if !full.starts_with(&self.root) {
            return Err(DeleteError::OutsideRoot);
        }
        let md = std::fs::symlink_metadata(&full).map_err(DeleteError::Missing)?;
        if md.dev() != t.dev || md.ino() != t.ino {
            return Err(DeleteError::Changed);
        }
        Ok(full)
    }
}

/// Filesystem operations, swappable in tests.
pub trait Remover {
    /// Moves `path` to the trash.
    fn trash(&self, path: &Path) -> Result<(), DeleteError>;
    /// Removes a file or symlink.
    fn remove_file(&self, path: &Path) -> io::Result<()>;
    /// Removes a directory recursively without following symlinks.
    fn remove_dir_all(&self, path: &Path) -> io::Result<()>;
}

/// The real filesystem.
#[derive(Debug, Clone, Copy, Default)]
pub struct FsRemover;

impl Remover for FsRemover {
    fn trash(&self, path: &Path) -> Result<(), DeleteError> {
        trash::delete(path).map_err(|e| DeleteError::Trash(e.to_string()))
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(path)
    }

    fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_dir_all(path)
    }
}

/// Result for one target.
#[derive(Debug)]
pub struct Outcome {
    pub target: Target,
    pub result: Result<(), DeleteError>,
}

/// Validates and removes every target, continuing after failures.
pub fn delete_all(
    targets: &[Target],
    method: Method,
    guard: &Guard,
    remover: &dyn Remover,
) -> Vec<Outcome> {
    targets
        .iter()
        .map(|t| Outcome {
            target: t.clone(),
            result: delete_one(t, method, guard, remover),
        })
        .collect()
}

fn delete_one(
    t: &Target,
    method: Method,
    guard: &Guard,
    remover: &dyn Remover,
) -> Result<(), DeleteError> {
    let path = guard.validate(t)?;
    match method {
        Method::Trash => remover.trash(&path),
        Method::Permanent => {
            let md = std::fs::symlink_metadata(&path).map_err(DeleteError::Missing)?;
            if md.file_type().is_dir() {
                remover.remove_dir_all(&path)?;
            } else {
                remover.remove_file(&path)?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
