//! A plan's groups, run one at a time with the step's gate at each one's end:
//! spike 022, slice 2. `docs/concepts/plan.md`, *Groups*, has the whole of it.
//!
//! - **Green**: the group's tasks are done, it commits once if a group follows,
//!   and the next group's first task gets its Drone. The step moves only after
//!   the last group.
//! - **Red with a retry left**: one Drone, the group's last, goes round with
//!   the red Checks and the group's tasks; the tasks stay handed in (answer 9).
//! - **Red with none left**: every task in the group turns `failed`, which is
//!   when Restart this task and Move apply.
//! - **A Judge refusal** stops the group for a person, as today (answer 3).
//!   Its tasks read `done`, and Restart this task answers each of them too.

use adapter_traits::{AgentHarness, CommitTime, Committed, Delivery, Vcs, WorkProduct};
use core_model::{
    Actor, EscalationTrigger, FailReason, GroupId, GroupRuns, Job, JobId, PlanChange, ResolvedStep,
    StepId, StepLevelTrigger, StepState, StepVerdict, TaskId, TaskState, TaskUpdate, WorkPlan,
};
use store::PlanHand;

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::dispatch::stopping;
use crate::gate::Ruling;
use crate::session::{LiveSession, Occasion};
use crate::work_plan::{plan_not_kept, PlanChanged};
use crate::working::Working;

/// What a group's gate came to, for what follows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupEnd {
    /// Its Checks and its Judge passed.
    Passed,
    /// A Check was red and the step's retries allow another run.
    Round,
    /// A Check was red on the last run the retries allow.
    Failed,
    /// It waits for a person: a Judge refused, the evidence is suspect, or the
    /// gate could not read what it needed.
    Stopped,
    /// The submission was not the step's, so nothing was ruled.
    NotRuled,
}

/// Which of the five a ruling is.
pub fn group_end(ruling: &Ruling) -> GroupEnd {
    match ruling {
        Ruling::Advanced { .. } | Ruling::Finished { .. } | Ruling::HeldForReview { .. } => {
            GroupEnd::Passed
        }
        Ruling::HandedBack { .. } => GroupEnd::Round,
        Ruling::Failed { .. } => GroupEnd::Failed,
        Ruling::Refused { .. }
        | Ruling::Questioned { .. }
        | Ruling::Suspect { .. }
        | Ruling::CouldNotDecide { .. } => GroupEnd::Stopped,
        Ruling::NotWhatTheStepAsked(_) => GroupEnd::NotRuled,
    }
}

/// What a run of the group is recorded as having come to: the step's own
/// verdict words, and the trigger the step move wrote.
pub fn verdict_of(ruling: &Ruling) -> StepVerdict {
    let gate_failure = || {
        StepLevelTrigger::of(EscalationTrigger::GateFailure).expect("gate_failure is step-level")
    };
    match ruling {
        Ruling::Advanced { .. } | Ruling::Finished { .. } | Ruling::HeldForReview { .. } => {
            StepVerdict::Passed
        }
        Ruling::HandedBack { retrying, .. } => StepVerdict::Failed(*retrying),
        Ruling::NotWhatTheStepAsked(_) => StepVerdict::NotReached,
        other => StepVerdict::Failed(stopping(other).unwrap_or_else(gate_failure)),
    }
}

/// The group the step is working: the first, in the order groups run, with a
/// task still owed or a gate it has not passed. `None` once every group with a
/// task has passed, which is when the step's own gate moves it.
pub fn current_group(plan: &WorkPlan, runs: &GroupRuns) -> Option<GroupId> {
    plan.groups().iter().copied().find(|group| {
        let mut tasks = plan
            .tasks_in(*group)
            .filter(|task| task.state() != TaskState::Dropped);
        let owed = tasks.clone().any(|task| task.state() != TaskState::Done);
        owed || (tasks.next().is_some() && !runs.passed(*group))
    })
}

/// Whether any group after this one still has a task to run, which is whether
/// a green gate here commits and keeps the step, and holds handoff's Checks.
pub fn a_group_follows(plan: &WorkPlan, group: GroupId) -> bool {
    plan.groups()
        .iter()
        .skip_while(|g| **g != group)
        .skip(1)
        .any(|later| {
            plan.tasks_in(*later)
                .any(|task| !matches!(task.state(), TaskState::Dropped | TaskState::Done))
        })
}

