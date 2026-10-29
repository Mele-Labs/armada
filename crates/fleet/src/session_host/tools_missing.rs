//! What a hosted Job's agent came up without, read off its process's `init` line.
//! `docs/concepts/session/a-job-that-came-up-without-its-tools.md`.
//!
//! **The harness says what it loaded before it takes the turn**, so a plugin that did not load, a
//! door that did not connect or a CLI with no task tools is visible on the first line and nowhere
//! else. Fleet keeps the reading in memory, publishes what is missing, and restarts the process
//! once for the two it can mend.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, HarnessReading, Vcs, WorkProduct};
use ipc::{ToolMissing, ToolsMissing};

use super::Runtime;
use crate::daemon::Fleet;

/// What `reading` lacks, in the wire's order. Empty is nothing missing.
pub(crate) fn missing_of(reading: &HarnessReading) -> Vec<ToolMissing> {
    [
        (!reading.mod_loaded, ToolMissing::Mod),
        (!reading.door_connected, ToolMissing::Door),
        (!reading.task_tools, ToolMissing::TaskTools),
    ]
    .into_iter()
    .filter_map(|(lacks, piece)| lacks.then_some(piece))
    .collect()
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Whether a launch passes `--plugin-dir` now: the test `process.rs` makes when it builds one.
    fn launch_loads_mod(&self) -> bool {
        crate::runtime::mod_dir(&self.host().home)
            .join(adapters::MOD_INSTALLED_MANIFEST)
            .is_file()
    }

    /// A process said `init`. Keeps what it says it lacks, and where the door did not connect, or the
    /// mod did not load though the launch passed it, restarts the process **once** and sends it the
    /// lines it had not finished. Whether it did, which leaves the caller nothing to do for this `init`.
    ///
    /// **The restart is once per process Fleet started for it.** The replacement is marked by its
    /// generation, so its own failure keeps the mark and does not loop, and a process started later
    /// for any other reason is mended again.
    pub(crate) async fn tools_healed(self: &Arc<Self>, id: &str, runtime: &Arc<Runtime>, reading: &HarnessReading) -> bool {
        let missing = missing_of(reading);
        let mendable = missing.contains(&ToolMissing::Door)
            || (missing.contains(&ToolMissing::Mod) && self.launch_loads_mod());
        let lines = {
            let mut state = runtime.state();
            // A line to send again is the turn this `init` opened, so one that has none, or that waits
            // to be moved into a slot, is left as it is.
            let heal = mendable
                && state.healed_into != Some(state.generation)
                && state.process.is_some()
                && state.moving.is_none()
                && !state.unstarted.is_empty();
            if !heal {
                state.tools_missing = (!missing.is_empty())
                    .then(|| ToolsMissing { harness_version: reading.version.clone(), missing });
                return false;
            }
            let lines = std::mem::take(&mut state.unstarted);
            Self::let_go(&mut state);
            state.healed_into = Some(state.generation);
            lines
        };
        self.send_again(id, runtime, lines).await;
        let _ = self.published_hosted(id).await;
        true
    }
}
