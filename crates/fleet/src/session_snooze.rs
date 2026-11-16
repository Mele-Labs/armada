//! A snoozed Job is paused and stays quiet until its time: Pocket pushes nothing for it and the
//! Operator raises nothing. When it wakes, Fleet resumes it if the snooze was what paused it.
//! `docs/concepts/session/snoozing-a-job.md`.
//!
//! The snooze is a row, so a Fleet that restarts meanwhile still wakes and resumes it. The row keeps
//! the pause marker the snooze set, and the wake resumes only a Job whose marker is still that one.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::Timestamp;
use ipc::{PauseSession, ResumeSession, SessionId, SessionRecord, SnoozeSession, WireError};

use crate::daemon::Fleet;
use crate::session_host::NO_SUCH_SESSION;

/// An ended Job, which has nothing left to raise.
const SESSION_NOT_SNOOZABLE: &str = "fleet.session_not_snoozable";
/// An `until` that is not an RFC3339 instant.
const SNOOZE_UNREADABLE: &str = "fleet.snooze_unreadable";

/// What a Job with no turn kept is told when its snooze ends and it is resumed.
const SNOOZE_ENDED: &str = "You were snoozed, and the snooze has ended. Carry on with what you were doing.";

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
    /// `snooze_session`: snooze the Job until `until` and pause it, or wake it now with none and
    /// resume it if the snooze paused it. **Refused, nothing changes**, but for a pause that ends the
    /// turn before the pool refuses the park, as `pause_session` is. Waking a Job that is not snoozed
    /// answers its row.
    pub(crate) async fn session_snoozed(self: &Arc<Self>, asked: SnoozeSession) -> Result<SessionRecord, Refusal> {
        let id = asked.session_id.as_str().to_string();
        let Some(until) = &asked.until else {
            let (paused_at, woken) = {
                let mut store = self.store().lock().await;
                let paused_at = store.snooze_paused_at(&id).map_err(|why| self.ledger_fault(why))?;
                (paused_at, store.keep_session_snooze(&id, None).map_err(|why| self.ledger_fault(why))?)
            };
            if woken {
                self.snooze_lifted(&id, paused_at).await;
            }
            return self.published_hosted(&id).await;
        };
        if Timestamp::from_rfc3339(until.as_str().to_string()).epoch_millis().is_none() {
            return Err(self.hosted_refusal(SNOOZE_UNREADABLE, &format!("{} is not a time", until.as_str())));
        }
        {
            let store = self.store().lock().await;
            let session = store
                .session(&id)
                .map_err(|why| self.ledger_fault(why))?
                .ok_or_else(|| self.hosted_refusal(NO_SUCH_SESSION, &format!("no job is named {id}")))?;
            if session.state == store::SessionState::Ended {
                return Err(Refusal::IllegalMove(WireError::raised(
                    SESSION_NOT_SNOOZABLE,
                    "This Job has ended, so it cannot be snoozed.",
                    self.run_id(),
                )));
            }
        }
        let paused_at = self.paused_for_snooze(&asked.session_id).await?;
        let mut store = self.store().lock().await;
        store.keep_session_snooze(&id, Some(until.as_str())).map_err(|why| self.ledger_fault(why))?;
        if let Some(paused_at) = paused_at {
            store.keep_snooze_paused_at(&id, &paused_at).map_err(|why| self.ledger_fault(why))?;
        }
        drop(store);
        self.published_hosted(&id).await
    }

    /// Pause the Job as `pause_session` does, and answer the marker that pause kept. **None** for a
    /// Job the person paused himself, and for one that runs in a terminal, which only he can stop.
    /// A Job whose Resume waits for a slot is paused again, its wait called off.
    async fn paused_for_snooze(&self, session_id: &SessionId) -> Result<Option<String>, Refusal> {
        let id = session_id.as_str();
        if self.terminal_session(id).await?.is_some() {
            return Ok(None);
        }
        let hosting = self.store().lock().await.hosting(id).map_err(|why| self.ledger_fault(why))?;
        match hosting {
            None => return Ok(None),
            Some(hosting) if hosting.paused_at.is_some() && !hosting.paused_resuming => return Ok(None),
            Some(_) => {}
        }
        self.session_paused(PauseSession { session_id: session_id.clone() }).await?;
        let hosting = self.store().lock().await.hosting(id).map_err(|why| self.ledger_fault(why))?;
        Ok(hosting.and_then(|hosting| hosting.paused_at))
    }

    /// The snooze of `id` has ended: resume it if the snooze paused it and it is still that pause.
    /// A Job the person resumed meanwhile, or paused again himself, is left as he left it.
    async fn snooze_lifted(self: &Arc<Self>, id: &str, paused_at: Option<String>) {
        let Some(paused_at) = paused_at else {
            return;
        };
        let still_ours = {
            let store = self.store().lock().await;
            let live = store.session(id).ok().flatten().is_some_and(|one| one.state != store::SessionState::Ended);
            live && store.hosting(id).ok().flatten().is_some_and(|one| one.paused_at.as_deref() == Some(paused_at.as_str()))
        };
        if !still_ours {
            return;
        }
        let asked = ResumeSession { session_id: SessionId::carried(id) };
        if let Err(why) = self.session_resumed_telling(asked, SNOOZE_ENDED).await {
            self.session_warned(id, "a job its snooze paused was not resumed when the snooze ended", format!("{why:?}"));
        }
    }

    /// Wake each Job whose snooze has ended, and resume the ones the snooze paused. **A tick of the
    /// notice loop calls it.** A time nothing can read wakes the Job, so nothing it raised stays hidden.
    pub(crate) async fn snoozes_ended(self: &Arc<Self>) {
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
            let woken = {
                let mut store = self.store().lock().await;
                let paused_at = store.snooze_paused_at(&id).ok().flatten();
                store.keep_session_snooze(&id, None).ok().filter(|woken| *woken).map(|_| paused_at)
            };
            if let Some(paused_at) = woken {
                self.snooze_lifted(&id, paused_at).await;
                let _ = self.published_hosted(&id).await;
            }
        }
    }
}
