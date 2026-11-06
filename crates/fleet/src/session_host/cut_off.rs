//! Subagents a process would take down with it, and those a replaced one did. `docs/concepts/session.md`,
//! *What a restart does to a session*.
//!
//! **A subagent lives in its agent's process.** A keeper carries the process across a Fleet
//! restart, and no new process can carry it. So a quiet spell and the move into a slot wait for
//! what is still working, and a process Fleet ends anyway (a tune) or loses (the keeper gone) takes
//! every subagent with it, and the one that replaces it has never heard of them.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::Timestamp;
use store::{AttachmentState, Holder, KeptAttachment};

use super::State;
use crate::daemon::Fleet;

/// How long work may show nothing before it stops holding its process: a subagent since its
/// transcript's last line, a background task since it started, as it writes nowhere Fleet reads.
/// **An hour, and nothing measured it.** Longer than any one tool call of a subagent, and a server
/// left running gives its process up in the end.
const HUNG_AFTER: Duration = Duration::from_secs(60 * 60);

impl Fleet
{
    /// Whether the Job has a subagent or a background task working, which ending its process takes
    /// down. The turn is over, so this is work the agent left running. With no process there is
    /// nothing to take down, whatever its rows still say.
    pub(super) async fn work_beyond_the_turn(&self, id: &str) -> bool {
        let held = self.store().lock().await.attachments_of(&Holder::session(id)).unwrap_or_default();
        !self.running_of(id, &held).is_empty()
    }

    /// Each subagent and background task of the Job still working, off `held`, its attachments: what
    /// kind, what it is and since when. **The one read of what is running**, so `work_beyond_the_turn`
    /// and what the listings and the Operator say of a Job are the same rule. Takes the attachments
    /// rather than the store, so a caller holding the store's lock reads it too.
    pub(crate) fn running_of(&self, id: &str, held: &[KeptAttachment]) -> Vec<ipc::RunningWork> {
        if self.hosts().of(id).state().process.is_none() {
            return Vec::new();
        }
        held.iter()
            .filter(|one| one.state == AttachmentState::Standing && self.still_working(id, one))
            .map(|one| {
                let said = |key: &str| one.detail.get(key).map(String::as_str).filter(|text| !text.trim().is_empty());
                let (kind, what) = match one.kind.as_str() {
                    "subagent" => (ipc::RunningKind::Subagent, said("description").or(said("type"))),
                    _ => (ipc::RunningKind::BackgroundTask, said("task").or(said("command"))),
                };
                ipc::RunningWork { kind, what: what.unwrap_or(&one.target).to_string(), since: ipc::Instant::carried(one.since.as_str()) }
            })
            .collect()
    }

    fn still_working(&self, id: &str, one: &KeptAttachment) -> bool {
        let home = &self.host().home;
        let last = match one.kind.as_str() {
            "subagent" if adapters::terminal_thread::subagent_ended(home, id, &one.target) => return false,
            "subagent" => adapters::terminal_thread::subagent_working_at(home, id, &one.target).unwrap_or_else(|| one.since.clone()),
            super::backgrounding::KIND => one.since.clone(),
            _ => return false,
        };
        let now = self.now().epoch_millis();
        now.zip(Timestamp::from_rfc3339(last).epoch_millis()).is_some_and(|(now, last)| now - last < HUNG_AFTER.as_millis() as i64)
    }

    /// Make each move that waited on working agents, once they have ended. **Called on the quiet
    /// sweep's timer, and by a test**, which would not wait for one. A turn's end makes the move
    /// itself, so this is for agents that end with no turn after them. **Boxed**, as
    /// `processes_moved_onto_mod` is: the move starts a process, whose start spawns the timer.
    pub(crate) fn held_moves_made(self: &Arc<Self>) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        let fleet = Arc::clone(self);
        Box::pin(async move {
            for (id, runtime) in fleet.hosts().all() {
                let due = runtime.state().move_due();
                if !due || fleet.work_beyond_the_turn(&id).await {
                    continue;
                }
                // Read again under the lock: a turn may have begun since, and its end moves it.
                let moving = {
                    let mut state = runtime.state();
                    if state.move_due() { state.moving.take() } else { None }
                };
                if let Some(moving) = moving {
                    fleet.moved_into(&id, moving).await;
                }
            }
        })
    }

    /// The subagents the session's last process had not finished, one line each. **Called
    /// where a new process starts for a session that has run**, so nothing it names can still be
    /// working. Their ledger rows are given back: they were let go before they were used.
    pub(crate) async fn cut_off_subagents(&self, id: &str) -> Vec<String> {
        let holder = Holder::session(id);
        let at = self.now().as_str().to_string();
        let mut store = self.store().lock().await;
        let held = store.attachments_of(&holder).unwrap_or_default();
        let mut cut = Vec::new();
        for one in held
            .iter()
            .filter(|one| one.kind == "subagent" && one.state == AttachmentState::Standing)
            .filter(|one| !adapters::terminal_thread::subagent_ended(&self.host().home, id, &one.target))
        {
            let _ = store.settle(
                &holder,
                &one.kind,
                &one.manifest_id,
                &one.target,
                AttachmentState::GivenBack,
                &at,
            );
            let said = |key: &str| one.detail.get(key).map(String::as_str).unwrap_or_default();
            cut.push(format!("{} \"{}\" ({})", said("type"), said("description"), one.target));
        }
        cut
    }
}

/// What the retro keeps of a restart that cut subagents off.
pub(super) fn cut_off_said(cut: &[String]) -> Option<String> {
    (!cut.is_empty()).then(|| format!("cut off: {}", cut.join("; ")))
}

/// `turn`, with what the last restart cut off ahead of it, once.
pub(super) fn told(state: &mut State, turn: String) -> String {
    if state.cut_off.is_empty() {
        return turn;
    }
    let cut = std::mem::take(&mut state.cut_off);
    format!(
        "This job's process was replaced while these subagents or background tasks were running. They stopped \
         and their work is lost: {}. Start again any that is still needed.\n\n{turn}",
        cut.join("; ")
    )
}
