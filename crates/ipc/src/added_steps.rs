//! Steps added to one Job on the wire: placed at approval, added to a running
//! Job, removed before they fire, and how each stands. `docs/concepts/trigger.md`,
//! *Steps added to one Job*. **Since 23.68.**
//!
//! **DTOs, never `core_model::AddedStep`.** `fleet::added_steps` is where the
//! conversion is. An addition sits beside the Job's frozen workflow and never in
//! it, so none of this is on `JobDetail`'s workflow: `JobDetail.additions` is
//! its own list.
//!
//! **Keeping one for every Job is `save_trigger`'s**, with `kept_from` naming
//! the addition: the Trigger a person saves is the one the editor drew, and
//! Fleet only records where it went.

use serde::{Deserialize, Serialize};

use crate::ids::{Instant, JobId, StepId};
use crate::triggers::{TriggerFiringState, TriggerMoment, TriggerRepair, TriggerScope};

/// What an added step runs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AddedRuns {
    /// A Command named in the repository's `armada.yml`, which Fleet runs with
    /// no Drone.
    Script { command: String },
    /// A skill a Drone runs. **Recorded `skipped`**, as a skill Trigger is.
    Skill { skill: String },
    /// A short brief for a Drone. **Recorded `skipped` and not run**: a step a
    /// Drone works needs a gate, and Fleet has none for it.
    Drone { brief: String },
}

/// One step to add: where it fires, what it runs and what a failure does.
/// `approve_dispatch`'s `additions` and `add_job_step`'s body.
///
/// **A place is a moment and a step**, the way a Trigger's is: before a step is
/// `step_starts`, after it is `step_passes`, and `pr_opened` hangs from the
/// delivering step. The gap before the pull request opens is the delivering
/// step's `step_starts`, and the gap after is `pr_opened`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddStep {
    pub runs: AddedRuns,
    pub when: TriggerMoment,
    pub step: StepId,
    /// The Job waits on a failure. **Carried and not acted on yet.**
    #[serde(default)]
    pub block: bool,
    /// A repair Drone is sent on a failure. **Carried and not acted on yet.**
    #[serde(default)]
    pub repair: bool,
}

/// `edit_job_step`'s body: the added step and the switches to change. A switch
/// left out is left as it is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditAddedStep {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair: Option<bool>,
}

/// `remove_job_step`'s body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveAddedStep {
    pub id: String,
}

/// What `remove_job_step` removed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddedStepRemoved {
    pub id: String,
}

/// When an addition was placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AddedPlaced {
    /// With the approval press.
    Approval,
    /// On a Job already underway.
    Running,
}

/// Why an addition is not run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AddedSkipReason {
    /// It names a Command this repository does not declare.
    NotInThisRepo,
    /// **No longer produced** (23.73): a skill runs on a side Drone. Kept so
    /// an older row still reads.
    SkillNotRun,
    /// **No longer produced** (23.73), as `SkillNotRun`.
    DroneStepNotRun,
    /// It failed and held the Job, and the owner skipped it. Since 23.68.
    ByOwner,
}

/// An addition that will not run, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddedSkip {
    pub reason: AddedSkipReason,
    /// The sentence Fleet writes in the Job's log, rendered and never matched on.
    pub said: String,
}

/// One step added to a Job, and how it stands. `JobDetail.additions`, and the
/// row `job.addition_changed` carries whole.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddedStep {
    /// `a1`, `a2` and so on, in the order they were added to this Job.
    pub id: String,
    pub runs: AddedRuns,
    pub when: TriggerMoment,
    pub step: StepId,
    #[serde(default)]
    pub block: bool,
    #[serde(default)]
    pub repair: bool,
    pub placed: AddedPlaced,
    pub added_at: Instant,
    /// `pending` until its moment has come, which is when it can still be
    /// removed. The latest firing's after that.
    pub state: TriggerFiringState,
    /// Why, where `state` is `skipped`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped: Option<AddedSkip>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<Instant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<Instant>,
    /// The Job's log line for this firing, `JobTrigger.log_at`'s reading: the
    /// note whose `at` is this and whose `addition` field is `id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_at: Option<Instant>,
    /// Where it was kept for every Job, once it was. Absent until then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kept: Option<TriggerScope>,
    /// What its repair has come to, as `JobTrigger.repair` carries a Trigger's.
    /// Present from the first repair Drone, and absent where `repair` is off or
    /// the step never failed. Since 23.72.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair_record: Option<TriggerRepair>,
}

