//! A hosted Job waiting on the owner, compacted just before its prompt cache runs out. A trial,
//! shipped off. `docs/concepts/session/where-the-tokens-went.md`, *A Job compacted while it waits*.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use config::settings as keys;
use core_model::Timestamp;
use ipc::SessionTurn;

use crate::daemon::Fleet;

/// What the Job is sent. The harness reads it as its own command, not as words for the agent.
pub(crate) const COMPACT: &str = adapters::COMPACT_COMMAND;
/// Most of the cache's hour: a sweep at most 30 seconds apart and a resume still land inside it.
pub(crate) const COMPACT_AFTER: Duration = Duration::from_secs(50 * 60);
/// The context under which a cold start costs too little to be worth a compaction.
pub(crate) const COMPACT_FROM: u64 = 100_000;

/// One Job as the sweep reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Waiting {
    /// It has an item on its waiting list.
    pub(crate) on_the_owner: bool,
    /// Since its last main-thread turn.
    pub(crate) quiet: Duration,
    /// That turn's input, cache-creation and cache-read tokens.
    pub(crate) context: u64,
    /// A compaction is kept after that turn.
    pub(crate) compacted: bool,
}

/// Whether the sweep sends the Job [`COMPACT`] now.
pub(crate) fn compacts(on: bool, job: Waiting) -> bool {
    on && job.on_the_owner && job.quiet >= COMPACT_AFTER && job.context >= COMPACT_FROM && !job.compacted
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
    /// Every hosted Job with no turn running that [`compacts`], sent [`COMPACT`] and kept. **Awaited
    /// in the sweep, never spawned**, so the next sweep reads the compaction this one kept. **Boxed**,
    /// because the send can start a process, which starts the sweep that calls this.
    pub(crate) fn compacted_while_waiting(self: &Arc<Self>) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(self.compacting())
    }

    async fn compacting(self: &Arc<Self>) {
        if !self.machine_settings().now().get(keys::COMPACT_WHILE_WAITING) {
            return;
        }
        let idle: Vec<String> = self
            .hosts()
            .all()
            .into_iter()
            .filter(|(_, runtime)| {
                let state = runtime.state();
                matches!(state.turn, SessionTurn::Idle) && state.asked.is_none() && state.start_failed.is_none()
            })
            .map(|(id, _)| id)
            .collect();
        for id in idle {
            let Some(job) = self.compaction_read(&id).await else { continue };
            if !compacts(true, job) {
                continue;
            }
            let at = self.now().as_str().to_string();
            if self.told_to_compact(&id).await.unwrap_or(false) {
                let _ = self.store().lock().await.keep_waiting_compaction(&id, &at, job.context);
            }
        }
    }

    /// How a live hosted Job that is not paused stands for [`compacts`], with the waiting list read
    /// last since it is the dearest. `None` where it is no such Job or has no main-thread turn.
    async fn compaction_read(&self, id: &str) -> Option<Waiting> {
        let store = self.store().lock().await;
        let hosting = store.hosting(id).ok().flatten()?;
        if hosting.paused_at.is_some() || hosting.start_held {
            return None;
        }
        let session = store.session(id).ok().flatten().filter(|one| one.state == store::SessionState::Live)?;
        let (turn_at, context) = store.last_main_context(id).ok().flatten()?;
        let since = Timestamp::from_rfc3339(turn_at.as_str()).epoch_millis()?;
        let now = self.now().epoch_millis()?;
        let quiet = Duration::from_millis(now.saturating_sub(since).max(0) as u64);
        if quiet < COMPACT_AFTER || context < COMPACT_FROM {
            return Some(Waiting { on_the_owner: false, quiet, context, compacted: false });
        }
        let compacted = store.waiting_compactions(id).ok()?.iter().any(|one| one.at > turn_at);
        let on_the_owner = !compacted && self.ledger_row(&store, &session).is_ok_and(|record| !record.waiting_for.is_empty());
        Some(Waiting { on_the_owner, quiet, context, compacted })
    }
}
