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

/// How often Fleet writes that it is up, and how long past that it was away rather than busy.
pub(crate) const BEAT: Duration = Duration::from_secs(15);
pub(crate) const OUTAGE_SECONDS: i64 = 90;

/// The word to a Job stopped with no step of its own to finish.
pub(crate) const NUDGE: &str = "Fleet: your turn ended with nothing running and nothing asked. Carry on now. If you wait on the person, \
     name it with waiting_for. If nothing is left, call all_done. A second stop like this one calls the \
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

/// When the latest work ended that wakes the agent as it ends: a background task, a subagent, a pull
/// request. The agent is told of that end a moment after the row says it, so a stop is not ripe before then.
pub(crate) fn woken_work_ended(held: &[store::KeptAttachment]) -> Option<i64> {
    let wakes = |kind: &str| kind == super::backgrounding::KIND || kind == "subagent" || kind == crate::pull_requesting::PR;
    held.iter().filter(|one| wakes(&one.kind) && one.state != store::AttachmentState::Standing).filter_map(|one| millis(&one.changed_at)).max()
}

/// Where the repair of a stop stands, from what is kept of it.
fn repair_of(kept: &KeptStop, repair_over: bool) -> Option<ipc::StopRepair> {
    match (&kept.repair_job, repair_over) {
        (Some(_), true) => Some(ipc::StopRepair::Failed),
        (Some(_), false) => Some(ipc::StopRepair::Running),
        (None, _) => kept.repair_held.then_some(ipc::StopRepair::Held),
    }
}

/// Whether a repair for a stop, silent or on a red pull request, has run since the person last wrote.
/// **One between two things he says**: a repair that put things right wakes the Job or pushes its branch,
/// and a Job that then stops again would otherwise draw another, so Fleet and a Job could loop at the cost
/// of a repair each time round.
pub(crate) fn stop_repair_spent(last: Option<&str>, spoke: Option<&str>) -> bool {
    let Some(last) = last.and_then(millis) else { return false };
    spoke.and_then(millis).is_none_or(|spoke| last > spoke)
}

/// What the row says of a Job that stopped with its agent alive: why, and what Fleet is doing about it.
/// `repair_due` is a repair for the stop still to come, which only a Job with its agent alive and no
/// repair since he last wrote has: with none, only he moves it now.
pub(crate) fn stopped(stop: &str, record: &ipc::SessionRecord, kept: &KeptStop, repair_over: bool, repair_due: bool) -> ipc::SessionStopped {
    let tasks: Vec<String> = record
        .activity
        .iter()
        .flat_map(|activity| &activity.todos)
        .filter(|one| one.state != ipc::SessionTodoState::Done)
        .map(|one| one.text.clone())
        .collect();
    let repair = repair_of(kept, repair_over);
    ipc::SessionStopped {
        reason: if tasks.is_empty() { ipc::StopReason::TurnEnded } else { ipc::StopReason::TasksOpen },
        at: ipc::Instant::carried(stop),
        tasks,
        said: None,
        restart: ipc::StopRestart::None,
        repair,
        gave_up: match repair {
            Some(ipc::StopRepair::Running) => false,
            Some(ipc::StopRepair::Failed | ipc::StopRepair::Held) => true,
            None => !repair_due,
        },
        down: None,
    }
}

/// What Fleet's note in the thread begins with when a process ends.
pub(crate) const PROCESS_ENDED: &str = "The job's process ended";

/// Its process died with the turn open and Fleet's one restart did not bring it back.
pub(crate) fn restart_spent(turn: &store::KeptOpenTurn) -> bool {
    turn.ended_at.is_some() && turn.started_again >= 1
}

