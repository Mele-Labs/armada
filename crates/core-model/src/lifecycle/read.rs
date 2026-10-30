//! How a state is read: which of four sources says an instance is in it, and what it has to say.
//!
//! **Fleet decides a state from what it observes, and what the agent says counts for nothing.**
//! A state therefore names a fact a reader can see, never a claim a Job can make. The readers
//! are other work: what is here is the vocabulary they report in, so a reader and a lifecycle
//! cannot disagree about what a word means.
//!
//! The sets below are closed. A fact no variant spells is a new variant, and every `match` on
//! it then fails to compile until it is handled.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use crate::watch::ReadingState;
use crate::workflow::StepId;

use super::machine::MalformedLifecycle;

/// Whose word an instance's state is read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StateSource {
    /// The tracker's own status, assignee and resolution.
    Tracker,
    /// The forge: draft or ready, reviewers, reviews, comments, merged.
    Forge,
    /// The state a watch's command prints.
    Watch,
    /// Fleet's own events.
    Fleet,
}

impl StateSource {
    pub const ALL: &'static [StateSource] =
        &[StateSource::Tracker, StateSource::Forge, StateSource::Watch, StateSource::Fleet];

    pub fn as_wire(self) -> &'static str {
        match self {
            StateSource::Tracker => "tracker",
            StateSource::Forge => "forge",
            StateSource::Watch => "watch",
            StateSource::Fleet => "fleet",
        }
    }

    pub fn from_wire(value: &str) -> Option<StateSource> {
        StateSource::ALL.iter().copied().find(|source| source.as_wire() == value)
    }
}

/// What the forge shows of a pull request.
///
/// **Which one a pull request is showing when several hold is the reader's to say**, as an
/// approved pull request still has its reviewers on it. A lifecycle only names the fact its
/// state stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ForgeFact {
    /// It is a draft.
    Draft,
    /// It is marked ready, and nobody has been asked to review it.
    Ready,
    /// Somebody has been asked to review it.
    ReviewersOn,
    /// A reviewer has commented or reviewed, and none has approved.
    InReview,
    /// The reviews approve it.
    Approved,
    /// It is merged.
    Merged,
    /// It is closed, and was not merged.
    Closed,
}

impl ForgeFact {
    pub const ALL: &'static [ForgeFact] = &[
        ForgeFact::Draft,
        ForgeFact::Ready,
        ForgeFact::ReviewersOn,
        ForgeFact::InReview,
        ForgeFact::Approved,
        ForgeFact::Merged,
        ForgeFact::Closed,
    ];

    pub fn as_wire(self) -> &'static str {
        match self {
            ForgeFact::Draft => "draft",
            ForgeFact::Ready => "ready",
            ForgeFact::ReviewersOn => "reviewers_on",
            ForgeFact::InReview => "in_review",
            ForgeFact::Approved => "approved",
            ForgeFact::Merged => "merged",
            ForgeFact::Closed => "closed",
        }
    }

    pub fn from_wire(value: &str) -> Option<ForgeFact> {
        ForgeFact::ALL.iter().copied().find(|fact| fact.as_wire() == value)
    }
}

/// What Fleet knows of a Job without asking anyone.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FleetFact {
    /// The Job was started, and has not ended.
    Started,
    /// A step of the Job entered, such as `plan`.
    StepEntered(StepId),
    /// The Job opened a pull request.
    PullRequestOpened,
    /// The Job's pull request landed on `main`.
    Landed,
    /// The Job ended, whether or not its pull request landed.
    Ended,
}

impl FleetFact {
    /// The spelling a piece file and a stored row use. A step is `step_entered:<step id>`.
    pub fn as_wire(&self) -> String {
        match self {
            FleetFact::Started => "started".to_string(),
            FleetFact::StepEntered(step) => alloc::format!("step_entered:{}", step.as_str()),
            FleetFact::PullRequestOpened => "pull_request_opened".to_string(),
            FleetFact::Landed => "landed".to_string(),
            FleetFact::Ended => "ended".to_string(),
        }
    }

    pub fn from_wire(value: &str) -> Option<FleetFact> {
        match value {
            "started" => Some(FleetFact::Started),
            "pull_request_opened" => Some(FleetFact::PullRequestOpened),
            "landed" => Some(FleetFact::Landed),
            "ended" => Some(FleetFact::Ended),
            other => match other.strip_prefix("step_entered:") {
                Some(step) if !step.is_empty() => Some(FleetFact::StepEntered(StepId::new(step))),
                _ => None,
            },
        }
    }
}

/// The tracker's own words for one state, such as `Code Review` and `In Review` for In review.
///
/// **Stored trimmed and matched exactly.** `In review` is not `In Review`: a lifecycle that
/// folded the case would be guessing which words a tracker means alike, and a word it guessed
/// wrong is a ticket moved on a hunch. The words a person has not listed are unmapped, and the
/// editor offers to map them.
///
/// None is allowed: the lifecycle Armada carries lists only `closed` because two trackers rarely share
/// the rest, so every other value is unmapped until a lifecycle scoped to a Workspace (a set of
/// repositories) or to a repository lists them.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TrackerWords(Vec<String>);

