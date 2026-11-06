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
use api::HostedSessions;
use store::KeptStop;

use crate::daemon::Fleet;

/// How long a stop stands before Fleet says anything, and how long a nudge has to start a turn. A
/// minute, and nothing measured it; the week's read after landing does.
pub const STOP_AFTER: Duration = Duration::from_secs(60);

/// How often every live Job is read again, for an excuse that lapsed with nothing published.
pub const SWEEP: Duration = Duration::from_secs(5 * 60);

/// How often Fleet writes that it is up, and how long past that it was away rather than busy.
pub const BEAT: Duration = Duration::from_secs(15);
pub const OUTAGE_SECONDS: i64 = 90;

/// The word to a Job stopped with no step of its own to finish.
pub const NUDGE: &str = "Fleet: your turn ended with nothing running and nothing asked. Carry on now. If you wait on the person, \
     name it with waiting_for. If nothing is left, call all_done. A second stop like this one calls the \
     person and marks this Job Stopped.";

/// Where a Job with no excuse stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Read {
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
pub fn read(stop: &str, kept: &KeptStop, spoke: Option<&str>, now: i64) -> Read {
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
pub fn stopped(stop: &str, record: &ipc::SessionRecord) -> ipc::SessionStopped {
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

/// What Fleet's note in the thread begins with when a process ends.
pub const PROCESS_ENDED: &str = "The job's process ended";

/// Its process died with the turn open and Fleet's one restart did not bring it back.
pub fn restart_spent(turn: &store::KeptOpenTurn) -> bool {
    turn.ended_at.is_some() && turn.started_again >= 1
}

/// What the row says of a Job dead with its restart spent: a repair Job is at it, or Fleet has run out.
/// `away` is the outages its open turn overlapped. A repair Job gets no repair of its own.
pub fn dead(
    turn: &store::KeptOpenTurn,
    kept: &KeptStop,
    repair_over: bool,
    away: &[(String, String)],
    said: Option<String>,
) -> ipc::SessionStopped {
    let repair = match (&kept.repair_job, repair_over) {
        (Some(_), true) => Some(ipc::StopRepair::Failed),
        (Some(_), false) => Some(ipc::StopRepair::Running),
        (None, _) => kept.repair_held.then_some(ipc::StopRepair::Held),
    };
    let down = away.last().map(|(from, to)| ipc::FleetAway { from: ipc::Instant::carried(from), to: ipc::Instant::carried(to) });
    ipc::SessionStopped {
        reason: if down.is_some() { ipc::StopReason::FleetDown } else { ipc::StopReason::ProcessGone },
        at: ipc::Instant::carried(turn.ended_at.as_deref().unwrap_or(&turn.since)),
        tasks: Vec::new(),
        said,
        restart: ipc::StopRestart::Failed,
        repair,
        gave_up: kept.repairs.is_some() || matches!(repair, Some(ipc::StopRepair::Failed | ipc::StopRepair::Held)),
        down,
    }
}

/// The Jobs whose stop is owed a word, by when it ended and the last thing that could have moved it, so
/// the tick reads no Job it has nothing to say to.
#[derive(Default)]
pub struct Owed(std::sync::Mutex<HashMap<String, (String, i64)>>);

impl Owed {
    /// What a read of the Job found. `None` is nothing owed.
    pub fn noted(&self, session: &str, owed: Option<(&str, i64)>) {
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
    pub fn ripe(&self, now: i64) -> Vec<String> {
        let held = self.0.lock().expect("held across no panic");
        held.iter().filter(|(_, (_, from))| now - from >= STOP_AFTER.as_millis() as i64).map(|(id, _)| id.clone()).collect()
    }

    pub fn stop_of(&self, session: &str) -> Option<String> {
        self.0.lock().expect("held across no panic").get(session).map(|(stop, _)| stop.clone())
    }
}

impl Fleet
{
    /// Every live Job read again, at start and every [`SWEEP`], so a stop is noticed that no publish
    /// would say: one standing across a restart, or one whose excuse lapsed quietly.
    pub async fn stops_swept(self: &Arc<Self>) {
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
            if self.stop_read_again(&id).await.is_none() {
                continue;
            }
            let Some(stop) = self.hosts().stops_owed().stop_of(&id) else { continue };
            let told = self.noticed_by_fleet(&id, NUDGE, ipc::Notice::StopNudge).await;
            let heard = matches!(told, Ok(told) if told.sent());
            let at = self.now().as_str().to_string();
            let _ = self.store().lock().await.keep_stop_told(&id, &stop, &at, !heard);
            self.hosts().stops_owed().noted(&id, None);
            let _ = self.published_hosted(&id).await;
        }
        self.repairs_started().await;
    }

    /// A Job dead with its restart spent gets a repair Job, one at a time and never for a repair Job,
    /// unless Fleet itself is unhealthy, when a new Job would fail the same way and he is told instead.
    async fn repairs_started(self: &Arc<Self>) {
        let (dead, running) = {
            let store = self.store().lock().await;
            let turns = store.open_turns().unwrap_or_default();
            let dead: Vec<String> = turns
                .iter()
                .filter(|turn| restart_spent(turn))
                .filter(|turn| self.hosts().of(&turn.session_id).state().process.is_none())
                .filter(|turn| store.session_stop(&turn.session_id).is_ok_and(|kept| kept.repair_job.is_none() && kept.repairs.is_none()))
                .filter(|turn| store.session(&turn.session_id).ok().flatten().is_some_and(|one| one.state == store::SessionState::Live))
                .map(|turn| turn.session_id.clone())
                .collect();
            (dead, store.repair_running().unwrap_or(true))
        };
        let Some(id) = dead.into_iter().next().filter(|_| !running) else { return };
        let healthy = self.health().await.is_ok_and(|health| !health.probes.iter().any(|one| one.outcome == "fail"));
        let held = self.store().lock().await.session_stop(&id).is_ok_and(|kept| kept.repair_held);
        if !healthy {
            if !held {
                let _ = self.store().lock().await.keep_repair_held(&id, true);
                let _ = self.published_hosted(&id).await;
            }
            return;
        }
        if self.repair_started(&id).await.is_err() {
            let _ = self.store().lock().await.keep_repair_held(&id, true);
            let _ = self.published_hosted(&id).await;
        }
    }

    /// Start a Job in the same repository, nested under the dead one, with no start card: he asked to press nothing.
    async fn repair_started(self: &Arc<Self>, id: &str) -> Result<(), api::Refusal> {
        let (session, hosting) = self.session_and_hosting(id).await?;
        let title = session.title.clone().unwrap_or_else(|| id.to_string());
        let manifest = ipc::ManifestId::carried(&hosting.manifest_id);
        let made = self
            .start_session(ipc::StartSession {
                manifest_id: manifest,
                title: Some(format!("Repair: {title}")),
                model: None,
                effort: None,
                mode: None,
                root: None,
                fork: None,
                request: None,
                after: None,
                held: false,
                project_rows: Vec::new(),
                on_your_words: None,
            })
            .await?;
        let repair = made.id.as_str().to_string();
        self.started_linked(id, &repair, &hosting.manifest_id, None).await?;
        self.store().lock().await.keep_repair_started(id, &repair).map_err(|why| self.ledger_fault(why))?;
        let brief = format!(
            "Job {id}, \"{title}\", lost its process during a turn, and Fleet's one restart did not bring it back. Find why from \
             its log and what its process last said, fix what stopped it, then send it a message so it carries on. If it cannot \
             be brought back, name what it needs with waiting_for. Then call all_done."
        );
        Arc::clone(self)
            .send_session_message(ipc::SendSessionMessage { session_id: made.id.clone(), text: brief, attachments: Vec::new(), mentions: Vec::new() })
            .await?;
        self.fleet_note(id, format!("Started a repair Job, {}", format_args!("Repair: {title}"))).await;
        let _ = self.published_hosted(id).await;
        Ok(())
    }
}
