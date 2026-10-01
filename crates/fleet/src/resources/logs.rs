//! Every log file a Job has, what each weighs, and whether a writer holds it.
//!
//! **"Being written" is an open file, never a recent mtime**, which cannot tell
//! a Drone paused for ten minutes from one that died. It is `lsof`'s answer: a
//! process holding the file open with write access.
//!
//! **Fleet is asked as well as the Job's tree, because Fleet is the writer.** A
//! Drone never opens its transcript: `crate::transcript::Recording` appends its
//! rows from Fleet's process, holding the transcript and the Job's log open
//! until the Drone's stdout closes. Asking only the tree would read `false` on
//! every live Drone. Fleet's own reads are read-only and are not writers, and a
//! kept brief is written whole and closed (`crate::asked`).
//!
//! Bounded by [`LOOK`] and shelled out, [`crate::headroom`]'s precedent.

use std::collections::BTreeSet;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use ipc::{LogFile, LogKind};
use tokio::process::Command;

use crate::asked::briefs_dir;
use crate::resources::LOOK;
use crate::transcript::{log_of, transcripts_dir};

/// One file found, before anything is asked about it.
struct Found {
    kind: LogKind,
    at: PathBuf,
    /// Size and inode, or nothing where the file would not `stat`.
    stat: Option<(u64, u64)>,
}

/// Every log the Job has, its own first, then transcripts, then briefs, each
/// set in name order — a transcript is named by a ULID, so that is the order
/// its Drones ran in.
///
/// `tree` is the Job's processes, [`ipc::JobResources::processes`]'s pids.
pub(crate) async fn of(records_root: &str, handle: &str, tree: &[u32]) -> Vec<LogFile> {
    let mut found: Vec<Found> = Vec::new();
    let own = log_of(records_root, handle);
    if own.symlink_metadata().is_ok() {
        found.push(found_at(LogKind::Job, own));
    }
    for (kind, dir) in [
        (LogKind::Transcript, transcripts_dir(records_root, handle)),
        (LogKind::Brief, briefs_dir(records_root, handle)),
    ] {
        found.extend(listed(&dir).into_iter().map(|at| found_at(kind, at)));
    }
    if found.is_empty() {
        return Vec::new();
    }
    let mut asked: Vec<u32> = tree.to_vec();
    asked.push(std::process::id());
    let measured: Vec<&Path> = found
        .iter()
        .filter(|one| one.stat.is_some())
        .map(|one| one.at.as_path())
        .collect();
    let writers = held_for_writing(&asked, &measured).await;
    found
        .into_iter()
        .map(|one| LogFile {
            kind: one.kind,
            path: relative(records_root, &one.at),
            bytes: one.stat.map(|(bytes, _)| bytes),
            // A file that would not `stat` has no inode to match, so nothing
            // was learned about it even where `lsof` answered.
            being_written: match (&writers, one.stat) {
                (Some(inodes), Some((_, inode))) => Some(inodes.contains(&inode)),
                _ => None,
            },
        })
        .collect()
}

fn found_at(kind: LogKind, at: PathBuf) -> Found {
    // `metadata` follows a link, so a link to nothing lists with no size.
    let stat = std::fs::metadata(&at)
        .ok()
        .map(|held| (held.len(), held.ino()));
    Found { kind, at, stat }
}

/// The entries of one directory, in name order, or none where it is not there.
fn listed(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_ok_and(|kind| !kind.is_dir()))
        .map(|entry| entry.path())
        .collect();
    paths.sort();
    paths
}

/// Relative to `records_root`, the spelling `Judged::brief_path` uses.
fn relative(records_root: &str, at: &Path) -> String {
    at.strip_prefix(records_root)
        .unwrap_or(at)
        .to_string_lossy()
        .into_owned()
}

/// The inodes among `files` that one of `pids` holds open for writing, or
/// nothing where `lsof` did not answer inside [`LOOK`].
///
/// **Matched by inode rather than by name**, because `lsof` prints the path it
/// resolved — `/private/var/…` for a `/var/…` it was handed on darwin.
///
/// **The exit status is not read.** `lsof` exits 1 both where it found nothing
/// and where a file was removed between the listing and the call, and prints
/// what it did find either way, so stdout is the answer.
async fn held_for_writing(pids: &[u32], files: &[&Path]) -> Option<BTreeSet<u64>> {
    if files.is_empty() {
        return Some(BTreeSet::new());
    }
    let pids: Vec<String> = pids.iter().map(u32::to_string).collect();
    let said = Command::new("lsof")
        .args(["-w", "-a", "-p", &pids.join(","), "-Fai", "--"])
        .args(files)
        .kill_on_drop(true)
        .output();
    let out = tokio::time::timeout(LOOK, said).await.ok()?.ok()?;
    Some(writing(&String::from_utf8_lossy(&out.stdout)))
}

/// The inodes `lsof -Fai` lists with write access, `w` or `u`.
///
/// Each open file is an `f` line, then its `a`ccess and its `i`node, in that
/// order; a `p` line starts the next process.
pub(crate) fn writing(said: &str) -> BTreeSet<u64> {
    let mut held = BTreeSet::new();
    let mut writable = false;
    for line in said.lines() {
        match line.split_at_checked(1) {
            Some(("f", _)) | Some(("p", _)) => writable = false,
            Some(("a", access)) => writable = matches!(access, "w" | "u"),
            Some(("i", inode)) if writable => {
                if let Ok(inode) = inode.parse() {
                    held.insert(inode);
                }
            }
            _ => {}
        }
    }
    held
}