impl TrackerWords {
    /// The words, trimmed. Refuses a blank one, and one listed twice.
    pub fn of<I, S>(words: I) -> Result<TrackerWords, MalformedLifecycle>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut kept: Vec<String> = Vec::new();
        for word in words {
            let word = word.as_ref().trim();
            if word.is_empty() {
                return Err(MalformedLifecycle::BlankWord);
            }
            if kept.iter().any(|have| have == word) {
                return Err(MalformedLifecycle::WordTwice { word: String::from(word) });
            }
            kept.push(String::from(word));
        }
        Ok(TrackerWords(kept))
    }

    pub fn as_slice(&self) -> &[String] {
        &self.0
    }

    pub fn claims(&self, value: &str) -> bool {
        let value = value.trim();
        self.0.iter().any(|word| word == value)
    }
}

/// How one state is read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StateRead {
    Tracker(TrackerWords),
    Forge(ForgeFact),
    /// The state a watch's command printed. A watch prints one of four, and its reading is
    /// already this crate's type.
    Watch(ReadingState),
    Fleet(FleetFact),
}

impl StateRead {
    pub fn source(&self) -> StateSource {
        match self {
            StateRead::Tracker(_) => StateSource::Tracker,
            StateRead::Forge(_) => StateSource::Forge,
            StateRead::Watch(_) => StateSource::Watch,
            StateRead::Fleet(_) => StateSource::Fleet,
        }
    }

    /// Whether `seen` is what this state stands for.
    pub fn claims(&self, seen: &Observed<'_>) -> bool {
        match (self, seen) {
            (StateRead::Tracker(words), Observed::Tracker(value)) => words.claims(value),
            (StateRead::Forge(fact), Observed::Forge(seen)) => fact == seen,
            (StateRead::Watch(state), Observed::Watch(seen)) => state == seen,
            (StateRead::Fleet(fact), Observed::Fleet(seen)) => fact == seen,
            _ => false,
        }
    }

    /// Whether the two would both claim something. Two states of one lifecycle that do are a
    /// reading with two answers.
    pub(super) fn overlaps(&self, other: &StateRead) -> Option<Overlap> {
        match (self, other) {
            (StateRead::Tracker(mine), StateRead::Tracker(theirs)) => mine
                .0
                .iter()
                .find(|word| theirs.0.contains(word))
                .map(|word| Overlap::Word(word.clone())),
            (StateRead::Forge(mine), StateRead::Forge(theirs)) => (mine == theirs).then_some(Overlap::Fact),
            (StateRead::Watch(mine), StateRead::Watch(theirs)) => (mine == theirs).then_some(Overlap::Fact),
            (StateRead::Fleet(mine), StateRead::Fleet(theirs)) => (mine == theirs).then_some(Overlap::Fact),
            _ => None,
        }
    }
}

pub(super) enum Overlap {
    Word(String),
    Fact,
}

/// One thing a reader saw, in the words its source used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Observed<'a> {
    /// A tracker's status, as the tracker spells it.
    Tracker(&'a str),
    Forge(ForgeFact),
    Watch(ReadingState),
    Fleet(FleetFact),
}

impl<'a> Observed<'a> {
    /// What a source said, as [`spelled`](Observed::spelled) wrote it down. `None` where the source
    /// has no such word, so a stored reading this build cannot place is not made up into one.
    pub fn read(source: StateSource, spelled: &'a str) -> Option<Observed<'a>> {
        match source {
            StateSource::Tracker => Some(Observed::Tracker(spelled)),
            StateSource::Forge => ForgeFact::from_wire(spelled).map(Observed::Forge),
            StateSource::Watch => ReadingState::from_wire(spelled).map(Observed::Watch),
            StateSource::Fleet => FleetFact::from_wire(spelled).map(Observed::Fleet),
        }
    }
}

impl Observed<'_> {
    pub fn source(&self) -> StateSource {
        match self {
            Observed::Tracker(_) => StateSource::Tracker,
            Observed::Forge(_) => StateSource::Forge,
            Observed::Watch(_) => StateSource::Watch,
            Observed::Fleet(_) => StateSource::Fleet,
        }
    }

    /// What was seen, as a person would be shown it where no state claims it.
    pub fn spelled(&self) -> String {
        match self {
            Observed::Tracker(value) => String::from(value.trim()),
            Observed::Forge(fact) => String::from(fact.as_wire()),
            Observed::Watch(state) => String::from(state.as_wire()),
            Observed::Fleet(fact) => fact.as_wire(),
        }
    }
}

impl fmt::Display for StateSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            StateSource::Tracker => "the tracker",
            StateSource::Forge => "the forge",
            StateSource::Watch => "a watch",
            StateSource::Fleet => "Fleet",
        })
    }
}