/// Why a group's tasks failed, naming the group, the run and the red Checks.
pub fn why_it_failed(group: GroupId, runs: &GroupRuns, ruling: &Ruling) -> FailReason {
    let run = runs.attempts(group).last().map_or(1, |run| run.run);
    let red: Vec<&str> = ruling
        .checks()
        .iter()
        .filter(|check| !check.outcome.advances())
        .map(|check| check.name.as_str())
        .collect();
    let said = match red.is_empty() {
        true => String::from("its gate failed"),
        false => red.join(", "),
    };
    FailReason::new(&format!(
        "{group}'s Checks were still red on run {run}, the last its retries allow: {said}"
    ))
    .expect("never blank")
}

/// What Fleet appends when a group's retries run out: every task in it that
/// is not dropped turns failed, with the reason.
pub fn failed(plan: &WorkPlan, group: GroupId, reason: &FailReason) -> Vec<PlanChange> {
    plan.tasks_in(group)
        .filter(|task| task.state() != TaskState::Dropped)
        .map(|task| PlanChange::Updated {
            task: task.id(),
            to: TaskUpdate::Failed(reason.clone()),
            shown: None,
        })
        .collect()
}

/// Why Restart this task, or a move, does not answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotRestartable {
    NoSuchTask {
        task: TaskId,
    },
    /// Restart answers a failed task, and a done one in a group the Judge
    /// refused: one still working or handed in is its group's own round to
    /// finish (answer 9).
    NotFailed {
        task: TaskId,
        state: TaskState,
    },
}

/// Whether Restart this task answers: on a failed task, and on a done task in
/// a group the Judge refused, which is one press to run that task again.
pub fn restartable(
    plan: &WorkPlan,
    runs: &GroupRuns,
    job: &Job,
    task: TaskId,
) -> Result<(), NotRestartable> {
    let named = plan.task(task).ok_or(NotRestartable::NoSuchTask { task })?;
    match named.state() {
        TaskState::Failed => Ok(()),
        TaskState::Done if judge_refused(runs, job, named.group()) => Ok(()),
        state => Err(NotRestartable::NotFailed { task, state }),
    }
}

/// Whether the Judge refused the group's last run after green Checks.
///
/// **`gate_failure` over tasks still done, with the step that run was filed
/// under `stopped`.** A red last run writes the same trigger but fails every
/// task. A Judge's *question* writes it too, and holds the step at
/// `awaiting_human` until a person answers: a refusal from him stops it, as a
/// refusal does, and an agreement advances it (owner, 2 Oct 2026: a question
/// waits for his answer). The step move is the record that changes when he
/// answers, so it is the one read rather than a mark on the group's run.
fn judge_refused(runs: &GroupRuns, job: &Job, group: GroupId) -> bool {
    runs.stopped_on_gate_failure(group)
        && runs
            .attempts(group)
            .last()
            .and_then(|run| job.step(&run.step))
            .is_some_and(|step| step.state() == StepState::Stopped)
}

/// Whether a person's move may be taken: never of a task, or a group holding
/// one, that is working or handed in, since its group's run is still going.
/// `None` where it may. A move putting a task before one it needs is taken.
pub fn in_flight(plan: &WorkPlan, change: &PlanChange) -> Option<(TaskId, TaskState)> {
    let flying = |state: TaskState| matches!(state, TaskState::Working | TaskState::HandedIn);
    match change {
        PlanChange::MovedTask { task, .. } => plan
            .task(*task)
            .filter(|t| flying(t.state()))
            .map(|t| (t.id(), t.state())),
        PlanChange::MovedGroup { group, .. } => plan
            .tasks_in(*group)
            .find(|t| flying(t.state()))
            .map(|t| (t.id(), t.state())),
        _ => None,
    }
}

/// Where a group's gate stands as it runs: which group and run, and whether a
/// group follows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AtGroup {
    pub group: GroupId,
    pub run: u32,
    pub follows: bool,
}

