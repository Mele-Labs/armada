//! What a rule looks at: which instances, and which facts of the Job. Conditions and the targets
//! of acts are built from the same two ideas, [`Among`] and a state.
//!
//! **A closed set, and every member is something Fleet already holds.** An instance's standing is
//! read from the world, and the Job's workflow, step and pull request are Fleet's own. There is no
//! condition on what an agent said, on a tracker field no state reads, or on anything else a
//! reader cannot see, so a rule that wants one is refused when it is read.

use crate::lifecycle::{LifecycleId, StateId};
use crate::workflow::StepId;

use super::ids::Words;

/// Which instances. The first three begin at the instance that entered a state, so they mean
/// nothing to a rule whose [`When`](super::When) has none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Among {
    /// The instance whose entering a state set the rule off.
    Entered,
    /// What the entered instance came from and the Job holds, of one lifecycle: a pull request's
    /// tickets.
    LinkedFrom(LifecycleId),
    /// What came from the entered instance and the Job holds, of one lifecycle: a ticket's pull
    /// requests, a Job's Jobs.
    LinkingTo(LifecycleId),
    /// Every instance of one lifecycle the Job holds.
    Held(LifecycleId),
}

impl Among {
    pub fn needs_an_instance(&self) -> bool {
        !matches!(self, Among::Held(_))
    }

    /// The lifecycle its instances are of, given the lifecycle of the one that entered.
    pub fn lifecycle<'a>(&'a self, entered: Option<&'a LifecycleId>) -> Option<&'a LifecycleId> {
        match self {
            Among::Entered => entered,
            Among::LinkedFrom(lifecycle) | Among::LinkingTo(lifecycle) | Among::Held(lifecycle) => Some(lifecycle),
        }
    }
}

/// How many of the instances `among` selects must be in the state.
///
/// **`Every` needs at least one**: all of nothing is true of any Job, and a rule that wrote to a
/// tracker because a Job held no tickets would be acting on a vacuum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Quantity {
    /// There is one, and every one is in the state.
    Every,
    /// At least one is.
    Any,
    /// None is. Holds where there are none.
    No,
}

impl Quantity {
    pub const ALL: &'static [Quantity] = &[Quantity::Every, Quantity::Any, Quantity::No];

    pub fn as_wire(self) -> &'static str {
        match self {
            Quantity::Every => "every",
            Quantity::Any => "any",
            Quantity::No => "no",
        }
    }

    pub fn from_wire(value: &str) -> Option<Quantity> {
        Quantity::ALL.iter().copied().find(|quantity| quantity.as_wire() == value)
    }
}

/// What a Job shows of itself, which is Fleet's to read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JobFacts {
    /// The workflow it follows, by the slug its file gives it.
    pub workflow: Option<Words>,
    /// The step it is on.
    pub step: Option<StepId>,
    /// Whether it has opened a pull request.
    pub pull_request_opened: bool,
}

/// A fact about the Job itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobCondition {
    /// It follows this workflow, by the slug its file gives it.
    Workflow(Words),
    /// It is on this step.
    AtStep(StepId),
    /// It has opened a pull request.
    PullRequestOpened,
}

/// One thing a rule asks, and **all of a rule's conditions must hold** for it to do anything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Condition {
    /// Of the instances `among` selects, `quantity` of them stand in `in_state`.
    Instances { among: Among, quantity: Quantity, in_state: StateId },
    Job(JobCondition),
}

impl Condition {
    pub fn among(&self) -> Option<&Among> {
        match self {
            Condition::Instances { among, .. } => Some(among),
            Condition::Job(_) => None,
        }
    }
}

/// Which instances an act is done to: those `among` selects, **standing in `in_state` where one is
/// given** and in any state where not. An act that names none does nothing, since there is nothing
/// to do it to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub among: Among,
    pub in_state: Option<StateId>,
}

impl Target {
    /// Every instance of `lifecycle` the Job holds, in any state.
    pub fn held(lifecycle: LifecycleId) -> Target {
        Target { among: Among::Held(lifecycle), in_state: None }
    }

    /// The instance that entered a state.
    pub fn entered() -> Target {
        Target { among: Among::Entered, in_state: None }
    }

    /// Only those standing in `state`.
    pub fn standing_in(mut self, state: StateId) -> Target {
        self.in_state = Some(state);
        self
    }
}
