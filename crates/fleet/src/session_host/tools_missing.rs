//! What a hosted Job's agent came up without, read off its process's `init` line.
//! `docs/concepts/session/a-job-that-came-up-without-its-tools.md`.
//!
//! **The harness says what it loaded before it takes the turn**, so a plugin that did not load, a
//! door that did not connect or a CLI with no task tools is visible on the first line and nowhere
//! else. Fleet keeps the reading in memory, publishes what is missing, and restarts the process
//! once for the two it can mend.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, HarnessReading, Vcs, WorkProduct};
use api::Refusal;
use ipc::{SessionRecord, SessionTurn, ToolMissing, ToolsMissing};

use super::Runtime;
use crate::daemon::Fleet;

/// A job that runs in a terminal, whose process Fleet does not host. A 422.
pub(crate) const SESSION_TOOLS_UNMENDABLE: &str = "fleet.session_tools_unmendable";
/// Fleet's copy of the mod could not be filled, so a restart would come up without it. A 422.
pub(crate) const SESSION_MOD_NOT_FILLED: &str = "fleet.session_mod_not_filled";

/// What a process with nothing unfinished to send again is told, so the restart has a turn to carry it.
const MENDED: &str = "Fleet restarted your process with the armada mod. Nothing is asked of you.";

/// Who asked for a restart onto Fleet's copy of the mod.
pub(super) enum Restart<'a> {
    /// The person pressed Restart the agent, and the process is let go whatever it is doing.
    Pressed,
    /// Fleet, because the mod the process reported is older than the copy: only between turns.
    Moved { to: &'a str, from: &'a str },
}

impl Restart<'_> {
    /// What the replacement is told when it has no unfinished line to carry the restart.
    fn word(&self) -> String {
        match self {
            Restart::Pressed => MENDED.to_string(),
            Restart::Moved { to, from } => format!("Fleet restarted your process onto armada mod {to} from {from}. Nothing is asked of you."),
        }
    }
}

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
                    .then(|| ToolsMissing { harness: reading.name.clone(), update: reading.update.clone(), harness_version: reading.version.clone(), missing });
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

    /// `mend_session_tools`: Fleet's copy of the mod filled, then the Job's process restarted so it
    /// loads it. **Answers once the restart is under way**, and clears no mark: the next `init` does.
    ///
    /// The lines the old process had not finished go to its replacement, as `tools_healed` sends them,
    /// with no once-guard since a person asked. A Job with none has no turn to carry the restart, so
    /// the replacement is started by a word of Fleet's. **A Job waiting to be moved into a slot, or
    /// paused, is left as it is**: the move and the resume start a process, which loads the filled copy.
    pub(crate) async fn tools_mended(self: &Arc<Self>, asked: ipc::MendSessionTools) -> Result<SessionRecord, Refusal> {
        let id = asked.session_id.as_str();
        if self.terminal_session(id).await?.is_some() {
            return Err(self.hosted_refusal(
                SESSION_TOOLS_UNMENDABLE,
                &format!("job {id} runs in a terminal, and Fleet restarts the agent of a job it hosts only"),
            ));
        }
        let (session, hosting) = self.session_and_hosting(id).await?;
        if session.state == store::SessionState::Ended {
            return Err(self.closed(id));
        }
        self.mod_filled().await.map_err(|said| {
            self.hosted_refusal(SESSION_MOD_NOT_FILLED, &format!("the armada mod's copy could not be filled: {said}"))
        })?;
        let runtime = self.hosts().of(id);
        self.restarted_onto_mod(id, &runtime, hosting.paused_at.is_some(), Restart::Pressed).await;
        self.published_hosted(id).await
    }

    /// Let the process go and start its replacement, which loads the copy of the mod as it is now.
    /// **The one restart onto the copy**, whoever asks for it. Whether it restarted: a Job waiting to
    /// be moved into a slot or paused is left as it is, and so is one a turn has begun in when Fleet
    /// itself asked, since that turn is not Fleet's to cut.
    pub(super) async fn restarted_onto_mod(self: &Arc<Self>, id: &str, runtime: &Arc<Runtime>, paused: bool, why: Restart<'_>) -> bool {
        let lines = {
            let mut state = runtime.state();
            let waits = state.moving.is_some() || paused || (matches!(why, Restart::Moved { .. }) && !state.between_turns());
            if waits {
                None
            } else {
                if let Some(asked) = state.asked.take() {
                    self.hosts().asks().withdraw(&asked.call);
                }
                let lines = std::mem::take(&mut state.unstarted);
                Self::let_go(&mut state);
                // The replacement is Fleet's own, so a piece it still lacks is marked and not restarted for again.
                state.healed_into = Some(state.generation);
                if let Restart::Moved { to, .. } = &why {
                    state.moved_onto = Some((*to).to_string());
                }
                state.turn = SessionTurn::Idle;
                state.restart_after_turn = false;
                state.reattached_busy = false;
                Some(lines)
            }
        };
        let Some(lines) = lines else { return false };
        self.calls_let_go(id).await;
        if lines.is_empty() {
            let _ = self.told_of_a_mend(id, &why.word()).await;
        } else {
            self.send_again(id, runtime, lines).await;
        }
        true
    }
}
