//! A step added to one Job. `docs/concepts/trigger.md`, *Steps added to one Job*.
//!
//! **Beside the frozen workflow and never in it.** The workflow is the yardstick
//! the work is judged against, and a person adding a formatter after step 3
//! must not move it. An addition is anchored to a moment (a step's start, a
//! step's pass, or the pull request opening) and fires there as a Trigger does,
//! for this Job only.
//!
//! **A record and never a verdict**, as a Trigger's firing is: nothing here
//! advances or fails a gate.

use alloc::string::String;

use crate::envelope::Timestamp;
use crate::job::{FrozenWorkflow, Job, JobStatus, StepId, StepState};
use crate::trigger::{can_hold, OnTriggerFailure, RepairRecord, TriggerState, TriggerWhen};

/// What an added step runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddedKind {
    /// A Command named in the repository's `armada.yml`. Fleet runs it with no
    /// Drone, as it runs a command Trigger.
    Script { command: String },
    /// A skill a side Drone runs, on a branch cut from the Job's.
    Skill { skill: String },
    /// A short brief for a side Drone, on a branch cut from the Job's. **It
    /// gates nothing**: the frozen workflow's step rows are the only gate, and
    /// the Drone's work is held as a fix for the owner to place.
    Drone { brief: String },
}

impl AddedKind {
    pub fn as_wire(&self) -> &'static str {
        match self {
            AddedKind::Script { .. } => "script",
            AddedKind::Skill { .. } => "skill",
            AddedKind::Drone { .. } => "drone",
        }
    }

    /// The command, the skill or the brief.
    pub fn text(&self) -> &str {
        match self {
            AddedKind::Script { command } => command,
            AddedKind::Skill { skill } => skill,
            AddedKind::Drone { brief } => brief,
        }
    }

    pub fn from_wire(kind: &str, text: String) -> Option<AddedKind> {
        match kind {
            "script" => Some(AddedKind::Script { command: text }),
            "skill" => Some(AddedKind::Skill { skill: text }),
            "drone" => Some(AddedKind::Drone { brief: text }),
            _ => None,
        }
    }
}

/// When it was placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placed {
    /// At the approval press, with the workflow and the Triggers.
    AtApproval,
    /// On a Job already underway.
    WhileRunning,
}

impl Placed {
    pub fn as_wire(self) -> &'static str {
        match self {
            Placed::AtApproval => "approval",
            Placed::WhileRunning => "running",
        }
    }

    pub fn from_wire(value: &str) -> Option<Placed> {
        [Placed::AtApproval, Placed::WhileRunning]
            .into_iter()
            .find(|placed| placed.as_wire() == value)
    }
}

/// Where a Script or a Skill was kept for every Job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kept {
    Repository,
    Machine,
}

impl Kept {
    pub fn as_wire(self) -> &'static str {
        match self {
            Kept::Repository => "repository",
            Kept::Machine => "machine",
        }
    }

    pub fn from_wire(value: &str) -> Option<Kept> {
        [Kept::Repository, Kept::Machine]
            .into_iter()
            .find(|kept| kept.as_wire() == value)
    }
}

/// Why an addition is not run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotRun {
    /// It names a Command this repository's `armada.yml` does not declare.
    NotInThisRepo { command: String },
    /// **No longer produced** (23.73): a skill runs on a side Drone. Kept so a
    /// row an earlier build wrote still reads.
    SkillNotRun { skill: String },
    /// **No longer produced** (23.73), as [`NotRun::SkillNotRun`].
    DroneStepNotRun,
    /// The owner skipped it while it held the Job.
    ByOwner,
}

impl core::fmt::Display for NotRun {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NotRun::NotInThisRepo { command } => {
                write!(f, "skipped: `{command}` is not in this repo")
            }
            NotRun::SkillNotRun { skill } => {
                write!(f, "skipped: skill `{skill}` is not run yet")
            }
            NotRun::DroneStepNotRun => f.write_str("skipped: a Drone step is not run yet"),
            NotRun::ByOwner => f.write_str("skipped by you while it held the Job"),
        }
    }
}

/// What came of an addition's moment. **Absent until the moment came**, which
/// is how a pending addition reads and why one can still be removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fired {
    pub state: TriggerState,
    pub not_run: Option<NotRun>,
    pub exit_code: Option<i32>,
    pub started_at: Timestamp,
    pub ended_at: Option<Timestamp>,
}

impl Fired {
    pub fn running(at: Timestamp) -> Fired {
        Fired {
            state: TriggerState::Running,
            not_run: None,
            exit_code: None,
            started_at: at,
            ended_at: None,
        }
    }

    pub fn skipped(why: NotRun, at: Timestamp) -> Fired {
        Fired {
            state: TriggerState::Skipped,
            not_run: Some(why),
            exit_code: None,
            started_at: at.clone(),
            ended_at: Some(at),
        }
    }

    pub fn awaiting_the_owner(at: Timestamp) -> Fired {
        Fired {
            state: TriggerState::AwaitingOwner,
            not_run: None,
            exit_code: None,
            started_at: at,
            ended_at: None,
        }
    }

