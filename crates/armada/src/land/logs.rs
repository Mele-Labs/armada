//! Where a turn's logs go: `logs/<entry>/<turn>/`, one directory per gate,
//! so a rerun never overwrites the turn that went red. Turns older than
//! [`KEPT_FOR`] are pruned. `docs/capabilities/merge-line.md`, *A turn's logs*.

use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use fleet::clock::{Clock, SystemClock};

use super::dir::{key, StateDir};
use super::queue::QueueEntry;

/// Two weeks: long enough to read a red found days later, and at about
/// 0.3 MB a turn the directory stays in the hundreds of megabytes.
pub const KEPT_FOR: Duration = Duration::from_secs(14 * 24 * 60 * 60);

/// A fresh directory for one gate of `group`, named by when it started, under
/// a single branch's own directory or one for the group.
pub fn turn_logs(state: &StateDir, group: &[QueueEntry]) -> io::Result<PathBuf> {
    let names: Vec<&str> = group.iter().map(|entry| entry.branch.as_str()).collect();
    fresh(&state.path().join("logs").join(key(&names.join("\n"))))
}

/// A new directory under `entry`, named by now; a suffix where two start in
/// the same millisecond.
fn fresh(entry: &Path) -> io::Result<PathBuf> {
    std::fs::create_dir_all(entry)?;
    // `:` is a path separator to Finder.
    let started = SystemClock::new().now().as_str().replace(':', "-");
    for n in 0.. {
        let name = match n {
            0 => started.clone(),
            n => format!("{started}-{n}"),
        };
        let at = entry.join(name);
        match std::fs::create_dir(&at) {
            Ok(()) => return Ok(at),
            Err(why) if why.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(why) => return Err(why),
        }
    }
    unreachable!("an unbounded range ends only by returning")
}

/// Remove every turn under `logs` last written more than `kept_for` ago, and
/// any entry left empty. Best effort: a turn that cannot be removed now is
/// tried again on the next.
pub fn prune(logs: &Path, kept_for: Duration) {
    let Ok(entries) = std::fs::read_dir(logs) else {
        return;
    };
    for entry in entries.flatten() {
        let entry = entry.path();
        let Ok(turns) = std::fs::read_dir(&entry) else {
            continue;
        };
        for turn in turns.flatten() {
            let old = turn
                .metadata()
                .and_then(|held| held.modified())
                .ok()
                .and_then(|at| at.elapsed().ok())
                .is_some_and(|age| age > kept_for);
            if old {
                let path = turn.path();
                // Entries from before turns had directories hold bare files.
                let _ = if path.is_dir() {
                    std::fs::remove_dir_all(&path)
                } else {
                    std::fs::remove_file(&path)
                };
            }
        }
        // Fails, as it should, while a turn is still inside.
        let _ = std::fs::remove_dir(&entry);
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::*;
    use crate::tests::TempDir;

    fn aged(path: &Path, by: Duration) {
        std::fs::File::open(path)
            .and_then(|held| held.set_modified(SystemTime::now() - by))
            .expect("a test directory takes a back-dated mtime");
    }

    #[test]
    fn prune_keeps_recent_turns_and_drops_old_ones_and_empty_entries() {
        let logs = TempDir::new();
        let old_turn = logs.path().join("aaaa/2026-09-01T00-00-00.000Z");
        let new_turn = logs.path().join("aaaa/2026-10-01T00-00-00.000Z");
        let alone = logs.path().join("bbbb/2026-09-02T00-00-00.000Z");
        let flat = logs.path().join("cccc/test.log");
        for dir in [&old_turn, &new_turn, &alone] {
            std::fs::create_dir_all(dir).expect("a turn directory");
        }
        std::fs::create_dir_all(flat.parent().expect("an entry")).expect("an entry");
        std::fs::write(&flat, "from before turns").expect("a flat log");
        let month = Duration::from_secs(30 * 24 * 60 * 60);
        for path in [&old_turn, &alone, &flat] {
            aged(path, month);
        }

        prune(logs.path(), KEPT_FOR);

        assert!(!old_turn.exists());
        assert!(new_turn.exists());
        assert!(
            !logs.path().join("bbbb").exists(),
            "an entry left empty goes"
        );
        assert!(
            !logs.path().join("cccc").exists(),
            "a flat log ages out too"
        );
    }

    #[test]
    fn two_turns_started_together_get_two_directories() {
        let entry = TempDir::new();
        let first = fresh(entry.path()).expect("a turn");
        let second = fresh(entry.path()).expect("a turn");
        assert_ne!(first, second);
        assert_eq!(first.parent(), second.parent(), "one entry, two turns");
    }
}
