//! A snoozed Job keeps working and stays quiet until its time: Pocket pushes nothing for it and
//! the Operator raises nothing. `docs/concepts/session/snoozing-a-job.md`.
//!
//! The snooze is a row, so a Fleet that restarts meanwhile still wakes it.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::Timestamp;
use ipc::{SessionRecord, SnoozeSession, WireError};

use crate::daemon::Fleet;
use crate::session_host::NO_SUCH_SESSION;

/// An ended Job, which has nothing left to raise.
const SESSION_NOT_SNOOZABLE: &str = "fleet.session_not_snoozable";
/// An `until` that is not an RFC3339 instant.
const SNOOZE_UNREADABLE: &str = "fleet.snooze_unreadable";

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
    /// `snooze_session`: snooze the Job until `until`, or wake it now with none. **Refused, nothing
    /// changes.** Waking a Job that is not snoozed answers its row.
    pub(crate) async fn session_snoozed(&self, asked: SnoozeSession) -> Result<SessionRecord, Refusal> {
        let id = asked.session_id.as_str().to_string();
        if let Some(until) = &asked.until {
            if Timestamp::from_rfc3339(until.as_str().to_string()).epoch_millis().is_none() {
                return Err(self.hosted_refusal(SNOOZE_UNREADABLE, &format!("{} is not a time", until.as_str())));
            }
        }
        {
            let mut store = self.store().lock().await;
            let session = store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(NO_SUCH_SESSION, &format!("no job is named {id}")))?;
            if asked.until.is_some() && session.state == store::SessionState::Ended {
                return Err(Refusal::IllegalMove(WireError::raised(
                    SESSION_NOT_SNOOZABLE,
                    "This Job has ended, so it cannot be snoozed.",
                    self.run_id(),
                )));
            }
            store
                .keep_session_snooze(&id, asked.until.as_ref().map(|until| until.as_str()))
                .map_err(|why| self.ledger_fault(why))?;
        }
        self.published_hosted(&id).await
    }

    /// Wake each Job whose snooze has ended. **A tick of the notice loop calls it.** A time nothing
    /// can read wakes the Job, so nothing it raised stays hidden.
    pub(crate) async fn snoozes_ended(&self) {
        // A store that will not answer is asked again on the next tick.
        let Ok(snoozed) = self.store().lock().await.sessions_snoozed() else {
            return;
        };
        let Some(now) = self.now().epoch_millis() else {
            return;
        };
        for (id, until) in snoozed {
            if Timestamp::from_rfc3339(until).epoch_millis().is_some_and(|due| due > now) {
                continue;
            }
            if matches!(self.store().lock().await.keep_session_snooze(&id, None), Ok(true)) {
                let _ = self.published_hosted(&id).await;
            }
        }
    }
}
