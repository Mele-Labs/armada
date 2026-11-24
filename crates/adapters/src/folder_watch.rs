//! A folder watched for writes, so a reader that keeps what a file said reads it again when it changes.
//!
//! **Events are a hint, never the truth.** `heard` gets the paths an event named and nothing about what happened to
//! them, from the thread the file system delivers on, so it hands them on and returns; the reader debounces and
//! reads the files again. A watch that could not start is `None`, and the reader falls back to what it was told.
//! The pool watch in [`crate::leasing`] is the same stream with more to say about git.

use std::path::{Path, PathBuf};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// One folder, watched with everything under it, for as long as this is held.
pub struct FolderWatch {
    _watcher: RecommendedWatcher,
}

impl FolderWatch {
    /// Watch `folder`, which must exist. `heard` is told the paths of each event that made, changed or removed one.
    pub fn start(folder: &Path, heard: impl Fn(Vec<PathBuf>) + Send + 'static) -> Option<FolderWatch> {
        let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
            let Ok(event) = event else { return };
            if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)) && !event.paths.is_empty() {
                heard(event.paths);
            }
        })
        .ok()?;
        watcher.watch(folder, RecursiveMode::Recursive).ok()?;
        Some(FolderWatch { _watcher: watcher })
    }
}
