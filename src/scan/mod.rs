//! Parallel filesystem scanner producing a [`crate::tree::Tree`].
//!
//! Directories are listed with `std::fs` and descended in parallel with
//! rayon. Each task returns its local subtree; a single-threaded merge then
//! copies everything into the arena, which keeps hard-link attribution
//! deterministic.

mod merge;
mod pseudo;
mod walker;

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::tree::Tree;

/// Scanner settings.
#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    /// Do not descend into directories on another filesystem (`-x`).
    pub one_file_system: bool,
}

/// Live counters shared between the scan and the progress screen.
#[derive(Debug, Default)]
pub struct Progress {
    entries: AtomicU64,
    bytes: AtomicU64,
    errors: AtomicU64,
    current: Mutex<PathBuf>,
}

/// Copy of the counters at one instant.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProgressSnapshot {
    /// Entries seen so far.
    pub entries: u64,
    /// Disk usage accumulated so far.
    pub bytes: u64,
    /// Errors so far.
    pub errors: u64,
    /// Directory being listed.
    pub current: PathBuf,
}

impl Progress {
    /// Reads the counters.
    pub fn snapshot(&self) -> ProgressSnapshot {
        ProgressSnapshot {
            entries: self.entries.load(Ordering::Relaxed),
            bytes: self.bytes.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
            current: self.current.lock().map(|p| p.clone()).unwrap_or_default(),
        }
    }

    fn add_entry(&self, disk: u64) {
        self.entries.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(disk, Ordering::Relaxed);
    }

    fn add_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    fn set_current(&self, path: &Path) {
        // Best effort: skip the update instead of waiting on contention.
        if let Ok(mut cur) = self.current.try_lock() {
            cur.clear();
            cur.push(path);
        }
    }
}

/// Why a scan did not produce a tree.
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    /// The cancel flag was raised.
    #[error("scan cancelled")]
    Cancelled,
    /// The root path cannot be resolved or read.
    #[error("cannot read {path}: {source}")]
    Root {
        /// Path given by the user.
        path: PathBuf,
        /// Underlying error.
        source: std::io::Error,
    },
}

/// Scans `root` and returns its tree, sorted by nothing (filesystem order).
///
/// `cancel` is polled before every directory; raising it makes the scan
/// stop early and return [`ScanError::Cancelled`].
pub fn scan(
    root: &Path,
    opts: &ScanOptions,
    progress: &Progress,
    cancel: &AtomicBool,
) -> Result<Tree, ScanError> {
    let root_err = |source| ScanError::Root {
        path: root.to_path_buf(),
        source,
    };
    let root = std::fs::canonicalize(root).map_err(root_err)?;
    let md = std::fs::symlink_metadata(&root).map_err(root_err)?;
    let ctx = walker::Ctx {
        opts,
        progress,
        cancel,
        root_dev: std::os::unix::fs::MetadataExt::dev(&md),
    };
    let run = || walker::walk_root(&ctx, &root, &md);
    let (meta, children) = match pool() {
        Some(pool) => pool.install(run),
        None => run(),
    };
    if cancel.load(Ordering::Relaxed) {
        return Err(ScanError::Cancelled);
    }
    Ok(merge::build(root, meta, children))
}

/// Dedicated pool with roomy stacks: recursion depth follows directory depth.
fn pool() -> Option<rayon::ThreadPool> {
    rayon::ThreadPoolBuilder::new()
        .stack_size(16 * 1024 * 1024)
        .thread_name(|i| format!("ramas-scan-{i}"))
        .build()
        .ok()
}

#[cfg(test)]
mod tests;
