//! What a release does about a slot's `target/`: Cargo never deletes a stale
//! artifact, so a slot kept warm for months grew to 50 GB. A release drops what
//! no build has written in a while, then the incremental caches and, if that is
//! not enough, the directory.
//! A release does it for its own slot, and Fleet's sweep does it for every build
//! folder of every checkout of a repository; both go through [`trim_target`].
//! What Fleet does on top of it for a free slot's age and for a short disk is `disk`.
//! The settings are `slot-build-trim-after-days`, `slot-build-ceiling-gib` and
//! `build-sweep-interval-minutes`.

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use super::disk::{build_folders_in, every_checkout};

/// Days a file under `target/` may go unwritten before a release drops it.
pub const TRIM_AFTER_DAYS: u64 = 14;

/// Gibibytes `target/` may hold after the stale files are gone before a
/// release removes it whole.
pub const CEILING_GIB: u64 = 20;

/// How a release trims a slot's build directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trim {
    pub older_than: Duration,
    pub ceiling_bytes: u64,
}

impl Trim {
    pub const SHIPPED: Trim = Trim {
        older_than: Duration::from_secs(TRIM_AFTER_DAYS * 24 * 60 * 60),
        ceiling_bytes: CEILING_GIB * 1024 * 1024 * 1024,
    };
}

/// What one trim did with a `target/`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trimmed {
    /// There was no `target/`.
    Absent,
    /// A cargo build holds its lock, so nothing was touched.
    Building,
    /// Stale files, and incremental caches where it took them, were dropped and
    /// what is left is under the ceiling.
    Swept,
    /// What was left was over the ceiling, so the directory went whole.
    Removed,
}

/// Trim `target`, best effort: a file that will not go stays. **Skipped while
/// a build runs in it**: cargo holds an advisory lock on each profile
/// directory's `.cargo-lock` for the length of a build, so the trim takes every
/// one of them first and keeps them until it is done. That is also what makes
/// it safe against a build starting between the look and the delete, since
/// cargo then waits on the trim.
pub fn trim_target(target: &Path, trim: Trim, now: SystemTime) -> Trimmed {
    if !target.is_dir() {
        return Trimmed::Absent;
    }
    let Some(_held) = locks(target) else {
        return Trimmed::Building;
    };
    let cutoff = now
        .checked_sub(trim.older_than)
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let kept = sweep(target, cutoff, true);
    if kept > trim.ceiling_bytes {
        remove_incremental(target);
        if size_of(target) > trim.ceiling_bytes {
            let _ = fs::remove_dir_all(target);
            return Trimmed::Removed;
        }
    }
    Trimmed::Swept
}

/// Every `incremental` directory under `target`, removed. **The cheap thing to
/// lose**: it is most of a slot's bytes, and without it the next change compiles
/// its own crates in full and no third-party crate again.
pub(super) fn remove_incremental(target: &Path) {
    let mut dirs = vec![target.to_path_buf()];
    for _ in 0..3 {
        let mut next = Vec::new();
        for dir in &dirs {
            let Ok(entries) = fs::read_dir(dir) else {
                continue;
            };
            for entry in entries.flatten() {
                if !entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                    continue;
                }
                if entry.file_name() == "incremental" {
                    let _ = fs::remove_dir_all(entry.path());
                } else {
                    next.push(entry.path());
                }
            }
        }
        dirs = next;
    }
}

/// Every `.cargo-lock` under `target` (the profile directories, with or
/// without a target triple above them), locked; `None` where any is held.
pub(super) fn locks(target: &Path) -> Option<Vec<fs::File>> {
    let mut held = Vec::new();
    let mut dirs = vec![target.to_path_buf()];
    for depth in 0..3 {
        let mut next = Vec::new();
        for dir in &dirs {
            let Ok(entries) = fs::read_dir(dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if entry.file_name() == ".cargo-lock" {
                    let file = fs::File::options()
                        .read(true)
                        .write(true)
                        .open(&path)
                        .ok()?;
                    file.try_lock().ok()?;
                    held.push(file);
                } else if depth < 2 && entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                    next.push(path);
                }
            }
        }
        dirs = next;
    }
    Some(held)
}

/// [`trim_target`] on every build folder of every checkout of `root` — see
/// [`super::disk::every_checkout`] and [`super::disk::build_folders_in`] — each with what it did.
///
/// **`held` are checkouts a person is working in** (a piloted Job's), and their
/// build folders are not looked at: a person's build output is not stale because
/// no Drone wrote it.
pub fn sweep_repository(
    root: &Path,
    trim: Trim,
    now: SystemTime,
    held: &[std::path::PathBuf],
) -> Vec<(std::path::PathBuf, Trimmed)> {
    let held: Vec<_> = held.iter().filter_map(|checkout| checkout.canonicalize().ok()).collect();
    every_checkout(root)
        .into_iter()
        .filter(|checkout| !held.contains(checkout))
        .flat_map(|checkout| build_folders_in(&checkout))
        .map(|target| {
            let done = trim_target(&target, trim, now);
            (target, done)
        })
        .collect()
}

/// Remove files under `dir` last written before `cutoff`, and directories that
/// leave empty, except `dir` itself where `top`. Returns the bytes left.
fn sweep(dir: &Path, cutoff: SystemTime, top: bool) -> u64 {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    let mut left = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.is_dir() {
            if is_build_script_output(&path) {
                left += size_of(&path);
                continue;
            }
            left += sweep(&path, cutoff, false);
            continue;
        }
        if meta.modified().is_ok_and(|written| written < cutoff) && fs::remove_file(&path).is_ok() {
            continue;
        }
        left += meta.len();
    }
    if !top {
        // Fails while anything is left in it, which is the point.
        let _ = fs::remove_dir(dir);
    }
    left
}

/// A build script's `out/` (`target/<profile>/build/<unit>/out`). **Never
/// trimmed file by file**: cargo does not list what a build script wrote among
/// a unit's outputs, so a missing `bindgen.rs` reads as built and the next
/// compile fails on it. `libsqlite3-sys` broke restarts six times on 8 Oct
/// 2026 this way.
fn is_build_script_output(dir: &Path) -> bool {
    dir.file_name().is_some_and(|name| name == "out")
        && dir
            .parent()
            .and_then(Path::parent)
            .and_then(Path::file_name)
            .is_some_and(|name| name == "build")
}

/// The bytes under `dir`, counted toward the ceiling.
pub(super) fn size_of(dir: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let meta = fs::symlink_metadata(entry.path()).ok()?;
            Some(if meta.is_dir() { size_of(&entry.path()) } else { meta.len() })
        })
        .sum()
}
