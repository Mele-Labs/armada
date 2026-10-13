//! A Drone's edit calls, read off its stream as they are heard.

use std::sync::Arc;

use adapter_traits::{AgentHarness, DroneEvent};
use ipc::{Saw, Voice};

use super::{Who, Writes};
use crate::clock::Clock;
use crate::edit_calls::repository_relative;
use crate::transcript::Tap;

/// Puts every file a Drone's edit call names into the table. **Never awaits**, as a tap may not:
/// the table is a short-held lock.
pub(crate) struct WritesSeen<H> {
    pub(crate) harness: Arc<H>,
    pub(crate) writes: Arc<Writes>,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) who: Who,
    pub(crate) manifest: String,
    pub(crate) worktree: String,
    pub(crate) home: String,
}

impl<H: AgentHarness + Send + Sync> Tap for WritesSeen<H> {
    fn saw(&self, events: &[DroneEvent]) {
        // A path outside the worktree is no part of this repository's tree.
        let inside: Vec<String> = events
            .iter()
            .filter_map(|event| self.harness.edited(event))
            .map(|path| repository_relative(&path, &self.worktree, &self.home))
            .filter(|path| !path.starts_with('/'))
            .collect();
        if !inside.is_empty() {
            self.writes.wrote(&self.who, &self.manifest, inside, &self.clock.now());
        }
    }

    fn noted(&self, _by: Voice, _saw: Saw) {}
}

/// Paths a terminal's mod named from its `cwd`, as the worktree names them. A path that climbs out
/// of the directory, or is absolute, is not one the mod should have sent and is dropped.
pub(crate) fn inside_worktree(worktree: &str, cwd: &str, paths: &[String]) -> Vec<String> {
    let from = cwd.trim_end_matches('/').strip_prefix(worktree.trim_end_matches('/'));
    let Some(prefix) = from.map(|rest| rest.trim_matches('/')) else {
        return Vec::new();
    };
    paths
        .iter()
        .filter(|path| !path.starts_with('/') && !path.split('/').any(|part| part == ".."))
        .map(|path| match prefix.is_empty() {
            true => path.trim_start_matches("./").to_string(),
            false => format!("{prefix}/{}", path.trim_start_matches("./")),
        })
        .collect()
}
