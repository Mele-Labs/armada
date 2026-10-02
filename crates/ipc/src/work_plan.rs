//! A Job's plan, as Bridge is served it: whole on `JobDetail::work_plan`,
//! counted on `JobSummary::tasks`, and pointed at by `job.plan_changed`.
//!
//! **`work_plan` and not `plan`**, because [`DeclaredPlan`](crate::DeclaredPlan)
//! is already on this wire and means where a step said its work would be.

use serde::{Deserialize, Serialize};

use crate::detail::Verdict;
use crate::enums::{Actor, GroupState, TaskState};
use crate::ids::{Instant, JobId, StepId};

/// An approach and its tasks, in plan order. What a Drone recorded and every
/// change since, with the history folded away.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkPlan {
    pub approach: String,
    /// Who recorded the plan as it stands. A later whole recording replaces it.
    pub recorded_by: ChangedBy,
    pub recorded_at: Instant,
    pub tasks: Vec<PlanTask>,
    /// The groups, in the order they run, each naming its tasks and its runs.
    /// Left out by a Fleet before 23.2, which ran a plan as one group.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<PlanGroup>,
}

/// One group of the plan: the tasks the step's gate runs at the end of, and how
/// each of its runs went. Since 23.2.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanGroup {
    /// `G1`, `G2`, … — minted by Fleet at the recording, never renumbered.
    pub id: String,
    /// Its tasks' ids, in plan order. Empty where a move took every one out.
    pub tasks: Vec<String>,
    pub state: GroupState,
    /// When its first run began. Absent until one has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<Instant>,
    /// When its last run was answered, with none open since.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<Instant>,
    /// Every run, oldest first. Left out where it has not run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attempts: Vec<PlanGroupRun>,
}

/// One run of a group. Since 23.2.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanGroupRun {
    /// Which run of the group, from one: a `CheckRun`'s `group_attempt`.
    pub attempt: u32,
    pub step_id: StepId,
    /// The step's run it was filed under: a `CheckRun`'s `attempt`.
    pub step_attempt: u32,
    pub started_at: Instant,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<Instant>,
    /// What its gate came to. Absent while it is open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict: Option<Verdict>,
    /// The commit a green run made, where it made one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
}

/// Who made a change: a run of a step, or a person.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "by", rename_all = "snake_case")]
pub enum ChangedBy {
    Step { step_id: StepId, attempt: u32 },
    Person,
}

/// One line of the plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanTask {
    /// `T1`, `T2`, … — stable for the life of the plan, never renumbered.
    pub id: String,
    pub title: String,
    /// What the other fields cannot hold: the exact wording, the gotcha. One
    /// line. Left out where the task has none, rather than sent empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
    /// The repository-relative paths the planner said this task touches, in
    /// the order given. **Left out where empty**, which is every task recorded
    /// before a plan carried them. Since 14.21.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scope: Vec<String>,
    /// What the planner said should prove the task, written with the plan.
    /// Left out where none was named. Since 14.21.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub expects: String,
    /// What the work said proved it, written by whoever did it. Read beside
    /// `expects`: the two disagreeing is the fact worth seeing. Since 14.21.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shown: Option<String>,
    /// A Drone's claim, or a person's. **It gates nothing.**
    pub state: TaskState,
    /// Present on a dropped task and on nothing else.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The group it runs in, `G1` and on. Since 23.2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// Present on a failed task and on nothing else: which group's Checks were
    /// still red on which run. Since 23.2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed_reason: Option<String>,
    /// Each stretch the task was marked `working`, oldest first — what Bridge
    /// places a turn in a task by. **Left out where empty**, which is every
    /// task no change ever marked working. Since 14.5.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub working_windows: Vec<WorkingWindow>,
}

/// From the change that marked a task `working` to the change that moved it
/// out. `left` is absent while it is still marked working.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkingWindow {
    pub entered: Instant,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left: Option<Instant>,
}

