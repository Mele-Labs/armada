//! What a rule does: the acts of the design's table, and the kind of each, which is what an
//! autonomy default is set against.
//!
//! **A kind is a row of the table and not its Kind column.** Four rows are Fleet's and two of them
//! default one way and two the other, so a default per column could not say what the table says.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::lifecycle::StateId;

use super::autonomy::Autonomy;
use super::ids::Words;
use super::select::Target;

/// The kind of an act, which is the unit an autonomy default is set for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActKind {
    /// Assign, move or close a ticket.
    TrackerWrite,
    /// Request reviewers, mark ready, merge, turn on auto-merge.
    ForgeWrite,
    /// A message from a template, which speaks as the owner.
    Draft,
    /// Start a Job.
    StartJob,
    /// Start a watch.
    StartWatch,
    /// Tell or wake the Job's agent.
    TellAgent,
    /// Run a Manifest command.
    RunCommand,
    /// Run a skill, or a Drone with a prompt.
    RunAgent,
}

impl ActKind {
    pub const ALL: &'static [ActKind] = &[
        ActKind::TrackerWrite,
        ActKind::ForgeWrite,
        ActKind::Draft,
        ActKind::StartJob,
        ActKind::StartWatch,
        ActKind::TellAgent,
        ActKind::RunCommand,
        ActKind::RunAgent,
    ];

    pub fn as_wire(self) -> &'static str {
        match self {
            ActKind::TrackerWrite => "tracker_write",
            ActKind::ForgeWrite => "forge_write",
            ActKind::Draft => "draft",
            ActKind::StartJob => "start_job",
            ActKind::StartWatch => "start_watch",
            ActKind::TellAgent => "tell_agent",
            ActKind::RunCommand => "run_command",
            ActKind::RunAgent => "run_agent",
        }
    }

    pub fn from_wire(value: &str) -> Option<ActKind> {
        ActKind::ALL.iter().copied().find(|kind| kind.as_wire() == value)
    }

    /// What a rule does of this kind when nobody has said otherwise. **A write that speaks for the
    /// owner, or starts a Job, asks. What Fleet does inside the Job acts.**
    pub fn default_autonomy(self) -> Autonomy {
        match self {
            ActKind::TrackerWrite | ActKind::ForgeWrite | ActKind::Draft | ActKind::StartJob => Autonomy::Asks,
            ActKind::StartWatch | ActKind::TellAgent | ActKind::RunCommand | ActKind::RunAgent => Autonomy::Acts,
        }
    }
}

/// A write to the tracker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrackerWrite {
    /// Assign to someone, by the tracker's own name for them.
    Assign { to: Words },
    /// Move to a state of the ticket's lifecycle. **The state, and not the tracker's word for it**:
    /// the word is the Workspace's to say, and the doing looks it up.
    Move { to: StateId },
    Close,
}

/// A write to the forge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ForgeWrite {
    /// Ask these people, by the forge's own names, to review. At least one.
    RequestReviewers { from: Vec<Words> },
    MarkReady,
    Merge,
    AutoMerge,
}

/// One thing a rule does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Act {
    Tracker { write: TrackerWrite, target: Target },
    Forge { write: ForgeWrite, target: Target },
    /// A message from a template, left on `target`.
    Draft { template: Words, target: Target },
    /// Start a Job on a prompt, in a workflow (by its slug) where one is named.
    StartJob { workflow: Option<Words>, prompt: Words },
    /// Start a watch from a recipe, with the values it needs. **`deploy` names the deploy it
    /// follows**, the key of an instance of the `deploy` lifecycle the Job takes on with it, so the
    /// watch's readings are where that deploy stands. A watch with none follows nothing.
    StartWatch { recipe: Words, values: BTreeMap<Words, Words>, deploy: Option<Words> },
    /// Say something to the Job's agent, which wakes it.
    Tell { message: Words },
    /// Wake the Job's agent with nothing to say.
    Wake,
    /// Run a command the Manifest declares.
    RunCommand { command: Words },
    RunSkill { skill: Words },
    RunDrone { prompt: Words },
}

impl Act {
    pub fn kind(&self) -> ActKind {
        match self {
            Act::Tracker { .. } => ActKind::TrackerWrite,
            Act::Forge { .. } => ActKind::ForgeWrite,
            Act::Draft { .. } => ActKind::Draft,
            Act::StartJob { .. } => ActKind::StartJob,
            Act::StartWatch { .. } => ActKind::StartWatch,
            Act::Tell { .. } | Act::Wake => ActKind::TellAgent,
            Act::RunCommand { .. } => ActKind::RunCommand,
            Act::RunSkill { .. } | Act::RunDrone { .. } => ActKind::RunAgent,
        }
    }

    /// The instances it is done to, for the acts that are done to some.
    pub fn target(&self) -> Option<&Target> {
        match self {
            Act::Tracker { target, .. } | Act::Forge { target, .. } | Act::Draft { target, .. } => Some(target),
            _ => None,
        }
    }
}