/// `save_trigger`'s `kept_from`: the addition this save keeps for every Job.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeptFrom {
    pub job_id: JobId,
    pub addition_id: String,
}

/// `job.addition_changed`: one of a Job's added steps moved, carried whole so a
/// client replaces its row without a read. One message per state change, and one
/// when it is added, removed or kept.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobAdditionChanged {
    pub job_id: JobId,
    pub addition: AddedStep,
    /// The addition was removed, and the row is what it was.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub removed: bool,
    pub at: Instant,
}

impl From<core_model::Placed> for AddedPlaced {
    fn from(placed: core_model::Placed) -> AddedPlaced {
        match placed {
            core_model::Placed::AtApproval => AddedPlaced::Approval,
            core_model::Placed::WhileRunning => AddedPlaced::Running,
        }
    }
}

impl From<core_model::Kept> for TriggerScope {
    fn from(kept: core_model::Kept) -> TriggerScope {
        match kept {
            core_model::Kept::Repository => TriggerScope::Repository,
            core_model::Kept::Machine => TriggerScope::Machine,
        }
    }
}

impl From<TriggerScope> for core_model::Kept {
    fn from(scope: TriggerScope) -> core_model::Kept {
        match scope {
            TriggerScope::Repository => core_model::Kept::Repository,
            TriggerScope::Machine => core_model::Kept::Machine,
        }
    }
}

impl From<&core_model::AddedKind> for AddedRuns {
    fn from(kind: &core_model::AddedKind) -> AddedRuns {
        match kind {
            core_model::AddedKind::Script { command } => AddedRuns::Script {
                command: command.clone(),
            },
            core_model::AddedKind::Skill { skill } => AddedRuns::Skill {
                skill: skill.clone(),
            },
            core_model::AddedKind::Drone { brief } => AddedRuns::Drone {
                brief: brief.clone(),
            },
        }
    }
}

impl From<&AddedRuns> for core_model::AddedKind {
    fn from(runs: &AddedRuns) -> core_model::AddedKind {
        match runs {
            AddedRuns::Script { command } => core_model::AddedKind::Script {
                command: command.clone(),
            },
            AddedRuns::Skill { skill } => core_model::AddedKind::Skill {
                skill: skill.clone(),
            },
            AddedRuns::Drone { brief } => core_model::AddedKind::Drone {
                brief: brief.clone(),
            },
        }
    }
}

impl From<&core_model::NotRun> for AddedSkip {
    fn from(not_run: &core_model::NotRun) -> AddedSkip {
        AddedSkip {
            reason: match not_run {
                core_model::NotRun::NotInThisRepo { .. } => AddedSkipReason::NotInThisRepo,
                core_model::NotRun::SkillNotRun { .. } => AddedSkipReason::SkillNotRun,
                core_model::NotRun::DroneStepNotRun => AddedSkipReason::DroneStepNotRun,
                core_model::NotRun::ByOwner => AddedSkipReason::ByOwner,
            },
            said: not_run.to_string(),
        }
    }
}

impl From<&core_model::AddedStep> for AddedStep {
    /// An addition as it stands. `log_at` is the instant the Job's log line for
    /// its latest firing carries, which `fleet::added_steps` stamps from the
    /// firing's own end.
    fn from(added: &core_model::AddedStep) -> AddedStep {
        let fired = added.fired.as_ref();
        let ended = fired.and_then(|fired| fired.ended_at.as_ref());
        AddedStep {
            id: added.id.clone(),
            runs: (&added.kind).into(),
            when: added.when.into(),
            step: StepId::from(&added.step),
            block: added.on_failure.block,
            repair: added.on_failure.repair,
            placed: added.placed.into(),
            added_at: Instant::from(&added.added_at),
            state: fired.map_or(TriggerFiringState::Pending, |fired| fired.state.into()),
            skipped: fired
                .and_then(|fired| fired.not_run.as_ref())
                .map(AddedSkip::from),
            exit_code: fired.and_then(|fired| fired.exit_code),
            started_at: fired.map(|fired| Instant::from(&fired.started_at)),
            ended_at: ended.map(Instant::from),
            log_at: fired.and_then(|fired| match fired.state {
                core_model::TriggerState::Running | core_model::TriggerState::AwaitingOwner => None,
                _ => Some(Instant::from(ended.unwrap_or(&fired.started_at))),
            }),
            kept: added.kept.map(TriggerScope::from),
            repair_record: (added.repair.tries > 0).then(|| TriggerRepair::from(&added.repair)),
        }
    }
}
