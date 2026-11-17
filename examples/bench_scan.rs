//! Simple scan benchmark: `cargo run --release --example bench_scan -- [PATH] [RUNS]`.
//!
//! Prints wall time per run, entries per second and the arena footprint.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use ramas::scan::{Progress, ScanOptions, scan};
use ramas::tree::SortKey;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = PathBuf::from(args.next().unwrap_or_else(|| "/usr".into()));
    let runs: usize = args.next().and_then(|r| r.parse().ok()).unwrap_or(3);
    for run in 1..=runs {
        let progress = Progress::default();
        let start = Instant::now();
        let mut tree = scan(
            &path,
            &ScanOptions::default(),
            &progress,
            &AtomicBool::new(false),
        )?;
        let scanned = start.elapsed();
        let start = Instant::now();
        tree.sort(SortKey::Size, false);
        let sorted = start.elapsed();
        let n = tree.len();
        println!(
            "run {run}: {n} entries, scan {:.3}s ({:.0} entries/s), sort {:.3}s, arena ~{:.1} MiB, errors {}",
            scanned.as_secs_f64(),
            n as f64 / scanned.as_secs_f64(),
            sorted.as_secs_f64(),
            tree.mem_bytes() as f64 / (1024.0 * 1024.0),
            tree.root().errors,
        );
    }
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    if let Some(l) = status.lines().find(|l| l.starts_with("VmHWM:")) {
        println!("peak RSS: {}", l.trim_start_matches("VmHWM:").trim());
    }
    Ok(())
}
