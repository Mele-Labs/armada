//! The question a Job's Drone asked, held on the record until the next Drone
//! opens with the answer. The owner's decision of 11 Oct 2026, #2094.
//!
//! **On the record and not on the working slot**, because the Drone that asked
//! stands down at once and gives its place back. The slot it asked from is
//! gone by the time anybody reads the question, and a Fleet restart in the
//! hours before the answer must not lose it.
//!
//! **No event**, for [`Job::on_branch`]'s reason: the columns are their own
//! authority. A child of `record` for its private fields, as `pausing` is.

use alloc::string::String;
use alloc::vec::Vec;

use crate::envelope::{Timestamp, Ulid};
use crate::workflow::StepId;

use super::Job;

/// One answer a Drone offered: the label a person picks and what the Drone
/// said picking it commits to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfferedAnswer {
    pub label: String,
    pub consequence: String,
}

/// What a person answered, and when. A label, a line leaving the choice to the Drone, or both.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GivenAnswer {
    /// The offered answer picked. Absent where they left it to the Drone.
    pub label: Option<String>,
    /// The instruction that tells the Drone to settle it itself, for a best or quick.
    pub deciding: Option<String>,
    pub at: Timestamp,
}

/// One of several things a Drone asked in a single ask, with the pick once
/// there is one. The Plan interview's unit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decision {
    /// Fleet's, `d1` onward in the order asked. Scoped to the question it is on.
    pub id: String,
    pub question: String,
    pub options: Vec<OfferedAnswer>,
    /// The label picked. `None` until the question is answered.
    pub chose: Option<String>,
}

impl Decision {
    pub fn offering(&self, label: &str) -> Option<&OfferedAnswer> {
        self.options.iter().find(|option| option.label == label)
    }

    pub fn labels(&self) -> Vec<&str> {
        self.options.iter().map(|o| o.label.as_str()).collect()
    }

    /// The option picked, once it is.
    pub fn chosen(&self) -> Option<&OfferedAnswer> {
        self.chose.as_deref().and_then(|label| self.offering(label))
    }
}

/// A question a Drone asked on one step, and the answer once there is one.
///
/// **`answer` is `None` while the Job waits on a person**, and `Some` from the
/// answer until the next Drone opens with it, when the whole value is cleared.
///
/// **Two shapes, one filled.** A single question fills `question` and `options`
/// and leaves `decisions` empty. An interview fills `decisions` and leaves
/// `question` blank and `options` empty; its answer is each decision's `chose`,
/// and `answer` then carries the instant with an empty label.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DroneQuestion {
    /// Fleet's, never the Drone's: an answer naming an id a peer invented would
    /// join to nothing.
    pub id: Ulid,
    pub step: StepId,
    pub asked_at: Timestamp,
    pub question: String,
    pub options: Vec<OfferedAnswer>,
    pub answer: Option<GivenAnswer>,
    pub decisions: Vec<Decision>,
}

impl DroneQuestion {
    /// The option with this label. Labels are unique, since `ipc::mcp`
    /// refuses a question offering two the same, so this is one match or none.
    pub fn offering(&self, label: &str) -> Option<&OfferedAnswer> {
        self.options.iter().find(|option| option.label == label)
    }

    /// Every label offered, for a refusal that names them.
    pub fn labels(&self) -> Vec<&str> {
        self.options.iter().map(|o| o.label.as_str()).collect()
    }

    /// The decision with this id, on an interview.
    pub fn decision(&self, id: &str) -> Option<&Decision> {
        self.decisions.iter().find(|decision| decision.id == id)
    }

    /// What was asked, for a sentence naming it: the question, or an
    /// interview's first.
    pub fn headline(&self) -> &str {
        match self.decisions.first() {
            Some(first) if self.question.is_empty() => &first.question,
            _ => &self.question,
        }
    }

    /// The option the person picked, once they have.
    pub fn chosen(&self) -> Option<&OfferedAnswer> {
        self.answer
            .as_ref()
            .and_then(|given| given.label.as_deref())
            .and_then(|label| self.offering(label))
    }
}

impl Job {
    /// The Job holding a question its Drone just asked. Also the read-back.
    pub fn asking(&self, question: DroneQuestion) -> Job {
        let mut job = self.clone();
        job.question = Some(alloc::boxed::Box::new(question));
        job
    }

    /// The question answered. **Nothing changes where none is held**: the
    /// caller has already refused that case with the words a person reads.
    pub fn question_answered(&self, given: GivenAnswer) -> Job {
        let mut job = self.clone();
        if let Some(question) = job.question.as_mut() {
            question.answer = Some(given);
        }
        job
    }

    /// An interview answered: each decision's pick, and when. **Nothing changes
    /// where none is held**, as [`question_answered`](Job::question_answered).
    pub fn question_decided(&self, picks: &[(String, String)], at: Timestamp) -> Job {
        let mut job = self.clone();
        if let Some(question) = job.question.as_mut() {
            for decision in question.decisions.iter_mut() {
                decision.chose = picks
                    .iter()
                    .find(|(id, _)| *id == decision.id)
                    .map(|(_, label)| label.clone());
            }
            question.answer = Some(GivenAnswer {
                label: None,
                deciding: None,
                at,
            });
        }
        job
    }

    /// The answer went into a Drone's opening brief, so nothing is held. For
    /// `redirect_delivered`'s reason: an answer that outlived one Drone would
    /// reach one working on something it was never about.
    pub fn question_delivered(&self) -> Job {
        let mut job = self.clone();
        job.question = None;
        job
    }

    pub fn question(&self) -> Option<&DroneQuestion> {
        self.question.as_deref()
    }

    /// Whether the Job is waiting on a person's answer. **Admission's predicate
    /// and the Board's**, so the two cannot disagree about it.
    pub fn waiting_on_an_answer(&self) -> bool {
        self.question
            .as_ref()
            .is_some_and(|question| question.answer.is_none())
    }
}