/// How many tasks stand where. `done` over every count but `dropped` is the
/// figure a person reads: a handed-in task and a failed one join the total and
/// neither joins `done`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskCounts {
    pub done: u32,
    pub working: u32,
    pub open: u32,
    pub dropped: u32,
    /// Since 22.0, and absent at zero, as on every Fleet before it.
    #[serde(default, skip_serializing_if = "none")]
    pub handed_in: u32,
    /// Since 22.0, and absent at zero.
    #[serde(default, skip_serializing_if = "none")]
    pub failed: u32,
}

fn none(count: &u32) -> bool {
    *count == 0
}

/// A Job's plan changed. **The counts ride along and the plan does not**: a
/// Board redraws its row from these, and an open Job reads `get_job` for the
/// rest — the stream is one bounded channel every Job shares.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobPlanChanged {
    pub job_id: JobId,
    pub tasks: TaskCounts,
    /// The task this change moved, `T1` and on. **Absent on a whole recording**,
    /// which moves every task at once. Since 23.1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    /// That task's state after the change, so a timeline can move without a
    /// read. Present exactly where `task` is. Since 23.1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<TaskState>,
    /// The group a gate's verdict moved, on that change alone. Since 23.2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// A Drone's tool call, a person's act, or Fleet marking a task's Drone
    /// starting, handing in, or its step's Checks passing.
    pub actor: Actor,
    pub at: Instant,
}

impl JobPlanChanged {
    /// The whole plan changed, and no one task names the change.
    pub fn recorded(
        job: &core_model::JobId,
        plan: &core_model::WorkPlan,
        actor: core_model::Actor,
        at: &core_model::Timestamp,
    ) -> JobPlanChanged {
        JobPlanChanged {
            job_id: job.into(),
            tasks: plan.counts().into(),
            task: None,
            state: None,
            group: None,
            actor: actor.into(),
            at: at.into(),
        }
    }

    /// A group's gate answered one of its runs.
    pub fn group_moved(
        job: &core_model::JobId,
        plan: &core_model::WorkPlan,
        group: core_model::GroupId,
        at: &core_model::Timestamp,
    ) -> JobPlanChanged {
        JobPlanChanged {
            group: Some(group.to_string()),
            ..JobPlanChanged::recorded(job, plan, core_model::Actor::Fleet, at)
        }
    }

    /// A kept change: an update names its task, an add the task it made, and a
    /// whole recording neither.
    pub fn changed(
        job: &core_model::JobId,
        change: &core_model::PlanChange,
        plan: &core_model::WorkPlan,
        actor: core_model::Actor,
        at: &core_model::Timestamp,
    ) -> JobPlanChanged {
        let moved = match change {
            core_model::PlanChange::Recorded { .. } | core_model::PlanChange::MovedGroup { .. } => {
                None
            }
            core_model::PlanChange::Updated { task, .. }
            | core_model::PlanChange::MovedTask { task, .. } => Some(*task),
            core_model::PlanChange::Added { .. } => plan.tasks().iter().map(|t| t.id()).max(),
        };
        match moved {
            Some(task) => JobPlanChanged::task_moved(job, plan, task, actor, at),
            None => JobPlanChanged::recorded(job, plan, actor, at),
        }
    }

    /// One task moved, and `plan` is what the change left: the state is read
    /// off it rather than passed, so the two cannot disagree.
    pub fn task_moved(
        job: &core_model::JobId,
        plan: &core_model::WorkPlan,
        task: core_model::TaskId,
        actor: core_model::Actor,
        at: &core_model::Timestamp,
    ) -> JobPlanChanged {
        let state = plan.task(task).map(|moved| moved.state().into());
        JobPlanChanged {
            task: state.map(|_| task.to_string()),
            state,
            ..JobPlanChanged::recorded(job, plan, actor, at)
        }
    }
}

impl From<core_model::TaskCounts> for TaskCounts {
    fn from(counts: core_model::TaskCounts) -> TaskCounts {
        TaskCounts {
            done: counts.done,
            working: counts.working,
            open: counts.open,
            dropped: counts.dropped,
            handed_in: counts.handed_in,
            failed: counts.failed,
        }
    }
}

