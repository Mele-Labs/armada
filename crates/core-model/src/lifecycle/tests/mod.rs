//! Tests of the lifecycle vocabulary. None needs a tracker, a forge or a Job to run.

mod declaring;
mod holding;
mod reading;

use alloc::string::String;
use alloc::vec::Vec;

use crate::lifecycle::{
    Lifecycle, LifecycleId, LifecycleScope, LifecycleState, LifecycleTransition, ScopeId, StateId, StateRead,
    TrackerWords,
};
use crate::workflow::Title;

fn id(written: &str) -> StateId {
    StateId::new(written).expect("a state id")
}

fn words(listed: &[&str]) -> StateRead {
    StateRead::Tracker(TrackerWords::of(listed).expect("tracker words"))
}

fn state(written: &str, read: StateRead) -> LifecycleState {
    LifecycleState::new(id(written), Title::new(written).expect("a label"), read)
}

fn moves(pairs: &[(&str, &str)]) -> Vec<LifecycleTransition> {
    pairs.iter().map(|(from, to)| LifecycleTransition { from: id(from), to: id(to) }).collect()
}

fn ends(named: &[&str]) -> Vec<StateId> {
    named.iter().map(|one| id(one)).collect()
}

fn scope(written: &str) -> LifecycleScope {
    LifecycleScope::Within(ScopeId::new(written).expect("a scope"))
}

/// A ticket lifecycle as a Workspace (a set of repositories) would scope it, with the tracker's words filled in.
fn my_stores_tickets() -> Lifecycle {
    Lifecycle::declared(
        LifecycleId::new("ticket").expect("an id"),
        Title::new("Ticket").expect("a name"),
        scope("my-store"),
        alloc::vec![
            state("in_progress", words(&["In Progress"])),
            state("in_review", words(&["Code Review", "In Review"])),
            state("closed", words(&["Done"])),
        ],
        moves(&[("in_progress", "in_review"), ("in_review", "closed")]),
        ends(&["closed"]),
    )
    .expect("a well formed lifecycle")
}

fn text(s: &str) -> String {
    String::from(s)
}
