//! A session the plan's limit stopped is woken when the limit resets. `docs/concepts/session.md`,
//! *A session that hit the plan's limit*.
//!
//! The wake is the one Fleet already has for its own words to a session (`told_by_fleet`): a hosted
//! session's process is started or fed a turn, and a terminal session's mod, which asks Fleet for
//! what it holds, submits it. The hold is a row, so a Fleet that restarts meanwhile still wakes it.

use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::Timestamp;

use crate::daemon::Fleet;

/// What the session is told when the limit has reset.
const WAKE: &str = "Continue";

/// How long past the reset the wake waits, so the window is surely open when the turn starts.
const MARGIN: Duration = Duration::from_secs(60);

/// How far past its wake time a session is still tried for. A terminal that stopped listening or a
/// process that will not start is left to the person after this.
const GIVE_UP: Duration = Duration::from_secs(30 * 60);

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
    /// The session took a turn, or was spoken to, so the wake is not owed. Whether it was held.
    pub(crate) async fn limit_cleared(&self, id: &str) -> bool {
        self.store().lock().await.clear_session_limit(id).unwrap_or(false)
    }

    /// Wake each session whose limit has reset, with `Continue`. **A tick of the notice loop calls
    /// it.** A session that ended is let go; one that could not be told yet is tried again on the
    /// next tick, until it is long overdue.
    pub(crate) async fn limits_lifted(self: &std::sync::Arc<Self>) {
        // A store that will not answer is asked again on the next tick.
        let Ok(held) = self.store().lock().await.sessions_limited() else {
            return;
        };
        let Some(now) = self.now().epoch_millis() else {
            return;
        };
        for (id, resets_at) in held {
            let Some(due) = Timestamp::from_rfc3339(resets_at).epoch_millis() else {
                // A reset nothing can read is no reset to wait for.
                self.limit_cleared(&id).await;
                continue;
            };
            let overdue = now - due - MARGIN.as_millis() as i64;
            if overdue < 0 {
                continue;
            }
            let woken = matches!(self.told_by_fleet(&id, WAKE, ipc::Notice::LimitWake).await, Ok(true));
            let ended = self
                .store()
                .lock()
                .await
                .session(&id)
                .map(|one| one.is_none_or(|one| one.state == store::SessionState::Ended))
                .unwrap_or(false);
            if woken || ended || overdue > GIVE_UP.as_millis() as i64 {
                self.limit_cleared(&id).await;
                let _ = self.published_hosted(&id).await;
            }
        }
    }
}
