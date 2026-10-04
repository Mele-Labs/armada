//! A directory that exists for one test and is gone after it.
//!
//! The runtime file's whole subject is a real path with a real file at it, so
//! nothing here is faked in memory: a test that never wrote a file could not
//! tell a removal-on-drop from a removal that never happened.
//!
//! **It also ends every process group spawned inside it.** A Drone is
//! `setsid`-detached so it outlives Fleet, which in a test means it outlives
//! the test: on 4 Oct 2026, 22 orphaned test Drones polling for a flag in a
//! directory already removed were forking `sleep` 1,100 times a second, and
//! froze the machine. `crate::Detached::spawn` hands each test child to
//! [`spawned_in`], and the drop below signals every group whose directory lies
//! under this one, before the directory goes. A test cannot opt out, because it
//! never calls anything to opt in.
//!
//! **The pid is not proved still ours, and the gap is stated.** POSIX keeps a
//! pid from being reused while a process group with that id exists, so the
//! signal can only reach a stranger if the whole group has died and the pid
//! counter has wrapped and been taken by a new group leader, all inside one
//! test's life. `crate::holder_of` would close that, at a `ps` per spawn on
//! every run of the suite.

use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Every group a test spawned with a directory, and that directory, resolved.
static SPAWNED: Mutex<Vec<(NonZeroU32, PathBuf)>> = Mutex::new(Vec::new());

/// Record a detached child, so the [`TempDir`] it runs in can end its group.
pub(crate) fn spawned_in(group: NonZeroU32, directory: &Path) {
    let directory = resolved(directory);
    SPAWNED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push((group, directory));
}

/// `/var` is `/private/var` on macOS, and a spawn may name either.
fn resolved(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// A directory under the system temp dir, removed on drop.
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn new() -> TempDir {
        let path = std::env::temp_dir().join(format!(
            "armada-fleet-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).expect("a temporary directory");
        TempDir { path }
    }

    /// The directory itself. A Drone's worktree stands in as one of these: the
    /// spawn needs a directory that is really there, and nothing about it needs
    /// to be a checkout.
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Where the runtime file under test lives.
    pub fn runtime_file(&self) -> PathBuf {
        self.path.join(crate::runtime::FILE_NAME)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let root = resolved(&self.path);
        let inside: Vec<NonZeroU32> = {
            let mut spawned = SPAWNED.lock().unwrap_or_else(PoisonError::into_inner);
            let (inside, outside) = std::mem::take(&mut *spawned)
                .into_iter()
                .partition(|(_, directory)| directory.starts_with(&root));
            *spawned = outside;
            inside.into_iter().map(|(group, _)| group).collect()
        };
        for group in inside {
            crate::group::end_the_group(group);
        }
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
