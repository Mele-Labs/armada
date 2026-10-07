//! A Trigger: something that runs at a moment in a Job. `docs/concepts/trigger.md`.
//!
//! **Not an escalation trigger.** [`crate::TriggerKind`] is why a Job stopped
//! and asked a person; this is a thing a person asked to have run. The names
//! here carry a `Trigger` prefix so the two cannot be confused at a call site.
//!
//! **A Trigger's exit code is not a Check's.** Nothing here advances or fails
//! a gate; what a failure does is [`OnTriggerFailure`], and both of its flags
//! default off.

use alloc::string::String;

use crate::envelope::Timestamp;
use crate::job::{StepId, WorkflowId};

/// The moment in a Job a Trigger runs at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TriggerWhen {
    StepStarts,
    StepPasses,
    /// **The delivering step's entry**, because `delivers` is read on entry
    /// (`crates/fleet/src/landing.rs`). It names no step of its own.
    PrOpened,
}

impl TriggerWhen {
    pub const ALL: &'static [TriggerWhen] = &[
        TriggerWhen::StepStarts,
        TriggerWhen::StepPasses,
        TriggerWhen::PrOpened,
    ];

    /// The word a file spells it with.
    pub fn as_wire(self) -> &'static str {
        match self {
            TriggerWhen::StepStarts => "step_starts",
            TriggerWhen::StepPasses => "step_passes",
            TriggerWhen::PrOpened => "pr_opened",
        }
    }

    pub fn from_wire(value: &str) -> Option<TriggerWhen> {
        TriggerWhen::ALL
            .iter()
            .copied()
            .find(|when| when.as_wire() == value)
    }
}

/// What a Trigger runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggerRuns {
    /// A Command named in the repository's `armada.yml`. Fleet runs it with
    /// no Drone.
    Command(String),
    /// A skill a Drone runs. **Modelled and not executed yet.**
    Skill(String),
}

/// What a failure does. **Both off unless the file says otherwise.**
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OnTriggerFailure {
    /// The Job waits on the failure rather than carrying on.
    pub block: bool,
    /// A repair Drone is sent to find the fix.
    pub repair: bool,
}

/// The three places a Trigger is set, **least specific first, and the order is
/// the rule**: `Ord` is what `config` compares, so a place is ranked by where
/// its variant is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TriggerSource {
    /// Compiled into the binary. **Ships none.** The level exists so a later
    /// default has somewhere to be written.
    Armada,
    /// `.armada/triggers/` in the repository, read from `main` only.
    Repository,
    /// This machine's own. **Not Kit**, which travels: a machine's Trigger is
    /// about this installation and never leaves it.
    Machine,
}

impl TriggerSource {
    pub fn as_wire(self) -> &'static str {
        match self {
            TriggerSource::Armada => "armada",
            TriggerSource::Repository => "repository",
            TriggerSource::Machine => "machine",
        }
    }
}

impl core::fmt::Display for TriggerSource {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            TriggerSource::Armada => "carried by Armada",
            TriggerSource::Repository => "from the repository",
            TriggerSource::Machine => "from this machine",
        })
    }
}

/// What makes two Triggers the same one: **when, step and name.** The most
/// specific place's copy replaces the others whole, with no field merge.
///
/// The workflow a Trigger applies to is not part of it, so a machine Trigger
/// narrowed to one workflow still replaces a repository one that applied to
/// all of them.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TriggerIdentity {
    pub when: TriggerWhen,
    pub step: Option<StepId>,
    pub name: String,
}

/// One Trigger, as a person wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trigger {
    name: String,
    when: TriggerWhen,
    /// `None` is every workflow.
    workflow: Option<WorkflowId>,
    /// `None` is every step. Never set with [`TriggerWhen::PrOpened`], which is
    /// the delivering step's entry whichever step that is.
    step: Option<StepId>,
    runs: TriggerRuns,
    on_failure: OnTriggerFailure,
}

impl Trigger {
    pub fn new(name: String, when: TriggerWhen, runs: TriggerRuns) -> Trigger {
        Trigger {
            name,
            when,
            workflow: None,
            step: None,
            runs,
            on_failure: OnTriggerFailure::default(),
        }
    }

    pub fn in_workflow(self, workflow: WorkflowId) -> Trigger {
        Trigger {
            workflow: Some(workflow),
            ..self
        }
    }

    pub fn at_step(self, step: StepId) -> Trigger {
        Trigger {
            step: Some(step),
            ..self
        }
    }

    pub fn with_failure(self, on_failure: OnTriggerFailure) -> Trigger {
        Trigger { on_failure, ..self }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn when(&self) -> TriggerWhen {
        self.when
    }

    pub fn workflow(&self) -> Option<&WorkflowId> {
        self.workflow.as_ref()
    }

    pub fn step(&self) -> Option<&StepId> {
        self.step.as_ref()
    }

    pub fn runs(&self) -> &TriggerRuns {
        &self.runs
    }

    pub fn on_failure(&self) -> OnTriggerFailure {
        self.on_failure
    }

    pub fn identity(&self) -> TriggerIdentity {
        TriggerIdentity {
            when: self.when,
            step: self.step.clone(),
            name: self.name.clone(),
        }
    }

    /// Whether this Trigger is for `workflow`, at `when`, on `step`. A
    /// [`TriggerWhen::PrOpened`] Trigger is asked of the delivering step.
    pub fn applies(&self, workflow: &WorkflowId, when: TriggerWhen, step: &StepId) -> bool {
        self.when == when
            && self.workflow.as_ref().is_none_or(|own| own == workflow)
            && self.step.as_ref().is_none_or(|own| own == step)
    }
}

/// What a Trigger comes to against one repository's Commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggerResolution {
    /// Fleet runs the Command. **`asks_first` is the Command's `destructive`
    /// flag**, carried so the owner is asked before it runs; nothing enforces
    /// that yet.
    Command { name: String, asks_first: bool },
    /// A Drone runs the skill. Not executed yet.
    Skill { name: String },
    /// Marked on the Job and not run. Never a refusal.
    Skipped(TriggerSkipped),
}

