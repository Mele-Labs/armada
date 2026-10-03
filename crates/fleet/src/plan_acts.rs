//! A person's two acts on a plan's groups: Restart this task (`#1656`) and a
//! move (`#1685`). Spike 022, slice 2, with the bodies its lock agrees.
//!
//! **Both apply once a group's own rounds are spent** (answer 9): a task still
//! working or handed in is its group's to finish, so neither act reaches one.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::{
    Actor, GroupId, Job, JobId, PlanChange, PlanRefused, TaskId, TaskUpdate, WorkPlan,
};
use store::PlanHand;

use crate::adrift::Adrift;
use crate::budget::budgeted_for;
use crate::daemon::Fleet;
use crate::grouping::{in_flight, restartable, NotRestartable};
use crate::resume::{Ending, Redirection as Instruction};
use crate::work_plan::plan_not_kept;

/// What a `move_plan` body asks for, or `None` where an id does not read.
pub(crate) fn the_move(body: &ipc::MovePlan) -> Option<PlanChange> {
    let group = GroupId::read(body.group.trim())?;
    Some(match &body.task {
        Some(task) => PlanChange::MovedTask {
            task: TaskId::read(task.trim())?,
            group,
            after: match &body.after {
                Some(after) => Some(TaskId::read(after.trim())?),
                None => None,
            },
        },
        None => PlanChange::MovedGroup {
            group,
            after: match &body.after {
                Some(after) => Some(GroupId::read(after.trim())?),
                None => None,
            },
        },
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
    /// Restart this task, from Bridge: refused unless the task failed or the
    /// Judge refused its group, then the task reopens and `restart_step`'s own
    /// act puts a Drone on it, which opens with what stopped the step.
    pub(crate) async fn restart_task_by_person(
        self: Arc<Self>,
        job_id: ipc::JobId,
        task: String,
        body: ipc::RestartTask,
    ) -> Result<Job, Refusal> {
        let task = TaskId::read(task.trim()).ok_or_else(|| self.refusal(Adrift::Unnameable))?;
        let said = match &body.note {
            Some(note) => {
                Some(Instruction::saying(note).ok_or_else(|| self.refusal(Adrift::Unnameable))?)
            }
            None => None,
        };
        budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move {
                fleet
                    .restarted_task(&job_id.to_domain(), task, said.as_ref())
                    .await
            }
        })
        .await
        .map_err(|why| self.refusal(why))
    }

    /// A person moves a task or a group, from Bridge, by `after`.
    pub(crate) async fn move_plan_by_person(
        self: Arc<Self>,
        job_id: ipc::JobId,
        body: ipc::MovePlan,
    ) -> Result<ipc::WorkPlan, Refusal> {
        let change = the_move(&body).ok_or_else(|| self.refusal(Adrift::Unnameable))?;
        let job = job_id.to_domain();
        let plan = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            let job = job.clone();
            async move { fleet.moved_by_person(&job, &change).await }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        let runs = self
            .group_runs_of(&job)
            .await
            .map_err(|why| self.refusal(why))?;
        Ok(self.served_plan(&job, &plan, &runs).await)
    }

    async fn restarted_task(
        &self,
        job: &JobId,
        task: TaskId,
        note: Option<&Instruction>,
    ) -> Result<Job, Adrift> {
        let plan = self.planned(job).await?;
        let runs = self.group_runs_of(job).await?;
        let held = self.load(job).await?;
        if let Err(why) = restartable(&plan, &runs, &held, task) {
            return Err(match why {
                NotRestartable::NoSuchTask { task } => Adrift::PlanRefused {
                    job: job.clone(),
                    why: PlanRefused::NoSuchTask { named: task },
                },
                NotRestartable::NotFailed { task, state } => Adrift::TaskNotFailed {
                    job: job.clone(),
                    named: task,
                    state,
                },
            });
        }
        // What the task goes back to on a refusal: failed with its reason, or
        // done, where the Judge refused its group after green Checks.
        // The Drone a refusal leaves idle is ended: the owner's press asks for
        // a new agent on this task, never a redirect of the last one.
        let (was, ending) = match plan.task(task).and_then(|t| t.failed_reason()) {
            Some(why) => (
                core_model::FailReason::new(why).map(TaskUpdate::Failed),
                Ending::Unheard,
            ),
            None => (Some(TaskUpdate::Done), Ending::Any),
        };
        self.changed_by_person(job, &reopened(task, TaskUpdate::Open))
            .await?;
        match self.restart_step_ending(job, note, ending).await {
            Ok(job) => Ok(job),
            // Refused: the task goes back as it was, so the plan says what the
            // Job does.
            Err(why) => {
                if let Some(was) = was {
                    self.changed_by_person(job, &reopened(task, was)).await?;
                }
                Err(why)
            }
        }
    }

    async fn moved_by_person(&self, job: &JobId, change: &PlanChange) -> Result<WorkPlan, Adrift> {
        let plan = self.planned(job).await?;
        if let Some((named, state)) = in_flight(&plan, change) {
            return Err(Adrift::TaskInFlight {
                job: job.clone(),
                named,
                state,
            });
        }
        self.changed_by_person(job, change).await
    }

    async fn planned(&self, job: &JobId) -> Result<WorkPlan, Adrift> {
        self.plan_of(job).await?.ok_or_else(|| Adrift::PlanRefused {
            job: job.clone(),
            why: PlanRefused::NoPlan,
        })
    }

    /// Keep a person's change, and say so on `job.plan_changed`.
    pub(crate) async fn changed_by_person(
        &self,
        job: &JobId,
        change: &PlanChange,
    ) -> Result<WorkPlan, Adrift> {
        let at = self.now();
        let plan = self
            .store()
            .lock()
            .await
            .change_plan(job, change, PlanHand::Person, &at)
            .map_err(|why| plan_not_kept(job, why))?;
        self.publish(ipc::Event::JobPlanChanged(ipc::JobPlanChanged::changed(
            job,
            change,
            &plan,
            Actor::Human,
            &at,
        )));
        Ok(plan)
    }
}

fn reopened(task: TaskId, to: TaskUpdate) -> PlanChange {
    PlanChange::Updated {
        task,
        to,
        shown: None,
    }
}
