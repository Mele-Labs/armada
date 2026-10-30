//! The four lifecycles Armada carries: a ticket, a pull request, a deploy and a Job.
//!
//! **Built in Rust and not read from a file**, because Fleet names the Job lifecycle's states
//! itself and a state it spells by a string it never checks is the convention failure this crate
//! is for. A piece file that spells the same lifecycle out reads back equal to it, which a test in
//! `config` holds.
//!
//! The ticket lists one tracker word, `closed`, which no tracker spells another way. Two trackers
//! rarely share the rest, and a lifecycle scoped to a Workspace (a set of repositories) or to a
//! repository is where a person says which are which; until then every other status is unmapped,
//! which is a fact, where a default would have been a guess.

use alloc::vec;
use alloc::vec::Vec;

use crate::watch::ReadingState;
use crate::workflow::Title;

use super::ids::{LifecycleId, StateId};
use super::machine::{Lifecycle, LifecycleScope, LifecycleState, LifecycleTransition};
use super::read::{FleetFact, ForgeFact, StateRead, TrackerWords};

fn id(written: &str) -> StateId {
    StateId::new(written).expect("a built-in state id is one word")
}

fn state(written: &str, label: &str, read: StateRead) -> LifecycleState {
    LifecycleState::new(id(written), Title::new(label).expect("a built-in label is not blank"), read)
}

fn moves(pairs: &[(&str, &str)]) -> Vec<LifecycleTransition> {
    pairs.iter().map(|(from, to)| LifecycleTransition { from: id(from), to: id(to) }).collect()
}

fn built(
    written: &str,
    name: &str,
    states: Vec<LifecycleState>,
    transitions: Vec<LifecycleTransition>,
    ends: &[&str],
) -> Lifecycle {
    Lifecycle::declared(
        LifecycleId::new(written).expect("a built-in lifecycle id is one word"),
        Title::new(name).expect("a built-in name is not blank"),
        LifecycleScope::Everywhere,
        states,
        transitions,
        ends.iter().map(|end| id(end)).collect(),
    )
    .expect("a built-in lifecycle is well formed")
}

impl Lifecycle {
    /// A ticket in the tracker: `ticket`.
    pub fn ticket() -> Lifecycle {
        let words = || StateRead::Tracker(TrackerWords::default());
        let closed = StateRead::Tracker(TrackerWords::of(["closed"]).expect("`closed` is one word"));
        built(
            "ticket",
            "Ticket",
            vec![
                state("todo", "To do", words()),
                state("in_progress", "In progress", words()),
                state("in_review", "In review", words()),
                state("closed", "Closed", closed),
            ],
            moves(&[("todo", "in_progress"), ("in_progress", "in_review"), ("in_review", "closed")]),
            &["closed"],
        )
    }

    /// A pull request on the forge: `pr`. **Closed is read and not on the way**: a pull request
    /// can be closed from any state, so no move into it is listed.
    pub fn pull_request() -> Lifecycle {
        built(
            "pr",
            "Pull request",
            vec![
                state("draft", "Draft", StateRead::Forge(ForgeFact::Draft)),
                state("ready", "Ready", StateRead::Forge(ForgeFact::Ready)),
                state("reviewers_on", "Reviewers on", StateRead::Forge(ForgeFact::ReviewersOn)),
                state("in_review", "In review", StateRead::Forge(ForgeFact::InReview)),
                state("approved", "Approved", StateRead::Forge(ForgeFact::Approved)),
                state("merged", "Merged", StateRead::Forge(ForgeFact::Merged)),
                state("closed", "Closed", StateRead::Forge(ForgeFact::Closed)),
            ],
            moves(&[
                ("draft", "ready"),
                ("ready", "reviewers_on"),
                ("reviewers_on", "in_review"),
                ("in_review", "approved"),
                ("approved", "merged"),
            ]),
            &["merged", "closed"],
        )
    }

    /// A deploy a merge sets off, read from a watch: `deploy`. **A watch that has not started
    /// reads `waiting`, which no state claims**, so a deploy is unmapped until its watch says it
    /// is running.
    pub fn deploy() -> Lifecycle {
        built(
            "deploy",
            "Deploy",
            vec![
                state("rolling_out", "Rolling out", StateRead::Watch(ReadingState::Running)),
                state("healthy", "Healthy", StateRead::Watch(ReadingState::Passed)),
                state("failed", "Failed", StateRead::Watch(ReadingState::Failed)),
            ],
            moves(&[("rolling_out", "healthy"), ("rolling_out", "failed")]),
            &["healthy", "failed"],
        )
    }

    /// A Job a Job starts, read from Fleet's own events: `job`.
    pub fn job() -> Lifecycle {
        built(
            "job",
            "Job",
            vec![
                state("running", "Running", StateRead::Fleet(FleetFact::Started)),
                state("landed", "Landed", StateRead::Fleet(FleetFact::Landed)),
                state("ended", "Ended", StateRead::Fleet(FleetFact::Ended)),
            ],
            moves(&[("running", "landed"), ("running", "ended")]),
            &["landed", "ended"],
        )
    }

    /// The four, in the order a Job meets them.
    pub fn built_in() -> Vec<Lifecycle> {
        vec![Lifecycle::ticket(), Lifecycle::pull_request(), Lifecycle::deploy(), Lifecycle::job()]
    }
}
