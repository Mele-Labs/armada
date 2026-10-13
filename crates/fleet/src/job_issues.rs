//! What is wrong with a Job right now, composed for `JobDetail::issues`.
//!
//! **Pointers over what `get_job` already carries**, so the list and the rows
//! it points at cannot disagree: [`issues_of`] reads the breakages, the
//! overlaps and the steps' rulings off the detail built a moment before. The
//! one issue with no field behind it is a quiet Drone, which this reads off its
//! transcript.
//!
//! # A quiet Drone is a working one that wrote nothing
//!
//! The threshold is `crate::silence`'s `quiet_after`, resolved for the Drone's
//! step as the vigil resolves it. **A Drone at rest and a Drone waiting on a
//! person or on the gate are quiet by design** and never reported, which is the
//! vigil's own exemption. This reads and does not poke: the vigil acts, and its
//! first act is a poke at the same threshold.

use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Job, JobStatus, Timestamp};
use ipc::{JobDetail, JobIssue};

use crate::adrift::Adrift;
use crate::converging::elapsed;
use crate::daemon::Fleet;
use crate::drones_had::{drones_had, read_so_far, SoFar};

/// When a Drone last wrote, if it has been quiet for longer than `quiet_after`.
///
/// **`None` for a Drone at rest**, whose run has ended and is held for the
/// gate or a person. A Drone that has written nothing at all is quiet since it
/// was spawned.
pub(crate) fn quiet_since(
    read: &SoFar,
    spawned_at: &Timestamp,
    now: &Timestamp,
    quiet_after: Duration,
) -> Option<ipc::Instant> {
    if read.at_rest_since.is_some() {
        return None;
    }
    let since = read
        .last_wrote
        .clone()
        .unwrap_or_else(|| ipc::Instant::from(spawned_at));
    (elapsed(&since.to_domain(), now) > quiet_after).then_some(since)
}

/// The issues a detail has, each once, the most pressing first: a Drone gone
/// quiet, the latest failed ruling, a breakage, then another Job's overlap.
pub(crate) fn issues_of(detail: &JobDetail, stuck: Option<JobIssue>) -> Vec<JobIssue> {
    let mut issues: Vec<JobIssue> = stuck.into_iter().collect();
    // The latest step to carry a failed ruling. A step's ruling stands until its
    // next run closes, so an earlier step that failed and was fixed carries a
    // passed one by now.
    let failed = detail
        .steps
        .iter()
        .filter_map(|step| {
            let verdict = step.last_verdict.as_ref()?;
            (verdict.named == "failed").then_some((step, verdict))
        })
        .max_by_key(|(step, _)| step.ordinal);
    if let Some((step, verdict)) = failed {
        issues.push(JobIssue::FailedVerdict {
            step_id: step.step_id.clone(),
            attempt: verdict.attempt,
            trigger: verdict.trigger.clone(),
        });
    }
    for breakage in &detail.breakages {
        let issue = JobIssue::Breakage {
            check: breakage.check.clone(),
            test: breakage.test.clone(),
        };
        if !issues.contains(&issue) {
            issues.push(issue);
        }
    }
    for overlap in detail.write_scope_overlaps.iter().flatten() {
        let issue = JobIssue::WriteScopeOverlap {
            job_id: overlap.job_id.clone(),
        };
        if !issues.contains(&issue) {
            issues.push(issue);
        }
    }
    issues
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
    /// The longest-quiet working Drone of a running Job, as an issue.
    ///
    /// **Cold for every Job that is not running, and for one waiting on a
    /// person** — `waiting_on_person` is the ask or command the caller already
    /// holds on the detail — so the transcript read is spent only where a Drone
    /// could be stuck.
    pub(crate) async fn stuck_drone_of(
        &self,
        job: &Job,
        waiting_on_person: bool,
    ) -> Result<Option<JobIssue>, Adrift> {
        if job.status() != JobStatus::Running
            || waiting_on_person
            || self.evidence_waiting_for(job.id()) > 0
        {
            return Ok(None);
        }
        let events = self
            .store()
            .lock()
            .await
            .events_for(job.id())
            .map_err(|cause| Adrift::Reading(store::LoadJobError::Unreadable(cause)))?;
        let served = self.served_by(job)?;
        let now = self.now();
        let mut found: Option<JobIssue> = None;
        let mut longest: Option<ipc::Instant> = None;
        for had in drones_had(&events) {
            if had.state != ipc::DroneState::Running {
                continue;
            }
            let read = read_so_far(served.records_root(), &job.handle(), &had.drone).await;
            let quiet_after = self
                .liveness()
                .at(served.manifest(), job, &had.step)
                .quiet_after();
            let Some(since) = quiet_since(&read, &had.spawned_at, &now, quiet_after) else {
                continue;
            };
            if longest
                .as_ref()
                .is_some_and(|held| held.as_str() <= since.as_str())
            {
                continue;
            }
            found = Some(JobIssue::StuckDrone {
                drone_id: (&had.drone).into(),
                step_id: (&had.step).into(),
                quiet_since: since.clone(),
            });
            longest = Some(since);
        }
        Ok(found)
    }
}
