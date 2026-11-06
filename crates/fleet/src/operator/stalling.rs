//! Fleet handing the Operator a Job gone quiet with work left, and doing what it rules on one.
//! **Read from state**: the Stopped mark Fleet already draws, and a red pull request the Job ended
//! a turn on after Fleet told it so, with no push since. Never from a transcript.
//!
//! **Once a stall.** A read of a row notes what it found; the tick hands each on unless the Log
//! already holds it or a call on it is out, so a row read many times arrives once.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::HostedSessions;
use ipc::Notice;

use super::seat::Seat;
use super::{held, Arrival, Quiet, Stall, Way, NEEDS_YOU_WAIT};
use crate::daemon::Fleet;

/// The notice Fleet keeps when it tells a Job its pull request failed: the red signal reused.
const CHECKS_FAILED: &str = "checks_failed";

/// What a read of a Job's row found it gone quiet with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Seen {
    /// Marked Stopped at `at`, with the tasks its own list still holds open.
    Stopped { at: String, tasks: Vec<String> },
    /// At rest on an open pull request whose required checks failed.
    Red { number: u64, failing: String },
}

/// What `record` says of a Job gone quiet. `at_rest` is the ledger's read that it is live, between
/// turns with nothing asked, and not parked. A dead process is not one: Fleet starts a
/// repair Job for it.
pub(crate) fn seen_of(record: &ipc::SessionRecord, at_rest: bool) -> Option<Seen> {
    // Work still running is not quiet: a subagent or a task may yet fix what it left.
    if !record.running.is_empty() {
        return None;
    }
    if record.standing == Some(ipc::SessionStanding::Stopped) {
        let stopped = record.stopped.as_ref()?;
        return matches!(stopped.reason, ipc::StopReason::TurnEnded | ipc::StopReason::TasksOpen)
            .then(|| Seen::Stopped { at: stopped.at.as_str().to_string(), tasks: stopped.tasks.clone() });
    }
    if !at_rest {
        return None;
    }
    record.attachments.iter().find_map(|one| {
        let detail = |key: &str| one.detail.get(key).map(String::as_str);
        let red = one.kind == crate::pull_requesting::PR
            && one.state == ipc::AttachmentState::Standing
            && detail("state") == Some("open")
            && detail("checks") == Some("failed");
        let number = one.target.parse().ok().filter(|_| red)?;
        Some(Seen::Red { number, failing: detail("failing").unwrap_or_default().to_string() })
    })
}

/// The stalls the last read of each row found, and those a call is out on.
#[derive(Default)]
pub(crate) struct Stalls {
    seen: Mutex<BTreeMap<String, Seen>>,
    out: Arc<Mutex<BTreeSet<String>>>,
}

impl Seat {
    /// What a read of `session`'s row found. `None` is a Job not gone quiet.
    pub(crate) fn stall_noted(&self, session: &str, seen: Option<Seen>) {
        let mut held_now = held(&self.stalls.seen);
        match seen {
            Some(seen) => held_now.insert(session.to_string(), seen),
            None => held_now.remove(session),
        };
    }

    fn stalls_seen(&self) -> Vec<(String, Seen)> {
        held(&self.stalls.seen).iter().map(|(id, seen)| (id.clone(), seen.clone())).collect()
    }

