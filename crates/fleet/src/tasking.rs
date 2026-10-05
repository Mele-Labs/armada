//! A step that works its plan one task at a time, each task by a Drone of its
//! own: spike 022, slice 1b. `docs/concepts/plan.md`, *A Drone per task*, has
//! the whole of it; in short:
//!
//! - **Fleet writes every state**, because two of eleven Drones told to call
//!   `update_task` did (#1752): working at the spawn, handed in at the task's
//!   `submit_evidence`, done when the step's Checks pass.
//! - **A hand-in fills no inbox.** The first turn after that Drone comes to
//!   rest ends it and spawns the next task's, asking neither the cap nor
//!   headroom (answer 2). `Working::settled` says why it waits.
//! - **The last hand-in fills it once**, with every task's claim, and its Drone
//!   stays for the outcome, as a step retry is one Drone today.
//! - **Since slice 2 that is per group**, and `crate::grouping` decides what
//!   its gate leads to; since slice 5 some run at once (`crate::crew`).

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, DroneId, GroupRuns, Job, JobId, JobStatus, PlanChange, PlanTask, ResolvedStep, Shown,
    StepId, TaskId, TaskState, TaskUpdate, Timestamp, WorkPlan,
};
use store::{PlanHand, TaskHandIn};
use verification::{Claimed, NotClaimed, ShownBy, Submission};

use crate::adrift::Adrift;
use crate::briefing::Opening;
use crate::crossing::{Crossed, Produced};
use crate::daemon::Fleet;
use crate::evidence::{cut_short, Call, EvidenceTool, NotSubmitted, Recorded};
use crate::gate::Ruling;
use crate::slots::Slot;
use crate::work_plan::plan_not_kept;
use crate::working::Working;

pub use crate::grouping::{
    a_group_follows, current_group, failed, group_end, in_flight, restartable, verdict_of,
    why_it_failed, GroupEnd, NotRestartable,
};

/// The task the next spawn on the step is put on: the first, **in the group
/// the step is working**, that is open or working under a Drone that is gone.
/// `None` once that group's tasks are all handed in, done, failed or dropped,
/// which is when the group's gate fires.
pub fn next_task<'p>(plan: &'p WorkPlan, runs: &GroupRuns) -> Option<&'p PlanTask> {
    next_task_beside(plan, runs, &[])
}

/// [`next_task`], beside the tasks live Drones are on: one of none of them,
/// safe to run with each that is still working. Slice 5.
pub fn next_task_beside<'p>(
    plan: &'p WorkPlan,
    runs: &GroupRuns,
    live: &[TaskId],
) -> Option<&'p PlanTask> {
    let group = current_group(plan, runs)?;
    let working: Vec<TaskId> = live
        .iter()
        .copied()
        .filter(|id| plan.task(*id).map(PlanTask::state) == Some(TaskState::Working))
        .collect();
    plan.tasks_in(group)
        .filter(|task| matches!(task.state(), TaskState::Open | TaskState::Working))
        .filter(|task| !live.contains(&task.id()))
        .find(|task| {
            working
                .iter()
                .all(|other| runs.together(plan, task.id(), *other))
        })
}

/// What Fleet appends when a task's Drone is spawned.
pub fn started(task: TaskId) -> PlanChange {
    PlanChange::Updated {
        task,
        to: TaskUpdate::Working,
        shown: None,
    }
}

/// What Fleet appends at a task's hand-in. `shown` is the hand-in's own
/// `shown_by`, or `the_evidence_accounts_for_itself` has nothing to weigh.
pub fn handed_in(task: TaskId, shown_by: &str) -> PlanChange {
    PlanChange::Updated {
        task,
        to: TaskUpdate::HandedIn,
        shown: Shown::new(shown_by),
    }
}

/// What Fleet appends for each handed-in task once the step's Checks pass.
pub fn done(task: TaskId) -> PlanChange {
    PlanChange::Updated {
        task,
        to: TaskUpdate::Done,
        shown: None,
    }
}

/// One task's hand-in, as its Drone made it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandIn {
    pub task: TaskId,
    pub claimed: String,
    pub shown_by: String,
    pub not_claimed: String,
}

/// The step's one submission, every task's claim in it, labelled by task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Together {
    pub claimed: String,
    pub shown_by: String,
    pub not_claimed: String,
}

