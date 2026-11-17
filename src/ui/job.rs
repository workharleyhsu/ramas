//! Background scan running in its own thread.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};

use crate::scan::{Progress, ProgressSnapshot, ScanError, ScanOptions, scan};
use crate::tree::{NodeId, Tree};

/// A scan in progress.
pub struct ScanJob {
    /// Path being scanned.
    pub root: PathBuf,
    /// Node replaced by the result (`None` = whole tree).
    pub target: Option<NodeId>,
    progress: Arc<Progress>,
    cancel: Arc<AtomicBool>,
    rx: Receiver<Result<Tree, ScanError>>,
    started: Instant,
}

impl ScanJob {
    /// Starts scanning `root` on a new thread.
    pub fn start(root: PathBuf, opts: ScanOptions, target: Option<NodeId>) -> Self {
        let progress = Arc::new(Progress::default());
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        let (p, c, r) = (progress.clone(), cancel.clone(), root.clone());
        std::thread::spawn(move || {
            let _ = tx.send(scan(&r, &opts, &p, &c));
        });
        ScanJob {
            root,
            target,
            progress,
            cancel,
            rx,
            started: Instant::now(),
        }
    }

    /// Asks the scan to stop.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// The result, once the scan thread is done.
    pub fn try_result(&self) -> Option<Result<Tree, ScanError>> {
        match self.rx.try_recv() {
            Ok(res) => Some(res),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(ScanError::Cancelled)),
        }
    }

    /// Current counters.
    pub fn snapshot(&self) -> ProgressSnapshot {
        self.progress.snapshot()
    }

    /// Time since start.
    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_delivers_tree() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("f"), b"hello").unwrap();
        let job = ScanJob::start(tmp.path().to_path_buf(), ScanOptions::default(), None);
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(res) = job.try_result() {
                assert_eq!(res.unwrap().root().files, 1);
                break;
            }
            assert!(Instant::now() < deadline, "scan timed out");
            std::thread::yield_now();
        }
    }
}