fn per_task(step: Option<&ResolvedStep>) -> bool {
    step.is_some_and(ResolvedStep::drone_per_task)
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
    /// Every run of every group of this Job's plan.
    pub(crate) async fn group_runs_of(&self, job: &JobId) -> Result<GroupRuns, Adrift> {
        self.store()
            .lock()
            .await
            .group_runs(job)
            .map_err(Adrift::Reading)
    }

    /// Stamp a group's run as started, where none is open: at its first task's
    /// spawn, and at a gate no task Drone opened one for.
    pub(crate) async fn group_started(
        &self,
        job: &JobId,
        step: &StepId,
        group: GroupId,
    ) -> Result<u32, Adrift> {
        let runs = self.group_runs_of(job).await?;
        if let Some(open) = runs.open_attempt(group) {
            return Ok(open.run);
        }
        let mut store = self.store().lock().await;
        let attempt = store
            .step_attempt(job, step)
            .map_err(|cause| Adrift::Reading(store::LoadJobError::Unreadable(cause)))?;
        let opening = runs.opening(group, step, attempt, self.now());
        store
            .record_group_move(job, &opening, None)
            .map_err(Adrift::Writing)?;
        Ok(runs.spent(group).number())
    }

    /// The group a gate on this step is about to rule on, its run opened if no
    /// task Drone opened one. `None` where the step works no task a Drone
    /// each, or no group is owed.
    pub(crate) async fn group_at_gate(
        &self,
        job: &core_model::Job,
        step: &StepId,
    ) -> Result<Option<AtGroup>, Adrift> {
        if !per_task(job.workflow().step(step)) {
            return Ok(None);
        }
        let Some(plan) = self.plan_of(job.id()).await? else {
            return Ok(None);
        };
        let runs = self.group_runs_of(job.id()).await?;
        let Some(group) = current_group(&plan, &runs) else {
            return Ok(None);
        };
        let run = self.group_started(job.id(), step, group).await?;
        Ok(Some(AtGroup {
            group,
            run,
            follows: a_group_follows(&plan, group),
        }))
    }

    /// What a ruling does to the group it ruled on, **before the step moves**:
    /// its tasks marked, and which group and run it was.
    pub(crate) async fn group_ruled(
        &self,
        ruling: &Ruling,
        job_id: &JobId,
        step: &StepId,
    ) -> Result<Option<(AtGroup, GroupEnd)>, Adrift> {
        let job = self.load(job_id).await?;
        let Some(at) = self.group_at_gate(&job, step).await? else {
            self.tasks_done(job_id, step, ruling).await?;
            return Ok(None);
        };
        let end = group_end(ruling);
        match end {
            GroupEnd::Passed => {
                self.group_done(job_id, step, at.group).await?;
            }
            GroupEnd::Failed => {
                let runs = self.group_runs_of(job_id).await?;
                let reason = why_it_failed(at.group, &runs, ruling);
                let plan = self.plan_of(job_id).await?.ok_or(Adrift::PlanRefused {
                    job: job_id.clone(),
                    why: core_model::PlanRefused::NoPlan,
                })?;
                for change in failed(&plan, at.group, &reason) {
                    self.fleet_marked(job_id, step, &change).await?;
                }
            }
            // A refusal after green Checks leaves the tasks done (answer 1);
            // the group still waits for a person.
            GroupEnd::Stopped => self.tasks_done(job_id, step, ruling).await?,
            GroupEnd::Round | GroupEnd::NotRuled => {}
        }
        Ok(Some((at, end)))
    }

    /// After a green gate: the group's commit, where a group follows, and
    /// then the next group's first Drone in place of this one.
    pub(crate) async fn group_passed(
        &self,
        at: AtGroup,
        job_id: &JobId,
        working: &mut Option<Working>,
    ) -> Result<bool, Adrift> {
        let commit = match at.follows {
            true => self.group_commit(job_id, at.group, working).await?,
            false => None,
        };
        let runs = self.group_runs_of(job_id).await?;
        if let Some(ended) = runs.closing(at.group, StepVerdict::Passed, commit, self.now()) {
            self.store()
                .lock()
                .await
                .record_group_move(job_id, &ended, None)
                .map_err(Adrift::Writing)?;
        }
        self.group_moved(job_id, at.group).await?;
        if !at.follows {
            return Ok(false);
        }
        self.put_next_task_drone(working).await?;
        Ok(true)
    }

    /// After the step moved for a red run or a stop: the run's end on the
    /// Record row that move made, and for a round, the next run opened and the
    /// Drone told which tasks it covers. Nothing where no group was ruled on.
    pub(crate) async fn group_moved_on(
        &self,
        grouped: Option<(AtGroup, GroupEnd)>,
        ruling: &Ruling,
        job_id: &JobId,
        step: &StepId,
        working: &Option<Working>,
    ) -> Result<(), Adrift> {
        let Some((at, end)) = grouped else {
            return Ok(());
        };
        let into = match end {
            GroupEnd::Round => StepState::Retrying,
            GroupEnd::Failed | GroupEnd::Stopped => match ruling {
                Ruling::Questioned { .. } => StepState::AwaitingHuman,
                _ => StepState::Stopped,
            },
            GroupEnd::Passed | GroupEnd::NotRuled => return Ok(()),
        };
        let runs = self.group_runs_of(job_id).await?;
        let mut store = self.store().lock().await;
        let seq = store
            .last_step_move_into(job_id, step, into)
            .map_err(Adrift::Reading)?;
        if let Some(ended) = runs.closing(at.group, verdict_of(ruling), None, self.now()) {
            store
                .record_group_move(job_id, &ended, seq)
                .map_err(Adrift::Writing)?;
        }
        if end == GroupEnd::Round {
            let runs = store.group_runs(job_id).map_err(Adrift::Reading)?;
            let attempt = store
                .step_attempt(job_id, step)
                .map_err(|cause| Adrift::Reading(store::LoadJobError::Unreadable(cause)))?;
            let opening = runs.opening(at.group, step, attempt, self.now());
            store
                .record_group_move(job_id, &opening, None)
                .map_err(Adrift::Writing)?;
        }
        drop(store);
        self.group_moved(job_id, at.group).await?;
        if end == GroupEnd::Round {
            self.told_the_round(job_id, at.group, working).await?;
        }
        Ok(())
    }

    /// Every handed-in or failed task of the group is done, at its green gate.
    async fn group_done(
        &self,
        job_id: &JobId,
        step: &StepId,
        group: GroupId,
    ) -> Result<(), Adrift> {
        let Some(plan) = self.plan_of(job_id).await? else {
            return Ok(());
        };
        for task in plan
            .tasks_in(group)
            .filter(|t| matches!(t.state(), TaskState::HandedIn | TaskState::Failed))
        {
            self.fleet_marked(job_id, step, &crate::tasking::done(task.id()))
                .await?;
        }
        Ok(())
    }

    /// Append a change Fleet makes to the plan, and say which task it moved.
    async fn fleet_marked(
        &self,
        job_id: &JobId,
        step: &StepId,
        change: &PlanChange,
    ) -> Result<(), Adrift> {
        let at = self.now();
        let plan = self
            .store()
            .lock()
            .await
            .change_plan(job_id, change, PlanHand::Step(step), &at)
            .map_err(|why| plan_not_kept(job_id, why))?;
        self.publish(ipc::Event::JobPlanChanged(ipc::JobPlanChanged::changed(
            job_id,
            change,
            &plan,
            Actor::Fleet,
            &at,
        )));
        Ok(())
    }

    /// `job.plan_changed`, naming the group whose gate just answered.
    async fn group_moved(&self, job_id: &JobId, group: GroupId) -> Result<(), Adrift> {
        if let Some(plan) = self.plan_of(job_id).await? {
            self.publish(ipc::Event::JobPlanChanged(
                ipc::JobPlanChanged::group_moved(job_id, &plan, group, &self.now()),
            ));
        }
        Ok(())
    }

    /// The group's one commit, on the worktree its Drones wrote in.
    async fn group_commit(
        &self,
        job_id: &JobId,
        group: GroupId,
        working: &Option<Working>,
    ) -> Result<Option<String>, Adrift> {
        let Some(at_work) = working.as_ref() else {
            return Ok(None);
        };
        let (_, _, worktree) = at_work.standing();
        let job = self.load(job_id).await?;
        let titles: Vec<String> = self
            .plan_of(job_id)
            .await?
            .map(|plan| {
                plan.tasks_in(group)
                    .map(|task| format!("{}: {}", task.id(), task.title()))
                    .collect()
            })
            .unwrap_or_default();
        let message = format!(
            "{} — {group}\n\n{}\n\nArmada job {}: group {group}'s Checks passed.\n\n\
             Committed by Fleet: the Drone that did the work is denied git.\n",
            job.title().as_str(),
            titles.join("\n"),
            job.id().as_str(),
        );
        let at = CommitTime::seconds_since_epoch(
            self.now()
                .epoch_millis()
                .unwrap_or_default()
                .div_euclid(1_000),
        );
        match self.vcs().commit_all(&worktree, &message, at) {
            Ok(Committed::Made { commit }) => Ok(Some(commit)),
            Ok(Committed::NothingToCommit) => Ok(None),
            Err(cause) => Err(Adrift::NotCommitted {
                job: job_id.clone(),
                cause: Box::new(cause),
            }),
        }
    }

    /// Tell the Drone going round which of the group's tasks its round covers.
    /// After the red Checks: `crate::dispatch::Fleet::tell` sends those.
    async fn told_the_round(
        &self,
        job_id: &JobId,
        group: GroupId,
        working: &Option<Working>,
    ) -> Result<(), Adrift> {
        let Some(at_work) = working.as_ref().filter(|at_work| at_work.is(job_id)) else {
            return Ok(());
        };
        let Some(plan) = self.plan_of(job_id).await? else {
            return Ok(());
        };
        let note = PlanChanged::round(group, &plan);
        at_work.instructed(Occasion::Plan, note.text());
        let _ = at_work.session().plan_changed(&note).await;
        Ok(())
    }
}
