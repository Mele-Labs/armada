//! The four lifecycles Armada carries: a ticket, a pull request, a deploy and a Job.
//!
//! **Built in Rust and not read from a file**, because Fleet names the Job lifecycle's states
//! itself and a state it spells by a string it never checks is the convention failure this crate
//! is for. A piece file that spells the same lifecycle out reads back equal to it, which a test in
//! `config` holds.
//!
//! The ticket lists one tracker word of its own, `closed`, which no tracker spells another way.
//! **A tracker brings the rest it uses** ([`Lifecycle::ticket_reading`]), so a repository on a
//! tracker that declares some reads from the first Job; one on a tracker that declares none leaves
//! every other status unmapped, which is a fact, where a default would have been a guess. A
//! lifecycle scoped to a Workspace (a set of repositories) or to a repository replaces the Ticket
//! outright, and is where a person says which words are which.

use alloc::vec;
use alloc::vec::Vec;

use crate::watch::ReadingState;
use crate::workflow::Title;

use super::ids::{LifecycleId, StateId};
use super::machine::{Lifecycle, LifecycleScope, LifecycleState, LifecycleTransition};
use super::read::{FleetFact, ForgeFact, StateRead, TrackerWords};

/// The words the ticket lists whatever the tracker, as `(state id, word)`.
const CARRIED: &[(&str, &str)] = &[("closed", "closed")];

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
    /// A ticket in the tracker: `ticket`, **listing only `closed`**.
    pub fn ticket() -> Lifecycle {
        Lifecycle::ticket_reading(&[])
    }

    /// `ticket` with the words a tracker declares added to the one it carries, as `(state id,
    /// word)`. **A blank word, a word repeated on one state, a word two states both claim and an id
    /// the ticket has no state for are left out**: what a tracker declares is not a person's file,
    /// and one bad pair must not take the rest with it. A status two states claim is read as
    /// neither, the same as a lifecycle that listed it twice is refused, **except that the word the
    /// ticket carries is never the one left out**.
    pub fn ticket_reading(declared: &[(&str, &str)]) -> Lifecycle {
        let words = |state: &str| {
            let mut kept: Vec<&str> = CARRIED.iter().filter(|(on, _)| *on == state).map(|(_, word)| *word).collect();
            for (_, word) in declared.iter().filter(|(on, _)| *on == state) {
                let word = word.trim();
                let claimed_elsewhere = declared.iter().chain(CARRIED.iter()).any(|(on, other)| *on != state && other.trim() == word);
                if !word.is_empty() && !claimed_elsewhere && !kept.contains(&word) {
                    kept.push(word);
                }
            }
            StateRead::Tracker(TrackerWords::of(kept).expect("blanks and repeats were left out"))
        };
        built(
            "ticket",
            "Ticket",
            vec![
                state("todo", "To do", words("todo")),
                state("in_progress", "In progress", words("in_progress")),
                state("in_review", "In review", words("in_review")),
                state("closed", "Closed", words("closed")),
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
        Lifecycle::built_in_reading(&[])
    }

    /// The four, with the ticket reading the words a tracker declares (`ticket_reading`).
    pub fn built_in_reading(ticket_words: &[(&str, &str)]) -> Vec<Lifecycle> {
        vec![Lifecycle::ticket_reading(ticket_words), Lifecycle::pull_request(), Lifecycle::deploy(), Lifecycle::job()]
    }
}
