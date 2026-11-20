//! How long a workflow step usually takes, and the Job that stands on one past that.
//! The design page *The Job's right column, by workflow step*; `docs/concepts/session/the-rail.md`.
//!
//! **The usual time is measured, not set**: the 90th percentile of how long Jobs on a workflow stayed on a step
//! before the next began, from the visits the rail kept (`store::workflow_step_dwells`), absent while fewer
//! than [`MIN_VISITS`] are known. A step past it with no move is flagged on the row (`WorkflowStrip::stalled`) and
//! the Job is told once, per visit to the step, what the step waits on. **The one word Fleet sends a Job about
//! its step**, which the owner asked for on 20 Nov 2026.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::Timestamp;
use store::Store;

use crate::clock::{elapsed, rfc3339_utc};
use crate::daemon::Fleet;
use crate::rail::{self, RailStep};
use crate::rail_ends::{self, End, Waiter};

/// Fewer visits than this say nothing of how long a step takes.
pub(crate) const MIN_VISITS: usize = 5;

/// How often the Jobs are read for a step gone past its time.
pub(crate) const EVERY: Duration = Duration::from_secs(60);

/// How long a workflow's measured times are kept before the visits are read again.
const KEPT_FOR_MS: i64 = 10 * 60 * 1000;

/// The 90th percentile of `seconds` by nearest rank, or `None` below [`MIN_VISITS`].
pub(crate) fn nine_in_ten(mut seconds: Vec<u64>) -> Option<u64> {
    if seconds.len() < MIN_VISITS {
        return None;
    }
    seconds.sort_unstable();
    let rank = (seconds.len() * 9).div_ceil(10);
    seconds.get(rank - 1).copied()
}

/// What was measured, and which Jobs are drawn amber. Never written down.
#[derive(Default)]
pub(crate) struct StepTimes {
    /// By workflow id: when it was read (epoch milliseconds) and each step's 9-in-10 seconds, by step id.
    read: Mutex<HashMap<String, (i64, Arc<HashMap<String, u64>>)>>,
    /// The Jobs last published with a stalled step.
    amber: Mutex<HashSet<String>>,
}

impl StepTimes {
    /// The 9-in-10 seconds of each step of `workflow` that has enough visits, by step id. Read again after ten minutes.
    pub(crate) fn usual(&self, store: &Store, workflow: &str, now_ms: i64) -> Arc<HashMap<String, u64>> {
        let mut read = self.read.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((at, found)) = read.get(workflow) {
            if now_ms - at < KEPT_FOR_MS {
                return Arc::clone(found);
            }
        }
        let mut by_step: HashMap<String, Vec<u64>> = HashMap::new();
        for (step, from, to) in store.workflow_step_dwells(workflow).unwrap_or_default() {
            let took = elapsed(&Timestamp::from_rfc3339(from), &Timestamp::from_rfc3339(to));
            by_step.entry(step).or_default().push(took.as_secs());
        }
        let found: Arc<HashMap<String, u64>> =
            Arc::new(by_step.into_iter().filter_map(|(step, seconds)| Some((step, nine_in_ten(seconds)?))).collect());
        read.insert(workflow.to_string(), (now_ms, Arc::clone(&found)));
        found
    }

    /// Note whether `session` is drawn amber now. **True where that moved**, so its row is published.
    pub(crate) fn amber_moved(&self, session: &str, stalled: bool) -> bool {
        let mut amber = self.amber.lock().unwrap_or_else(PoisonError::into_inner);
        match stalled {
            true => amber.insert(session.to_string()),
            false => amber.remove(session),
        }
    }
}

/// When a step that began at `since` went past `usual` seconds: `None` while it has not.
pub(crate) fn stalled_since(since: &str, usual: u64, now: &Timestamp) -> Option<String> {
    let began = Timestamp::from_rfc3339(since).epoch_millis()?;
    let past = began + (usual as i64) * 1000;
    (now.epoch_millis()? > past).then(|| rfc3339_utc(past))
}

