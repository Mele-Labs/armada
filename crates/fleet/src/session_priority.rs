//! The person's one prioritised Job, whose builds a loaded machine runs while other Jobs' wait.
//!
//! The store keeps one id. Ending the Job leaves it there and the reader ignores it, so nothing has
//! to clear the mark on the many ways a Job ends.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{PrioritiseSession, SessionRecord, WireError};

use crate::daemon::Fleet;
use crate::session_host::NO_SUCH_SESSION;

/// An ended Job, which has no builds left to run.
const SESSION_NOT_PRIORITISABLE: &str = "fleet.session_not_prioritisable";

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
    /// `prioritise_session`: move the mark to this Job, or clear it where this Job holds it, and
    /// publish each row whose mark moved. **Refused, nothing changes**, for an ended Job.
    pub(crate) async fn session_prioritised(&self, asked: PrioritiseSession) -> Result<SessionRecord, Refusal> {
        let id = asked.session_id.as_str().to_string();
        let before = {
            let mut store = self.store().lock().await;
            let session = store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(NO_SUCH_SESSION, &format!("no job is named {id}")))?;
            if session.state == store::SessionState::Ended {
                return Err(Refusal::IllegalMove(WireError::raised(
                    SESSION_NOT_PRIORITISABLE,
                    "This Job has ended, so it cannot be prioritised.",
                    self.run_id(),
                )));
            }
            let before = store.prioritised_session().map_err(|why| self.ledger_fault(why))?;
            let after = (before.as_deref() != Some(id.as_str())).then_some(id.as_str());
            store.keep_prioritised_session(after).map_err(|why| self.ledger_fault(why))?;
            before
        };
        if let Some(other) = before.filter(|other| *other != id) {
            // The Job that lost the mark may have gone from the ledger; its row has nothing to redraw then.
            let _ = self.published_hosted(&other).await;
        }
        self.published_hosted(&id).await
    }

    /// The live prioritised Job's id. **None** when none is, when it has ended, or when the store
    /// will not answer.
    pub(crate) async fn prioritised_session_id(&self) -> Option<String> {
        let store = self.store().lock().await;
        let id = store.prioritised_session().ok().flatten()?;
        let live = store.session(&id).ok().flatten().is_some_and(|one| one.state != store::SessionState::Ended);
        live.then_some(id)
    }
}