    /// Ended. **Zero is the only pass**, as for a Trigger. A failure of one with
    /// `repairs` on is [`TriggerState::Repairing`] and has no end, since the
    /// repair is what settles it. One that `blocks` is [`TriggerState::Held`].
    pub fn ended(
        self,
        exit_code: Option<i32>,
        blocks: bool,
        repairs: bool,
        at: Timestamp,
    ) -> Fired {
        let (state, ended_at) = match exit_code {
            Some(0) => (TriggerState::Passed, Some(at)),
            _ if repairs => (TriggerState::Repairing, None),
            _ if blocks => (TriggerState::Held, Some(at)),
            _ => (TriggerState::Failed, Some(at)),
        };
        Fired {
            state,
            exit_code,
            ended_at,
            ..self
        }
    }
}

/// One step added to one Job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedStep {
    /// `a1`, `a2` and so on, in the order they were added to this Job.
    pub id: String,
    pub kind: AddedKind,
    pub when: TriggerWhen,
    /// The step it hangs from. For [`TriggerWhen::PrOpened`], the delivering one.
    pub step: StepId,
    /// `block` and `repair` are both acted on, a Script's as a Trigger's are.
    /// A Skill's and a Drone's `repair` is always off.
    pub on_failure: OnTriggerFailure,
    pub placed: Placed,
    pub added_at: Timestamp,
    /// The latest firing. A step run again fires it again.
    pub fired: Option<Fired>,
    pub kept: Option<Kept>,
    /// What its repair has come to. Empty for a step that never failed or has
    /// `repair` off.
    pub repair: RepairRecord,
}

impl AddedStep {
    /// Whether this step holds its Job: it blocks, a hold can stand where it
    /// is, and its latest firing failed and was not settled, or is a destructive
    /// Command still asking him. A repair under way is still a failure nobody
    /// has settled.
    pub fn holds_the_job(&self, workflow: &FrozenWorkflow) -> bool {
        self.on_failure.block
            && can_hold(workflow, self.when, &self.step)
            && self.fired.as_ref().is_some_and(|fired| {
                fired.state.holds_a_blocking_job()
                    || fired.state == TriggerState::AwaitingOwner
                    || self.repair.side_run_in_flight(fired.state)
            })
    }
}

/// Why a place is not one an added step can go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Misplaced {
    /// The workflow has no such step.
    NoSuchStep(StepId),
    /// `pr_opened` hangs from the delivering step, which is where the pull
    /// request opens, and this step is not it.
    NotTheDeliveringStep(StepId),
    /// A workflow that delivers nowhere opens no pull request.
    NothingDelivers,
}

impl core::fmt::Display for Misplaced {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Misplaced::NoSuchStep(step) => {
                write!(f, "the workflow has no step `{}`", step.as_str())
            }
            Misplaced::NotTheDeliveringStep(step) => write!(
                f,
                "the pull request opens at the delivering step, and `{}` is not it",
                step.as_str()
            ),
            Misplaced::NothingDelivers => {
                f.write_str("this workflow opens no pull request to add a step after")
            }
        }
    }
}

/// Whether a place exists in `workflow`. **Whether it is still ahead is
/// [`behind`]'s question**, which a Job answers and a workflow cannot.
pub fn placeable(
    workflow: &FrozenWorkflow,
    when: TriggerWhen,
    step: &StepId,
) -> Result<(), Misplaced> {
    if workflow.step(step).is_none() {
        return Err(Misplaced::NoSuchStep(step.clone()));
    }
    if when == TriggerWhen::PrOpened {
        match workflow.delivering_step() {
            None => return Err(Misplaced::NothingDelivers),
            Some(delivering) if delivering.id() != step => {
                return Err(Misplaced::NotTheDeliveringStep(step.clone()))
            }
            Some(_) => {}
        }
    }
    Ok(())
}

/// Why a place on a running Job is behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Behind {
    /// The Job has not been approved, so nothing runs and the approval carries it.
    NotApproved,
    /// The Job is over.
    Ended,
    /// The step has started, so its start has come and gone.
    Started,
    /// The step has passed, or the Job is on a step after it.
    Passed,
}

impl Behind {
    pub fn as_wire(self) -> &'static str {
        match self {
            Behind::NotApproved => "not_approved",
            Behind::Ended => "ended",
            Behind::Started => "started",
            Behind::Passed => "passed",
        }
    }
}

/// Whether the moment `(when, step)` has already come on `job`. **A gap behind
/// the current step is refused**; a place at or after it is not.
///
/// `pr_opened` is the delivering step's entry, so it is behind once that step
/// has started. `step_passes` is behind once the step has advanced or the Job
/// stands on a step after it. Read off the step rows, which a loop return
/// rewrites, so a step sent back is ahead again.
pub fn behind(job: &Job, when: TriggerWhen, step: &StepId) -> Option<Behind> {
    match job.status() {
        JobStatus::Proposing | JobStatus::AwaitingApproval => return Some(Behind::NotApproved),
        status if status.is_terminal() => return Some(Behind::Ended),
        _ => {}
    }
    let row = job.step(step)?;
    let current = job.current_step().map(|current| current.ordinal());
    match when {
        TriggerWhen::StepStarts | TriggerWhen::PrOpened => {
            (row.state() != StepState::NotStarted).then_some(Behind::Started)
        }
        TriggerWhen::StepPasses => {
            let past = row.state() == StepState::Advanced
                || current.is_some_and(|current| row.ordinal() < current);
            past.then_some(Behind::Passed)
        }
    }
}
