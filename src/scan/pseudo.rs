//! Detection of pseudo filesystems that must not be scanned.

use std::path::Path;

/// Paths always skipped when met below the scan root.
const SKIP_PATHS: &[&str] = &["/proc", "/sys", "/dev"];

/// `statfs.f_type` magics of kernel pseudo filesystems.
const PSEUDO_MAGICS: &[u64] = &[
    0x9fa0,      // proc
    0x6265_6572, // sysfs
    0x1cd1,      // devpts
    0x0027_e0eb, // cgroup
    0x6367_7270, // cgroup2
    0x6462_6720, // debugfs
    0x7472_6163, // tracefs
    0x7363_6673, // securityfs
    0x6165_676c, // pstore
    0xcafe_4a11, // bpf
    0x6265_6570, // configfs
    0xde5e_81e4, // efivarfs
    0x1980_0202, // mqueue
    0x9584_58f6, // hugetlbfs
    0x6573_5543, // fusectl
    0x4249_4e4d, // binfmt_misc
];

/// Whether the directory at `path` is a pseudo filesystem. The (costly)
/// `statfs` call only happens on mount points, signalled by `dev_changed`.
pub(super) fn is_pseudo(path: &Path, dev_changed: bool) -> bool {
    if SKIP_PATHS.iter().any(|p| path == Path::new(p)) {
        return true;
    }
    dev_changed && fs_magic(path).is_some_and(|m| PSEUDO_MAGICS.contains(&m))
}

fn fs_magic(path: &Path) -> Option<u64> {
    rustix::fs::statfs(path).ok().map(|s| s.f_type as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_paths_are_skipped() {
        assert!(is_pseudo(Path::new("/proc"), false));
        assert!(is_pseudo(Path::new("/sys"), false));
        assert!(is_pseudo(Path::new("/dev"), false));
        assert!(!is_pseudo(Path::new("/home"), false));
    }

    #[test]
    fn magic_detects_proc_mount() {
        // /proc is a mount point on every Linux system.
        assert_eq!(fs_magic(Path::new("/proc")), Some(0x9fa0));
    }
}
