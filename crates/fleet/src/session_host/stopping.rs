//! A Job that ended its turn with nothing running and nothing asked answers for it: Fleet tells it once
//! to carry on or name what it waits on, and if it stops again its row says Stopped.
//! `docs/concepts/session/what-a-job-is-doing.md`.
//!
//! **Read from what Fleet saw, never from the agent saying so.** A stop is named by when its turn ended
//! (`last_turn_at`), so a turn that ended on tools alone counts. Whatever excuses it is read by the
//! ledger, which holds every fact; this only says where a Job stands once none does. Pure, so each rule
//! is read and tested without a Fleet.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use store::KeptStop;

use crate::daemon::Fleet;

/// How long a stop stands before Fleet says anything, and how long a nudge has to start a turn. A
/// minute, and nothing measured it; the week's read after landing does.
pub(crate) const STOP_AFTER: Duration = Duration::from_secs(60);

/// How often every live Job is read again, for an excuse that lapsed with nothing published.
pub(crate) const SWEEP: Duration = Duration::from_secs(5 * 60);

/// The word to a Job stopped with no step of its own to finish.
pub(crate) const NUDGE: &str = "Fleet: your turn ended with nothing running and nothing asked. Carry on now. If you wait on the person, \
     name it with waiting_for. If nothing is left, call finish_up and then all_done. A second stop like this one calls the \
     person and marks this Job Stopped.";

/// Where a Job with no excuse stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Read {
    /// Nothing to say: passed over, or the nudge has not had its time.
    Quiet,
    /// It has not been told about this stop.
    Owed,
    /// It was told and stopped again, or the word reached nobody.
    Stopped,
}

fn millis(at: &str) -> Option<i64> {
    core_model::Timestamp::from_rfc3339(at).epoch_millis()
}

/// Where a Job stands whose turn ended at `stop` and that nothing excuses. **One nudge per stop**: the
/// stop after a nudge goes to Stopped, so a Job can never loop with Fleet, unless the person has written
/// since or something else woke it, which clears what was told.
pub(crate) fn read(stop: &str, kept: &KeptStop, spoke: Option<&str>, now: i64) -> Read {
    if kept.passed_over.as_deref() == Some(stop) {
        return Read::Quiet;
    }
    let Some((told_stop, told_at)) = &kept.told else {
        return Read::Owed;
    };
    if spoke.and_then(millis).zip(millis(told_at)).is_some_and(|(spoke, told)| spoke > told) {
        return Read::Owed;
    }
    match millis(stop).zip(millis(told_stop)) {
        Some((stop, told)) if stop > told => Read::Stopped,
        // Its turn has not begun: the word reached nobody, or it has had its time.
        _ if kept.unheard => Read::Stopped,
        _ if millis(told_at).is_some_and(|at| now - at > STOP_AFTER.as_millis() as i64) => Read::Stopped,
        _ => Read::Quiet,
    }
}

/// What the row says of a Job that stopped with its agent alive: why, and that only he moves it now,
/// since a repair Job would have nothing to fix.
pub(crate) fn stopped(stop: &str, record: &ipc::SessionRecord) -> ipc::SessionStopped {
    let tasks: Vec<String> = record
        .activity
        .iter()
        .flat_map(|activity| &activity.todos)
        .filter(|one| one.state != ipc::SessionTodoState::Done)
        .map(|one| one.text.clone())
        .collect();
    ipc::SessionStopped {
        reason: if tasks.is_empty() { ipc::StopReason::TurnEnded } else { ipc::StopReason::TasksOpen },
        at: ipc::Instant::carried(stop),
        tasks,
        said: None,
        restart: ipc::StopRestart::None,
        repair: None,
        gave_up: true,
        down: None,
    }
}

/// The Jobs whose stop is owed a word, by when it ended and the last thing that could have moved it, so
/// the tick reads no Job it has nothing to say to.
#[derive(Default)]
pub(crate) struct Owed(std::sync::Mutex<HashMap<String, (String, i64)>>);

impl Owed {
    /// What a read of the Job found. `None` is nothing owed.
    pub(crate) fn noted(&self, session: &str, owed: Option<(&str, i64)>) {
        let mut held = self.0.lock().expect("held across no panic");
        match owed {
            Some((stop, from)) => {
                held.insert(session.to_string(), (stop.to_string(), from));
            }
            None => {
                held.remove(session);
            }
        }
    }

    /// The Jobs whose minute is up.
    pub(crate) fn ripe(&self, now: i64) -> Vec<String> {
        let held = self.0.lock().expect("held across no panic");
        held.iter().filter(|(_, (_, from))| now - from >= STOP_AFTER.as_millis() as i64).map(|(id, _)| id.clone()).collect()
    }

    pub(crate) fn stop_of(&self, session: &str) -> Option<String> {
        self.0.lock().expect("held across no panic").get(session).map(|(stop, _)| stop.clone())
    }
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
    /// Every live Job read again, at start and every [`SWEEP`], so a stop is noticed that no publish
    /// would say: one standing across a restart, or one whose excuse lapsed quietly.
    pub(crate) async fn stops_swept(self: &Arc<Self>) {
        let live = self.store().lock().await.live_session_ids().unwrap_or_default();
        for id in live {
            let _ = self.stop_read_again(&id).await;
        }
    }

    /// The row read again, which notes whether its stop is owed. Published by nobody: nothing has moved.
    async fn stop_read_again(&self, id: &str) -> Option<ipc::SessionRecord> {
        let store = self.store().lock().await;
        let session = store.session(id).ok().flatten()?;
        self.ledger_row(&store, &session).ok()
    }

    /// Tell each Job whose stop is ripe, once.
    pub async fn stops_told(self: &Arc<Self>) {
        let now = self.now().epoch_millis().unwrap_or(0);
        for id in self.hosts().stops_owed().ripe(now) {
            // Read again: an excuse may have come since the last read.
            let Some(record) = self.stop_read_again(&id).await else { continue };
            let Some(stop) = self.hosts().stops_owed().stop_of(&id) else { continue };
            let said = self.stop_said(&record).await;
            let told = self.noticed_by_fleet(&id, &said, ipc::Notice::StopNudge).await;
            let heard = matches!(told, Ok(told) if told.sent());
            let at = self.now().as_str().to_string();
            let _ = self.store().lock().await.keep_stop_told(&id, &stop, &at, !heard);
            self.hosts().stops_owed().noted(&id, None);
            let _ = self.published_hosted(&id).await;
        }
    }
}
