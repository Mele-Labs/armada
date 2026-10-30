//! A hosted Job's process that runs an older armada mod than Fleet's copy.
//! `docs/concepts/session/a-job-that-came-up-without-its-tools.md`.
//!
//! **A restart of Fleet refreshes the copy and leaves the processes it adopted on the mod they
//! loaded when they started**, and an adopted process reports its mod once, so no report says it
//! fell behind. The sweep that already walks every hosted process between turns asks it, and
//! restarts a process whose reported version is older, as the person's own press does.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use store::{AttachmentState, Holder};

use super::tools_missing::Restart;
use crate::daemon::Fleet;

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
    /// Restart each hosted process between turns whose mod is older than Fleet's copy. **Called on
    /// the quiet sweep's timer, and by a test**, which would not wait for one. A process with a turn
    /// running is asked again at the next call, so a turn is never cut. **Boxed**, as `listening` is:
    /// the restart starts a process, whose start spawns the timer that calls this, and a future that
    /// contains itself cannot be proved `Send`.
    pub(crate) fn processes_moved_onto_mod(self: &Arc<Self>) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        let fleet = Arc::clone(self);
        Box::pin(async move {
            let Some(current) = crate::sessioning::mod_copy_version(&fleet.host().home) else {
                return;
            };
            let between: Vec<String> = fleet
                .hosts()
                .all()
                .into_iter()
                .filter(|(_, runtime)| {
                    let state = runtime.state();
                    state.between_turns() && state.moved_onto.as_deref() != Some(current.as_str())
                })
                .map(|(id, _)| id)
                .collect();
            for id in between {
                fleet.process_moved_onto_mod(&id, &current).await;
            }
        })
    }

    /// One Job's process, restarted when its reported mod is older than `current`. **Once for each
    /// version of the copy**, so a replacement that reports an older one is left as it is. A Job that
    /// reported none is not read as older: its process may not have said so yet, and one that came up
    /// without the mod is `tools_healed`'s.
    async fn process_moved_onto_mod(self: &Arc<Self>, id: &str, current: &str) {
        // A terminal job has no hosting, so it is refused here with no more said.
        let Ok((session, hosting)) = self.session_and_hosting(id).await else {
            return;
        };
        let Some(had) = session
            .mod_version
            .as_deref()
            .filter(|had| crate::sessioning::mod_is_older(Some(had), current))
        else {
            return;
        };
        if session.state == store::SessionState::Ended || hosting.paused_at.is_some() || self.work_beyond_the_turn(id).await {
            return;
        }
        let runtime = self.hosts().of(id);
        if self.restarted_onto_mod(id, &runtime, false, Restart::Moved { to: current, from: had }).await {
            let _ = self.published_hosted(id).await;
        }
    }

    /// Whether the Job has a subagent or a background task working, which a restart takes down with
    /// the process. The turn is over, so this is work the agent left running.
    async fn work_beyond_the_turn(&self, id: &str) -> bool {
        let held = self.store().lock().await.attachments_of(&Holder::session(id)).unwrap_or_default();
        held.iter().filter(|one| one.state == AttachmentState::Standing).any(|one| match one.kind.as_str() {
            "subagent" => !adapters::terminal_thread::subagent_ended(&self.host().home, id, &one.target),
            super::backgrounding::KIND => true,
            _ => false,
        })
    }
}