    /// Rule on a stall in the background, unless a call on it is already out.
    fn hand_stall(&self, arrival: Arrival, within: Duration) {
        let Some(operator) = self.operator() else { return };
        let out = Arc::clone(&self.stalls.out);
        if !held(&out).insert(arrival.id.clone()) {
            return;
        }
        tokio::spawn(async move {
            let id = arrival.id.clone();
            operator.ruled(arrival, within).await;
            held(&out).remove(&id);
        });
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
    /// Each Job a read found gone quiet, handed to the Operator once. Once a tick, and nothing
    /// while the Operator is off.
    pub async fn stalls_handed(&self) {
        if !self.operator_on() {
            return;
        }
        for (id, seen) in self.operator_seat().stalls_seen() {
            let Some(stall) = self.stall_of(&id, seen).await else { continue };
            let arrival = stall.arrival();
            if self.store().lock().await.operator_ruled(&arrival.id).unwrap_or(true) {
                continue;
            }
            self.operator_seat().hand_stall(arrival, NEEDS_YOU_WAIT);
        }
    }

    /// The stall `seen` as the Operator reads it: the Job's plan, what is done and what is left.
    /// **A red pull request is a stall only once a turn ended after Fleet told the Job of it**,
    /// since that turn is the one that left it red.
    async fn stall_of(&self, id: &str, seen: Seen) -> Option<Stall> {
        let store = self.store().lock().await;
        let session = store.session(id).ok().flatten()?;
        let hosting = store.hosting(id).ok().flatten()?;
        let (quiet, tasks) = match seen {
            Seen::Stopped { at, tasks } => (Quiet::Stopped { at }, tasks),
            Seen::Red { number, failing } => {
                let (head, told) = store.pull_notice_latest(number, CHECKS_FAILED, &format!("session:{id}")).ok().flatten()?;
                let millis = |at: &str| core_model::Timestamp::from_rfc3339(at).epoch_millis();
                let ended = session.last_turn_at.as_deref().and_then(millis)?;
                if ended <= millis(&told)? {
                    return None;
                }
                (Quiet::Red { number, head, failing }, Vec::new())
            }
        };
        let plan = store.session_plan(id).ok().flatten();
        let planned: Vec<&store::KeptPlanTask> = plan.iter().flat_map(|plan| &plan.groups).flat_map(|group| &group.tasks).collect();
        let (done, left) = match planned.is_empty() {
            true => (Vec::new(), tasks),
            false => {
                let (done, left): (Vec<_>, Vec<_>) = planned.into_iter().partition(|task| crate::plan_record::status(task) == Some("completed"));
                (done.into_iter().map(|task| task.text.clone()).collect(), left.into_iter().map(|task| task.text.clone()).collect())
            }
        };
        let nudged = store.operator_rulings_of(id).unwrap_or_default().iter().any(|row| row.outcome == Way::Nudge.word());
        Some(Stall {
            session_id: id.to_string(),
            manifest_id: hosting.manifest_id,
            title: session.title.unwrap_or_else(|| id.to_string()),
            quiet,
            plan: plan.map(|plan| plan.summary),
            done,
            left,
            nudged,
        })
    }

    /// A Job gone quiet is told the next step the Operator named, as Fleet's word, and wakes on it.
    pub(super) async fn operator_nudged(self: &Arc<Self>, id: &str, step: &str) {
        let text = format!(
            "The Operator read your plan against what is left of it. Your next step: {step}\n\
             Carry on with it now. If you wait on the person, name it with waiting_for. If nothing is left, call all_done."
        );
        let _ = self.told_by_fleet(id, &text, Notice::Operator).await;
    }

    /// What is left of a Job gone quiet starts as a Job of its own, nested under it and briefed
    /// with what Fleet saw. **The quiet Job is left to the person**: its branch may hold work.
    pub(super) async fn operator_started_rest(self: &Arc<Self>, id: &str, seen: &str) {
        let Ok((session, hosting)) = self.session_and_hosting(id).await else { return };
        let title = session.title.clone().unwrap_or_else(|| id.to_string());
        let made = self
            .start_session(ipc::StartSession {
                manifest_id: ipc::ManifestId::carried(&hosting.manifest_id),
                title: Some(format!("Rest of {title}")),
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
            .await;
        let made = match made {
            Ok(made) => made,
            Err(why) => return self.session_warned(id, "the Operator's new Job for the rest did not start", format!("{why:?}")),
        };
        let rest = made.id.as_str().to_string();
        if self.started_linked(id, &rest, &hosting.manifest_id, None).await.is_err() {
            return;
        }
        let mut brief = format!("Job {id}, \"{title}\", went quiet with work left, and the Operator started what is left as this Job. What Fleet saw:\n\n{seen}");
        if let Some(branch) = hosting.lease_branch {
            brief.push_str(&format!("\n\nIts work is committed on branch `{branch}`. Take what you need from it."));
        }
        brief.push_str("\n\nDo what is left, then call all_done.");
        let sent = Arc::clone(self)
            .send_session_message(ipc::SendSessionMessage { session_id: made.id.clone(), text: brief, attachments: Vec::new(), mentions: Vec::new() })
            .await;
        if sent.is_ok() {
            self.fleet_note(id, format!("The Operator started what is left of this Job as \"Rest of {title}\".")).await;
            let _ = self.published_hosted(id).await;
        }
    }

    /// A Job gone quiet goes on his Needs you list with the Operator's read.
    pub(super) async fn operator_raised(&self, id: &str, read: &str) {
        let text = format!("The Operator raised this Job: {read}");
        self.fleet_note(id, text.clone()).await;
        self.call_raised(id, Notice::Operator, &text).await;
    }
}
