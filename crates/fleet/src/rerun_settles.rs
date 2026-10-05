//! What a passing re-run does to the tasks its red run failed. Job 3, 4 Oct
//! 2026: every Check passed on a press, and the Judge then read T1 to T4 as
//! `failed` with the red run's reason, refused, and stopped the step again.
//!
//! **A task is `failed` only because its group's Checks were red at the end of
//! the retries** (`crate::grouping::failed` is the one writer), so the pass
//! settles the failures of the group it ruled on and no other group's.
//!
//! **The Drone's `shown` is kept, never rewritten.** The Judge's brief says
//! beside the plan that it was written before the re-run: see [`note`].

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Attempt, FrozenWorkflow, GroupId, JobId, PlanAuthor, PlanChange, PlanEntry, StepEvidence,
    StepId, TaskState, Timestamp, WorkPlan,
};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::grouping::current_group;
use crate::tasking::{done, green};

/// What the pass settles in `group`: each task in it that is `failed`.
pub(crate) fn settled(plan: &WorkPlan, group: GroupId) -> Vec<PlanChange> {
    plan.tasks_in(group)
        .filter(|task| task.state() == TaskState::Failed)
        .map(|task| done(task.id()))
        .collect()
}

/// The plan as those changes leave it, for the reading the Judge makes before
/// they are written. A change the plan would not take leaves it as it was.
pub(crate) fn as_settled(
    plan: &WorkPlan,
    changes: &[PlanChange],
    step: &StepId,
    attempt: Attempt,
    at: &Timestamp,
) -> WorkPlan {
    changes.iter().fold(plan.clone(), |plan, change| {
        let entry = PlanEntry {
            change: change.clone(),
            by: PlanAuthor::Step {
                step_id: step.clone(),
                attempt,
            },
            at: at.clone(),
        };
        WorkPlan::after(Some(&plan), &entry).unwrap_or(plan)
    })
}

/// What the Judge is told beside the plan: which tasks the re-run settled, and
/// that each `shown` above was written before it.
pub(crate) fn note(group: GroupId, changes: &[PlanChange]) -> Option<String> {
    let tasks: Vec<String> = changes
        .iter()
        .filter_map(|change| match change {
            PlanChange::Updated { task, .. } => Some(task.to_string()),
            _ => None,
        })
        .collect();
    (!tasks.is_empty()).then(|| {
        format!(
            "Tasks settled by a re-run: {} in {group} were marked failed when the group's \
             Checks were red on the last run its retries allow. Fleet then ran the Checks \
             again on the same work and every one passed, so they stand as done. Each \
             `shown` above was written by its Drone before that re-run, and a failure it \
             describes is the red run's.",
            tasks.join(", ")
        )
    })
}

/// The note put after the plan's own text, in the step that records it.
pub(crate) fn with_the_note(
    mut recorded: Vec<(StepId, StepEvidence)>,
    workflow: &FrozenWorkflow,
    note: Option<&str>,
) -> Vec<(StepId, StepEvidence)> {
    let (Some(note), Some(step)) = (
        note,
        workflow.steps().iter().find(|step| step.records_plan()),
    ) else {
        return recorded;
    };
    if let Some((_, evidence)) = recorded.iter_mut().find(|(id, _)| id == step.id()) {
        evidence.claimed = format!("{}\n\n{note}", evidence.claimed);
    }
    recorded
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
    /// Write the settling where the re-run's ruling is green, **before the step
    /// moves** so a client re-reading on the move finds the tasks done. The
    /// group is read from the plan and the runs without opening a run: the gate
    /// has just ruled on the one it opened.
    pub(crate) async fn settled_by_the_rerun(
        &self,
        job: &JobId,
        step: &StepId,
        ruling: &Ruling,
    ) -> Result<(), Adrift> {
        if !green(ruling) {
            return Ok(());
        }
        let Some(plan) = self.plan_of(job).await? else {
            return Ok(());
        };
        let runs = self.group_runs_of(job).await?;
        let Some(group) = current_group(&plan, &runs) else {
            return Ok(());
        };
        for change in settled(&plan, group) {
            self.fleet_marked(job, step, &change).await?;
        }
        Ok(())
    }
}
