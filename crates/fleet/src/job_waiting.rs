//! What a quiet Job is waiting on, composed for `JobDetail::waiting`.
//!
//! **Read off what the detail already says**, so the list and the rows it
//! points at cannot disagree: [`waiting_of`] takes the queued reason and the
//! budget hold `get_job` filled and the steps' states. The one thing it cannot
//! read off the detail is which Jobs a dependency wait names, and that is
//! [`Fleet::jobs_waited_on`].
//!
//! # Empty is a Job that is working, or one that waits on a person
//!
//! A paused or frozen Job, a Job at a gate and a Job on an ask are held by a
//! person, and the Job's own status and asks already say so. This list is for
//! what clears without one.

use std::collections::BTreeMap;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{Job, JobStatus, QueuedReason, StepState};
use ipc::{JobDetail, JobWaiting};

use crate::adrift::Adrift;
use crate::coupling::unfinished_peers;
use crate::daemon::Fleet;

/// The Jobs a dependency wait names, and what to call each, or none.
pub(crate) type Awaited = Vec<(ipc::JobId, Option<String>)>;

/// What `detail` is waiting on, most specific first.
///
/// `awaited` is read only for a Job queued behind a dependency, and a queued
/// Job whose wait it cannot name still says so as a resource.
pub(crate) fn waiting_of(detail: &JobDetail, awaited: Awaited) -> Vec<JobWaiting> {
    match detail.job.status.domain() {
        JobStatus::Queued => queued(detail, awaited),
        JobStatus::Running => in_transition(detail).into_iter().collect(),
        _ => Vec::new(),
    }
}

fn queued(detail: &JobDetail, awaited: Awaited) -> Vec<JobWaiting> {
    let Some(reason) = detail.job.queued_reason.map(|reason| reason.domain()) else {
        return Vec::new();
    };
    match reason {
        // A person's to lift, and the row or the ask already says which.
        QueuedReason::Paused | QueuedReason::AwaitingAnswer | QueuedReason::Frozen => Vec::new(),
        QueuedReason::BlockedByDependency if !awaited.is_empty() => awaited
            .into_iter()
            .map(|(job_id, title)| JobWaiting::Job { job_id, title })
            .collect(),
        QueuedReason::OverBudget => vec![JobWaiting::Resource {
            name: detail
                .job
                .budget_hold
                .map_or(reason.as_wire(), |hold| hold.as_wire())
                .to_string(),
        }],
        QueuedReason::BlockedByDependency | QueuedReason::WaitingOnResources => {
            vec![JobWaiting::Resource {
                name: reason.as_wire().to_string(),
            }]
        }
    }
}

/// The step a running Job has reached and no Drone has opened on yet.
///
/// **Not a step that is `running`**: that one has a Drone, or its Checks, or a
/// Judge on it, and the Now draws those. A step that is `retrying` or not yet
/// started while the Job runs is the gap between one and the next.
fn in_transition(detail: &JobDetail) -> Option<JobWaiting> {
    let current = detail
        .steps
        .iter()
        .filter(|step| step.state.domain() != StepState::Advanced)
        .min_by_key(|step| step.ordinal)?;
    matches!(
        current.state.domain(),
        StepState::NotStarted | StepState::Retrying
    )
    .then(|| JobWaiting::Transition {
        step_id: current.step_id.clone(),
    })
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
    /// The Jobs this one is queued behind: its unfinished dependencies, or the
    /// unfinished Jobs it dispatched when it is held for those.
    ///
    /// **Cold unless `detail` is queued behind a dependency**, so the board
    /// read is spent only on the Job that is.
    pub(crate) async fn jobs_waited_on(
        &self,
        job: &Job,
        detail: &JobDetail,
    ) -> Result<Awaited, Adrift> {
        let blocked = detail
            .job
            .queued_reason
            .is_some_and(|reason| reason.domain() == QueuedReason::BlockedByDependency);
        if !blocked {
            return Ok(Vec::new());
        }
        let (loaded, _) = self.every_job().await?;
        let standing: BTreeMap<_, _> = loaded
            .jobs
            .iter()
            .map(|held| (held.id().clone(), held.status()))
            .collect();
        let named: Vec<core_model::JobId> = unfinished_peers(job, &standing);
        Ok(named
            .iter()
            .map(|id| {
                let title = loaded
                    .jobs
                    .iter()
                    .find(|held| held.id() == id)
                    .map(|held| held.title().as_str().to_string());
                (ipc::JobId::from(id), title)
            })
            .collect())
    }
}
