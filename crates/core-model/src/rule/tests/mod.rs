//! Tests of the rule vocabulary and the engine. None needs a tracker, a forge or a Job to run: an
//! occurrence is a value, and what the Job holds is built here.

mod engine;
mod model;

use crate::lifecycle::{
    HeldInstances, InstanceRef, Lifecycle, LifecycleId, LifecycleInstance, LifecycleScope, LifecycleState, LifecycleTransition,
    Moved, Observed, ScopeId, Standing, StateId, StateRead, TrackerWords,
};
use crate::rule::{Act, Among, Condition, OnFailure, Quantity, Rule, RuleId, RuleParts, Target, TrackerWrite, When, Words};
use crate::workflow::{StepId, Title};

pub(super) fn lifecycle_id(written: &str) -> LifecycleId {
    LifecycleId::new(written).expect("a lifecycle id")
}

pub(super) fn state(written: &str) -> StateId {
    StateId::new(written).expect("a state id")
}

pub(super) fn words(written: &str) -> Words {
    Words::new(written).expect("words")
}

pub(super) fn step(written: &str) -> StepId {
    StepId::new(written)
}

/// my-store's tickets, in the words its tracker uses.
pub(super) fn tickets() -> Lifecycle {
    let state_of = |id: &str, listed: &[&str]| {
        LifecycleState::new(state(id), Title::new(id).expect("a label"), StateRead::Tracker(TrackerWords::of(listed).expect("words")))
    };
    let move_of = |from: &str, to: &str| LifecycleTransition { from: state(from), to: state(to) };
    Lifecycle::declared(
        lifecycle_id("ticket"),
        Title::new("Ticket").expect("a name"),
        LifecycleScope::Within(ScopeId::new("my-store").expect("a scope")),
        vec![
            state_of("todo", &["To Do"]),
            state_of("in_progress", &["In Progress"]),
            state_of("in_review", &["In Review"]),
            state_of("closed", &["Done"]),
        ],
        vec![move_of("todo", "in_progress"), move_of("in_progress", "in_review"), move_of("in_review", "closed")],
        vec![state("closed")],
    )
    .expect("a well formed lifecycle")
}

pub(super) fn at(lifecycle: &Lifecycle, key: &str) -> InstanceRef {
    InstanceRef::of(lifecycle, key).expect("a key")
}

pub(super) fn into(state_id: &str) -> Standing {
    Standing::In(state(state_id))
}

/// What a move into `state_id` from nowhere reads like to the engine.
pub(super) fn moved_into(state_id: &str) -> Moved {
    Moved { from: Standing::Unread, to: into(state_id) }
}

/// Two tickets the Job holds, read in the tracker's words, and a pull request for both.
pub(super) fn a_job_with_two_tickets(first: &str, second: &str) -> HeldInstances {
    let (ticket, pull_request) = (tickets(), Lifecycle::pull_request());
    let mut held = HeldInstances::new();
    for (key, word) in [("ARM-41", first), ("ARM-42", second)] {
        held.take_on(LifecycleInstance::taken_on(at(&ticket, key))).expect("held");
        held.get_mut(&at(&ticket, key)).expect("held").observe(&ticket, &Observed::Tracker(word)).expect("its own");
    }
    let pr = LifecycleInstance::taken_on(at(&pull_request, "812")).linked_to(at(&ticket, "ARM-41")).linked_to(at(&ticket, "ARM-42"));
    held.take_on(pr).expect("held");
    held
}

pub(super) fn parts(id: &str, when: When, does: Vec<Act>) -> RuleParts {
    RuleParts {
        id: RuleId::new(id).expect("a rule id"),
        name: Title::new(id).expect("a name"),
        scope: LifecycleScope::Everywhere,
        when,
        conditions: Vec::new(),
        does,
        autonomy: None,
        on_failure: OnFailure::default(),
    }
}

pub(super) fn rule(parts: RuleParts) -> Rule {
    Rule::declared(parts).expect("a well formed rule")
}

/// Move every ticket the Job holds that is still in To do into In progress.
pub(super) fn move_todo_tickets() -> Act {
    Act::Tracker {
        write: TrackerWrite::Move { to: state("in_progress") },
        target: Target::held(lifecycle_id("ticket")).standing_in(state("todo")),
    }
}

/// A plan step starts: some ticket is still in To do.
pub(super) fn when_planning_starts(mut parts: RuleParts) -> RuleParts {
    parts.when = When::StepStarts(step("plan"));
    parts.conditions = vec![Condition::Instances {
        among: Among::Held(lifecycle_id("ticket")),
        quantity: Quantity::Any,
        in_state: state("todo"),
    }];
    parts
}