/// A person adds a task to a Job's plan. `#897`.
///
/// **Only the title is required.** A task with nothing beyond its title is
/// legal, and `""` for `after` is the end of the list — the same spelling the
/// `add_task` MCP tool takes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddTask {
    pub title: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub scope: Vec<String>,
    #[serde(default)]
    pub expects: String,
    #[serde(default)]
    pub after: String,
}

/// Restart this task: a fresh Drone on one failed task. `#1656`.
///
/// **No body is valid**, and is the plain restart. A note is what the new
/// Drone reads first, `RestartRequested`'s shape; a blank one is refused.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestartTask {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A person moves a task, or a whole group, in a Job's plan. `#1685`.
///
/// **With `task`**, that task goes into `group`, after the task `after` names,
/// or first in the group where `after` is absent. **Without**, `group` goes
/// after the group `after` names, or first. By `after` and never by index, as
/// `add_task` places one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MovePlan {
    pub group: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
}

/// A person drops a task from a Job's plan, with a reason. `#897`.
///
/// **`reason` is never empty.** A blank one is refused at the Fleet boundary
/// for `Redirection`'s reason: a decoded request is well-formed, and a reason
/// with nothing in it is a value that cannot work — a 422 and not a 400.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DropTask {
    pub task: String,
    pub reason: String,
}

impl From<&core_model::PlanAuthor> for ChangedBy {
    fn from(author: &core_model::PlanAuthor) -> ChangedBy {
        match author {
            core_model::PlanAuthor::Step { step_id, attempt } => ChangedBy::Step {
                step_id: step_id.into(),
                attempt: attempt.number(),
            },
            core_model::PlanAuthor::Person => ChangedBy::Person,
        }
    }
}

impl From<&core_model::WorkPlan> for WorkPlan {
    /// The plan with no group run yet, which is each group read off its tasks.
    fn from(plan: &core_model::WorkPlan) -> WorkPlan {
        WorkPlan::of(plan, &core_model::GroupRuns::default())
    }
}

impl WorkPlan {
    /// The plan, and each group's runs as Fleet recorded them.
    pub fn of(plan: &core_model::WorkPlan, runs: &core_model::GroupRuns) -> WorkPlan {
        WorkPlan {
            approach: plan.approach().to_string(),
            recorded_by: plan.recorded_by().into(),
            recorded_at: plan.recorded_at().into(),
            tasks: plan
                .tasks()
                .iter()
                .map(|task| PlanTask {
                    id: task.id().to_string(),
                    title: task.title().to_string(),
                    note: task.note().to_string(),
                    scope: task
                        .scope()
                        .iter()
                        .map(|path| path.as_str().to_string())
                        .collect(),
                    expects: task.expects().to_string(),
                    shown: task.shown().map(|shown| shown.as_str().to_string()),
                    state: task.state().into(),
                    reason: task.reason().map(str::to_string),
                    group: Some(task.group().to_string()),
                    failed_reason: task.failed_reason().map(str::to_string),
                    working_windows: task
                        .working_windows()
                        .iter()
                        .map(|window| WorkingWindow {
                            entered: (&window.entered).into(),
                            left: window.left.as_ref().map(Instant::from),
                        })
                        .collect(),
                })
                .collect(),
            groups: plan
                .groups()
                .iter()
                .map(|group| PlanGroup {
                    id: group.to_string(),
                    tasks: plan.tasks_in(*group).map(|t| t.id().to_string()).collect(),
                    state: runs.state(*group, plan).into(),
                    started_at: runs.started_at(*group).map(Instant::from),
                    ended_at: runs.ended_at(*group).map(Instant::from),
                    attempts: runs
                        .attempts(*group)
                        .map(|run| PlanGroupRun {
                            attempt: run.run,
                            step_id: (&run.step).into(),
                            step_attempt: run.step_attempt.number(),
                            started_at: (&run.started_at).into(),
                            ended_at: run.ended.as_ref().map(|ended| (&ended.at).into()),
                            verdict: run
                                .ended
                                .as_ref()
                                .map(|ended| Verdict::of(run.step_attempt.number(), ended.verdict)),
                            commit: run.ended.as_ref().and_then(|ended| ended.commit.clone()),
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}
