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
}

impl core::fmt::Display for TriggerSkipped {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TriggerSkipped::NotInThisRepo { command } => {
                write!(f, "skipped: `{command}` is not in this repo")
            }
        }
    }
}