/// The group's submission once none of its tasks is open or working, and
/// `None` until: its handed-in and failed tasks' claims.
///
/// In plan order, and a task's latest hand-in wins, since a Drone the Checks
/// sent the work back to hands in again. A handed-in task with no hand-in in
/// hand is still named, by title and `shown`, so no task is left out.
pub fn together(plan: &WorkPlan, runs: &GroupRuns, hand_ins: &[HandIn]) -> Option<Together> {
    let group = current_group(plan, runs)?;
    if next_task(plan, runs).is_some() {
        return None;
    }
    let mut claimed = Vec::new();
    let mut shown_by = Vec::new();
    let mut not_claimed = Vec::new();
    for task in plan
        .tasks_in(group)
        .filter(|task| matches!(task.state(), TaskState::HandedIn | TaskState::Failed))
    {
        let id = task.id();
        match hand_ins.iter().rev().find(|hand_in| hand_in.task == id) {
            Some(hand_in) => {
                claimed.push(format!("{id}: {}", hand_in.claimed));
                shown_by.push(format!("{id}: {}", hand_in.shown_by));
                if !hand_in.not_claimed.trim().is_empty() {
                    not_claimed.push(format!("{id}: {}", hand_in.not_claimed));
                }
            }
            None => {
                claimed.push(format!("{id}: {}", task.title()));
                shown_by.push(format!(
                    "{id}: {}",
                    task.shown().map_or("nothing was kept", Shown::as_str)
                ));
            }
        }
    }
    (!claimed.is_empty()).then(|| Together {
        claimed: claimed.join("\n"),
        shown_by: shown_by.join("\n"),
        not_claimed: not_claimed.join("\n"),
    })
}

/// Whether the step's Checks passed, which is when a handed-in task is done
/// (answer 1). A Judge refusing after them does not undo it.
pub(crate) fn green(ruling: &Ruling) -> bool {
    !matches!(
        ruling,
        Ruling::Failed { .. }
            | Ruling::HandedBack { .. }
            | Ruling::CouldNotDecide { .. }
            | Ruling::NotWhatTheStepAsked(_)
    ) && ruling.checks().iter().all(|check| check.outcome.advances())
}