/// What the row says of a Job dead with its restart spent: a repair is at it, or Fleet has run out.
/// `away` is the outages its open turn overlapped.
pub(crate) fn dead(
    turn: &store::KeptOpenTurn,
    kept: &KeptStop,
    repair_over: bool,
    away: &[(String, String)],
    said: Option<String>,
) -> ipc::SessionStopped {
    let repair = repair_of(kept, repair_over);
    let down = away.last().map(|(from, to)| ipc::FleetAway { from: ipc::Instant::carried(from), to: ipc::Instant::carried(to) });
    ipc::SessionStopped {
        reason: if down.is_some() { ipc::StopReason::FleetDown } else { ipc::StopReason::ProcessGone },
        at: ipc::Instant::carried(turn.ended_at.as_deref().unwrap_or(&turn.since)),
        tasks: Vec::new(),
        said,
        restart: ipc::StopRestart::Failed,
        repair,
        gave_up: matches!(repair, Some(ipc::StopRepair::Failed | ipc::StopRepair::Held)),
        down,
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

/// The Jobs read Stopped with their agent alive and a repair still to come: the stop, and the pull
/// request Fleet told them failed where one has not been pushed to since.
#[derive(Default)]
pub(crate) struct Silent(std::sync::Mutex<HashMap<String, (String, Option<u64>)>>);

impl Silent {
    /// What a read of the Job found. `None` is no stop to repair.
    pub(crate) fn noted(&self, session: &str, stop: Option<&str>, red: Option<u64>) {
        let mut held = self.0.lock().expect("held across no panic");
        match stop {
            Some(stop) => {
                held.insert(session.to_string(), (stop.to_string(), red));
            }
            None => {
                held.remove(session);
            }
        }
    }

    pub(crate) fn ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.0.lock().expect("held across no panic").keys().cloned().collect();
        ids.sort();
        ids
    }

    pub(crate) fn stop_of(&self, session: &str) -> Option<(String, Option<u64>)> {
        self.0.lock().expect("held across no panic").get(session).cloned()
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
    pub(crate) async fn stop_read_again(&self, id: &str) -> Option<ipc::SessionRecord> {
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

    /// A Job dead with its restart spent, or stopped again with its agent alive, gets a hidden repair, one
    /// at a time across Fleet, unless Fleet itself is unhealthy, when a new agent would fail the same way
    /// and he is told instead. A dead Job's goes first.
    async fn repairs_started(self: &Arc<Self>) {
        if self.repairs().running_any() {
            return;
        }
        let dead: Vec<String> = {
            let store = self.store().lock().await;
            let turns = store.open_turns().unwrap_or_default();
            turns
                .iter()
                .filter(|turn| restart_spent(turn))
                .filter(|turn| self.hosts().of(&turn.session_id).state().process.is_none())
                .filter(|turn| store.session_stop(&turn.session_id).is_ok_and(|kept| kept.repair_job.is_none()))
                .filter(|turn| store.session(&turn.session_id).ok().flatten().is_some_and(|one| one.state == store::SessionState::Live))
                .map(|turn| turn.session_id.clone())
                .collect()
        };
        if let Some(id) = dead.into_iter().next() {
            let saw = self.died_seen(&id).await;
            self.repair_tried(&id, crate::repair::RepairBreak::Died, saw).await;
            return;
        }
        if let Some((id, stop, red)) = self.silent_stop().await {
            let mut saw = Vec::new();
            let broke = match red {
                Some(number) => {
                    saw.push((String::from("Pull request"), format!("#{number}")));
                    crate::repair::RepairBreak::RedPull { number }
                }
                None => crate::repair::RepairBreak::Quiet,
            };
            if let Some(log) = adapters::terminal_thread::find(&self.host().home, &id) {
                saw.push((String::from("Its transcript"), log.display().to_string()));
            }
            saw.push((String::from("Its last turn ended"), stop));
            self.repair_tried(&id, broke, saw).await;
        }
    }

    /// The first Job read Stopped with its agent alive that nothing waits on and no repair has run for,
    /// read again so that an excuse that came since is seen, with the stop and its red pull request.
    async fn silent_stop(self: &Arc<Self>) -> Option<(String, String, Option<u64>)> {
        for id in self.hosts().stops_silent().ids() {
            let unrepaired = self.store().lock().await.session_stop(&id).is_ok_and(|kept| kept.repair_job.is_none());
            if !unrepaired || self.hosts().of(&id).state().process.is_none() || self.stop_read_again(&id).await.is_none() {
                continue;
            }
            if let Some((stop, red)) = self.hosts().stops_silent().stop_of(&id) {
                return Some((id, stop, red));
            }
        }
        None
    }

    /// Start the repair for `id`. A refusal is held, as it always was, and an unhealthy Fleet says so once.
    pub(crate) async fn repair_tried(self: &Arc<Self>, id: &str, broke: crate::repair::RepairBreak, saw: Vec<(String, String)>) {
        let held = self.store().lock().await.session_stop(id).is_ok_and(|kept| kept.repair_held);
        if let Err(why) = self.repair_started(id, broke, saw).await {
            let unhealthy = matches!(&why, api::Refusal::Unacceptable(raised) if raised.code == crate::repair::REPAIR_UNHEALTHY);
            if !held || !unhealthy {
                let _ = self.store().lock().await.keep_repair_held(id, true);
                let _ = self.published_hosted(id).await;
            }
        }
    }

    /// What Fleet saw of a Job whose process died: where its log is, and what its process last said.
    async fn died_seen(&self, id: &str) -> Vec<(String, String)> {
        let mut saw = Vec::new();
        if let Some(log) = adapters::terminal_thread::find(&self.host().home, id) {
            saw.push((String::from("Its log"), log.display().to_string()));
        }
        if let Some(said) = self.stop_read_again(id).await.and_then(|record| record.stopped).and_then(|stopped| stopped.said) {
            saw.push((String::from("What its process last said"), said));
        }
        saw
    }
}