/// What the step waits on, and whose it is: the cause a stalled step carries and the nudge repeats.
pub(crate) fn cause_of(steps: &[RailStep], at: usize) -> String {
    let ends = rail_ends::ends_of(steps, at);
    let Some(first) = ends.first() else {
        return "Nothing Fleet observes ends this step.".to_string();
    };
    let who = match first.waits_on() {
        Waiter::ThePerson(_) => "you",
        Waiter::TheJob => "the Job",
        Waiter::TheMerge => "the merge",
        Waiter::TheWorld => "the world",
    };
    let said: Vec<String> = rail_ends::ends_any(steps, at);
    format!("Waiting on {who}. The step ends when {}.", said.join(", or "))
}

/// A span as a person reads it: `5h 30m`, `22m`, `45s`.
pub(crate) fn spoken(seconds: u64) -> String {
    match (seconds / 3600, (seconds % 3600) / 60) {
        (0, 0) => format!("{seconds}s"),
        (0, minutes) => format!("{minutes}m"),
        (hours, 0) => format!("{hours}h"),
        (hours, minutes) => format!("{hours}h {minutes}m"),
    }
}

/// The one word to a Job that has stood on a step past its usual time.
pub(crate) fn nudge_told(workflow: &str, step: &RailStep, stood: u64, usual: u64, cause: &str, asks: bool) -> String {
    let press = if asks {
        " If it is done, raise one waiting_for item with approves: true so he can press Approve. Do not say it is done in the thread: only his press ends the step."
    } else {
        ""
    };
    format!(
        "Fleet: this Job has been on the \"{}\" step of its {workflow} workflow for {}, longer than the {} that 9 in 10 Jobs take. {cause}{press}",
        step.label,
        spoken(stood),
        spoken(usual)
    )
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
    /// Read every live Job for a step gone past its usual time. **Publish a row whose amber moved, and tell a Job
    /// once for each visit to the step**, at a quiet moment: nothing waits on the person and its turn is over.
    pub(crate) async fn steps_stalled_told(self: &Arc<Self>) {
        let live = self.store().lock().await.live_session_ids().unwrap_or_default();
        for id in live {
            let Some(record) = self.stop_read_again(&id).await else { continue };
            let stalled = record.workflow_strip.as_ref().is_some_and(|strip| strip.stalled.is_some());
            if self.step_times().amber_moved(&id, stalled) {
                let _ = self.published_hosted(&id).await;
            }
            if stalled {
                self.stalled_step_told(&id, &record).await;
            }
        }
    }

    async fn stalled_step_told(self: &Arc<Self>, id: &str, record: &ipc::SessionRecord) {
        let resting = matches!(record.standing, Some(ipc::SessionStanding::Idle | ipc::SessionStanding::Replied));
        if !resting || !record.waiting_for.is_empty() {
            return;
        }
        let (Some(manifest), Some(workflow), Some(at)) = (record.manifest_id.as_ref(), record.workflow.as_deref(), record.workflow_step) else {
            return;
        };
        let Some(strip) = record.workflow_strip.as_ref() else { return };
        let (Some(stalled), Some(usual)) = (strip.stalled.as_ref(), strip.usual_secs) else { return };
        let Some(steps) = self.job_rail_steps(id, manifest.as_str(), workflow).await else { return };
        let Some(step) = steps.get(at as usize) else { return };
        let mut kept = match self.store().lock().await.session_rail(id, &rail::step_ids(&steps)) {
            Ok(kept) => kept,
            Err(_) => return,
        };
        if kept.stalled.contains(&at) {
            return;
        }
        let stood = elapsed(&Timestamp::from_rfc3339(stalled.since.as_str()), &self.now()).as_secs() + usual;
        let asks = rail_ends::ends_of(&steps, at as usize).contains(&End::YouApproved);
        let name = self.workflow_name(manifest.as_str(), workflow);
        let words = nudge_told(&name, step, stood, usual, &stalled.cause, asks);
        // Told is a word that reached it; a Job that could not hear it is read again.
        if !matches!(self.told_by_fleet(id, &words, ipc::Notice::RailNudge).await, Ok(true)) {
            return;
        }
        kept.stalled.push(at);
        let ids = rail::step_ids(&steps);
        if let Err(why) = self.store().lock().await.keep_session_rail(id, &kept, &ids) {
            self.session_warned(id, "a step's nudge was not kept", why);
        }
    }
}