fn per_task(job: &Job, step: &StepId) -> bool {
    job.workflow()
        .step(step)
        .is_some_and(ResolvedStep::drone_per_task)
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
    /// The task a spawn on this step is put on, with the plan it is in. `None`
    /// where the step works no task a Drone each, and where none is left: a
    /// step restarted with every task handed in gets one Drone, as without
    /// the key.
    pub(crate) async fn task_to_work(
        &self,
        job: &Job,
        step: &StepId,
    ) -> Result<Option<(WorkPlan, PlanTask)>, Adrift> {
        if !per_task(job, step) {
            return Ok(None);
        }
        let Some(plan) = self.plan_of(job.id()).await? else {
            return Ok(None);
        };
        let runs = self.group_runs_of(job.id()).await?;
        let live = self.live_tasks(job.id());
        let task = next_task_beside(&plan, &runs, &live).cloned();
        Ok(task.map(|task| (plan, task)))
    }

    /// Mark the task working and bind the Drone to it, before the spawn is
    /// announced: Bridge reads `list_job_drones` on `drone.spawned`.
    pub(crate) async fn put_on_task(
        &self,
        job: &Job,
        step: &StepId,
        drone: &DroneId,
        task: TaskId,
    ) -> Result<(), Adrift> {
        let at = self.now();
        let plan = {
            let mut store = self.store().lock().await;
            let plan = store
                .change_plan(job.id(), &started(task), PlanHand::Step(step), &at)
                .map_err(|why| plan_not_kept(job.id(), why))?;
            store
                .record_task_drone(job.id(), drone, step, task, &at)
                .map_err(Adrift::Writing)?;
            plan
        };
        self.drone_on_task(job.id(), drone, task);
        self.task_moved(job.id(), &plan, task, &at);
        // Fleet stamps the group's start at its first task's spawn.
        if let Some(group) = plan.task(task).map(PlanTask::group) {
            self.group_started(job.id(), step, group).await?;
        }
        Ok(())
    }

    /// A task's Drone hands in. **Called holding its slot**, from both of
    /// `submit_evidence`'s doors, so the step cannot move under it.
    pub(crate) async fn hand_in_task(
        &self,
        working: &mut Option<Working>,
        call: Call<'_>,
        at: Timestamp,
    ) -> Result<Recorded, NotSubmitted> {
        let Some(at_work) = working.as_mut() else {
            return Err(NotSubmitted::NothingIsWorking);
        };
        let Some(task) = at_work.task() else {
            return Err(NotSubmitted::NothingIsWorking);
        };
        let (job, step, drone) = at_work.drone();
        // Refused as a malformed call before anything is kept, as a step's is.
        Submission::submitted(
            call.evidence_type,
            call.claimed,
            call.shown_by,
            call.not_claimed,
        )
        .map_err(NotSubmitted::Malformed)?;
        self.kept_edits(at_work, &at)
            .await
            .map_err(|why| NotSubmitted::NotKept(why.to_string()))?;
        let (plan, hand_ins) = self
            .kept_hand_in(&job, &step, &drone, task, call, &at)
            .await
            .map_err(NotSubmitted::NotKept)?;
        at_work.task_handed_in(at.clone());
        self.task_moved(&job, &plan, task, &at);
        let runs = self
            .group_runs_of(&job)
            .await
            .map_err(|why| NotSubmitted::NotKept(why.to_string()))?;
        if together(&plan, &runs, &hand_ins).is_some()
            && self
                .ran_apart(&job, &step, &plan, &runs, &at)
                .await
                .map_err(|why| NotSubmitted::NotKept(why.to_string()))?
        {
            return Ok(Recorded);
        }
        let Some(together) = together(&plan, &runs, &hand_ins) else {
            return Ok(Recorded);
        };
        let step_call = Call {
            evidence_type: call.evidence_type,
            claimed: Claimed(&together.claimed),
            shown_by: ShownBy(&together.shown_by),
            not_claimed: NotClaimed(&together.not_claimed),
            review: call.review,
        };
        self.kept_pending(&job, &step, step_call, &at).await?;
        let recorded = EvidenceTool::for_job(job.clone(), self.inbox())
            .submit(step_call, at.clone())
            .map_err(NotSubmitted::Malformed)?;
        cut_short(working, &at);
        self.published_submission(&job, &step, call.evidence_type, &at);
        Ok(recorded)
    }

    /// Write the hand-in down, and answer with the plan it leaves and every
    /// hand-in this step has had.
    async fn kept_hand_in(
        &self,
        job: &JobId,
        step: &StepId,
        drone: &DroneId,
        task: TaskId,
        call: Call<'_>,
        at: &Timestamp,
    ) -> Result<(WorkPlan, Vec<HandIn>), String> {
        let mut store = self.store().lock().await;
        let plan = store
            .change_plan(
                job,
                &handed_in(task, call.shown_by.0),
                PlanHand::Step(step),
                at,
            )
            .map_err(|why| why.to_string())?;
        store
            .record_task_hand_in(
                job,
                drone,
                &TaskHandIn {
                    claimed: call.claimed.0.to_string(),
                    shown_by: call.shown_by.0.to_string(),
                    not_claimed: call.not_claimed.0.to_string(),
                    at: at.clone(),
                },
            )
            .map_err(|why| why.to_string())?;
        let hand_ins = store
            .task_drones(job)
            .map_err(|why| format!("{why:?}"))?
            .into_iter()
            .filter(|bound| &bound.step_id == step)
            .filter_map(|bound| {
                bound.handed_in.map(|hand_in| HandIn {
                    task: bound.task,
                    claimed: hand_in.claimed,
                    shown_by: hand_in.shown_by,
                    not_claimed: hand_in.not_claimed,
                })
            })
            .collect();
        Ok((plan, hand_ins))
    }

    /// Whether the Drone in this slot handed its task in and the next task's
    /// Drone is owed. Also what keeps `reap` from reading its exit as a Drone
    /// that left without handing in.
    pub(crate) async fn between_tasks(&self, at_work: &Working) -> Result<bool, Adrift> {
        if !at_work.has_handed_in() {
            return Ok(false);
        }
        let (job, _, _) = at_work.standing();
        if self.evidence_waiting_for(&job) > 0 {
            return Ok(false);
        }
        if self.load(&job).await?.status() != JobStatus::Running {
            return Ok(false);
        }
        let Some(plan) = self.plan_of(&job).await? else {
            return Ok(false);
        };
        let runs = self.group_runs_of(&job).await?;
        Ok(next_task(&plan, &runs).is_some())
    }

    /// End a task's Drone that handed in, once it has come to rest, and put the
    /// next task's on the same worktree, from the step's baseline and every
    /// path declared so far.
    pub(crate) async fn next_task_drone(&self, slot: &Slot) -> Result<(), Adrift> {
        let mut held = slot.lock().await;
        let working = &mut *held;
        let Some(at_work) = working.as_ref() else {
            return Ok(());
        };
        if !self.between_tasks(at_work).await?
            || !at_work.settled(&self.now(), self.norms().report_grace())
        {
            return Ok(());
        }
        // A task left may be one a Drone beside this one is on, or one not
        // safe beside it: then this Drone waits rather than taking the step.
        let job = self.load(&at_work.standing().0).await?;
        let mut live = self.live_tasks(job.id());
        live.retain(|task| Some(*task) != at_work.task());
        let Some(plan) = self.plan_of(job.id()).await? else {
            return Ok(());
        };
        let runs = self.group_runs_of(job.id()).await?;
        if next_task_beside(&plan, &runs, &live).is_none() {
            return Ok(());
        }
        self.put_next_task_drone(working).await
    }

    /// End the Drone in this slot and put the next task's on its worktree.
    /// **The caller has decided a task is owed**: the turn above, or a green
    /// group's gate in `crate::grouping`.
    pub(crate) async fn put_next_task_drone(
        &self,
        working: &mut Option<Working>,
    ) -> Result<(), Adrift> {
        let Some(at_work) = working.as_ref() else {
            return Ok(());
        };
        let (job_id, step, _) = at_work.standing();
        let carried = at_work.carried();
        let stood_down = self.stood_down(&job_id, working).await?;
        let job = self.load(&job_id).await?;
        let worktree = match stood_down {
            Some(stood_down) => stood_down.worktree,
            None => self.surviving_worktree(&job)?,
        };
        // What the part before produced, as the step's first Drone was told.
        let recorded = self
            .store()
            .lock()
            .await
            .step_evidence(&job_id)
            .map_err(Adrift::Reading)?;
        let crossed =
            Crossed::nothing().and_produced(Produced::before(job.workflow(), &step, &recorded));
        self.put_a_drone_on(
            &job,
            &step,
            worktree,
            Opening::fresh().carrying(crossed),
            working,
        )
        .await?;
        if let Some(at_work) = working.as_mut() {
            at_work.carrying(carried);
        }
        Ok(())
    }

    /// Every handed-in task on this step is done, once its Checks pass. Before
    /// the step moves, so a client re-reading on the move finds them done.
    pub(crate) async fn tasks_done(
        &self,
        job_id: &JobId,
        step: &StepId,
        ruling: &Ruling,
    ) -> Result<(), Adrift> {
        if !green(ruling) || !per_task(&self.load(job_id).await?, step) {
            return Ok(());
        }
        let Some(plan) = self.plan_of(job_id).await? else {
            return Ok(());
        };
        for task in plan
            .tasks()
            .iter()
            .filter(|task| task.state() == TaskState::HandedIn)
        {
            let at = self.now();
            let plan = self
                .store()
                .lock()
                .await
                .change_plan(job_id, &done(task.id()), PlanHand::Step(step), &at)
                .map_err(|why| plan_not_kept(job_id, why))?;
            self.task_moved(job_id, &plan, task.id(), &at);
        }
        Ok(())
    }

    /// The task an adopted Drone was put on, and when it handed it in, off
    /// the record. `None` for a Drone that worked its step, and where the
    /// record will not say.
    pub(crate) async fn task_of(
        &self,
        job: &JobId,
        drone: &DroneId,
    ) -> Option<(TaskId, Option<Timestamp>)> {
        let bound = self.store().lock().await.task_drones(job).ok()?;
        bound
            .into_iter()
            .find(|bound| &bound.drone_id == drone)
            .map(|bound| (bound.task, bound.handed_in.map(|hand_in| hand_in.at)))
    }

    /// `job.plan_changed`, naming the task Fleet just marked and its state.
    fn task_moved(&self, job: &JobId, plan: &WorkPlan, task: TaskId, at: &Timestamp) {
        self.publish(ipc::Event::JobPlanChanged(ipc::JobPlanChanged::task_moved(
            job,
            plan,
            task,
            Actor::Fleet,
            at,
        )));
    }
}