/// Why a Trigger will not run in this repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriggerSkipped {
    /// It names a Command this repository's `armada.yml` does not declare.
    NotInThisRepo { command: String },
    /// It names a skill, which a Drone runs and Fleet does not yet. **Never a
    /// resolution**: a skill resolves to [`TriggerResolution::Skill`], and it
    /// is firing that finds nothing to run it with.
    SkillNotRun { skill: String },
}

impl core::fmt::Display for TriggerSkipped {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TriggerSkipped::NotInThisRepo { command } => {
                write!(f, "skipped: `{command}` is not in this repo")
            }
            TriggerSkipped::SkillNotRun { skill } => {
                write!(f, "skipped: skill `{skill}` is not run yet")
            }
        }
    }
}

/// One Trigger as it stood when the Job was approved, **bound to the step it
/// fires on**. A Trigger with no step fires on every step, and freezing it
/// writes one of these per step of the Job's workflow, so firing never asks the
/// catalogue again and a file saved later changes nothing for this Job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenTrigger {
    pub name: String,
    pub when: TriggerWhen,
    /// The step it fires on. For [`TriggerWhen::PrOpened`] that is the
    /// workflow's delivering step.
    pub step: StepId,
    pub source: TriggerSource,
    pub resolution: TriggerResolution,
    pub on_failure: OnTriggerFailure,
}

/// Where one firing stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerState {
    /// Marked and not run: [`TriggerSkipped`] says why.
    Skipped,
    Running,
    /// Exited zero. A Trigger has no `expect_exit_code`, which is a Check's.
    Passed,
    /// Exited another way: a non-zero code, a signal, a timeout, or a program
    /// that never started.
    Failed,
    /// A destructive Command, held until the owner says it may run. **Nothing
    /// asks him yet**, so for now it stays here.
    AwaitingOwner,
}

impl TriggerState {
    pub fn as_wire(self) -> &'static str {
        match self {
            TriggerState::Skipped => "skipped",
            TriggerState::Running => "running",
            TriggerState::Passed => "passed",
            TriggerState::Failed => "failed",
            TriggerState::AwaitingOwner => "awaiting_owner",
        }
    }

    pub fn from_wire(value: &str) -> Option<TriggerState> {
        [
            TriggerState::Skipped,
            TriggerState::Running,
            TriggerState::Passed,
            TriggerState::Failed,
            TriggerState::AwaitingOwner,
        ]
        .into_iter()
        .find(|state| state.as_wire() == value)
    }
}

/// One firing of a [`FrozenTrigger`] at its moment. **A record and never a
/// verdict**: nothing reads `state` to advance or fail a gate, and a failure
/// is a line here and in the Job's log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerFiring {
    pub name: String,
    pub when: TriggerWhen,
    pub step: StepId,
    pub source: TriggerSource,
    /// Carried from the Trigger and **not acted on yet**.
    pub on_failure: OnTriggerFailure,
    pub state: TriggerState,
    /// Why, where `state` is [`TriggerState::Skipped`].
    pub skipped: Option<TriggerSkipped>,
    /// Absent for a signal, a timeout and a program that never started.
    pub exit_code: Option<i32>,
    pub started_at: Timestamp,
    /// Absent while it runs, and while it waits on the owner.
    pub ended_at: Option<Timestamp>,
}

impl TriggerFiring {
    fn of(trigger: &FrozenTrigger, state: TriggerState, at: Timestamp) -> TriggerFiring {
        TriggerFiring {
            name: trigger.name.clone(),
            when: trigger.when,
            step: trigger.step.clone(),
            source: trigger.source,
            on_failure: trigger.on_failure,
            state,
            skipped: None,
            exit_code: None,
            started_at: at,
            ended_at: None,
        }
    }

    pub fn running(trigger: &FrozenTrigger, at: Timestamp) -> TriggerFiring {
        TriggerFiring::of(trigger, TriggerState::Running, at)
    }

    pub fn skipped(trigger: &FrozenTrigger, why: TriggerSkipped, at: Timestamp) -> TriggerFiring {
        TriggerFiring {
            skipped: Some(why),
            ended_at: Some(at.clone()),
            ..TriggerFiring::of(trigger, TriggerState::Skipped, at)
        }
    }

    pub fn awaiting_the_owner(trigger: &FrozenTrigger, at: Timestamp) -> TriggerFiring {
        TriggerFiring::of(trigger, TriggerState::AwaitingOwner, at)
    }

    /// This firing, ended. **Zero is the only pass.**
    pub fn ended(self, exit_code: Option<i32>, at: Timestamp) -> TriggerFiring {
        let state = match exit_code {
            Some(0) => TriggerState::Passed,
            _ => TriggerState::Failed,
        };
        TriggerFiring {
            state,
            exit_code,
            ended_at: Some(at),
            ..self
        }
    }
}
