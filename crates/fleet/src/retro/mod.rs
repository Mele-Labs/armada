//! A Job's retro: once a Job ends, what got in the way while it ran, and
//! whose way. `docs/concepts/retro.md` is the concept.
//!
//! [`record`] assembles the record and does no I/O; [`gathering`] reads what
//! it is assembled from; [`writing`] asks a model once per ended Job, off the
//! turn; [`serving`] answers `get_job_retro` and `list_lessons`; [`agreeing`]
//! answers a person's agree or disagree on an item. **Nothing here moves a Job
//! or reaches a Drone**, and the one thing a retro does, a person's agreeing,
//! is to propose a Job at the approval gate.
//!
//! It also signs a person's move for the door it came through, which every
//! move asks of [`signed`]: a retro of a Job an agent drove is wrong about
//! whose friction it was unless the record says an agent drove it.

mod agreeing;
mod asking;
mod changing;
pub(crate) mod gathering;
pub(crate) mod record;
mod reviewing;
mod serving;
mod session_agreeing;
mod session_record;
mod session_writing;
mod writing;

pub(crate) use asking::ASK;
pub(crate) use reviewing::REVIEW;
pub(crate) use session_writing::SESSION_QUESTION;
pub(crate) use writing::{LANDS_IN, QUESTION, TEXTS};

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use core_model::{Actor, JobId, Via};

#[cfg(test)]
pub(crate) use writing::read;
#[cfg(test)]
pub(crate) mod review_for_tests {
    pub(crate) use super::reviewing::read;
}
pub(crate) use writing::reflected;

/// The lines Fleet writes into a Job's log when a person is asked something
/// and when they answer. **Named once**, because [`record`] pairs them.
pub(crate) mod lines {
    pub(crate) const A_DRONE_ASKS: &str = "the drone asked a question and is waiting for an answer";
    pub(crate) const A_PERSON_ANSWERS_A_DRONE: &str = "a person answered the drone's question";
    pub(crate) const A_JUDGE_ASKS: &str =
        "a judge criterion refused, and a person is being asked rather than the step stopping";
    pub(crate) const A_PERSON_AGREES: &str = "a person agreed with a judge's refusal";
    pub(crate) const A_PERSON_DISAGREES_ONCE: &str =
        "a person disagreed with a judge's refusal, for this step";
    pub(crate) const A_PERSON_DISAGREES_ALWAYS: &str =
        "a person disagreed with a judge's refusal, and this repository stops being asked \
         about that criterion";
    pub(crate) const A_COMMAND_ASKS: &str =
        "the drone reached for a command it was not granted, and a person is being asked";
    pub(crate) const A_PERSON_ANSWERS_A_COMMAND: &str =
        "a person answered a command the drone was waiting on";
    pub(crate) const A_DRONE_RAN_CHECKS: &str =
        "the Drone asked for the step's checks and they were run";
    /// The same ask, when it began. **Not counted by a retro**, which reads the
    /// line above alone.
    pub(crate) const A_DRONE_STARTED_CHECKS: &str =
        "the Drone asked for the step's checks and they were started";
    /// A gate's red run again alone before it was ruled on — `crate::confirming`.
    pub(crate) const A_RED_RUN_ALONE: &str =
        "a check failed with other checks running beside it, and was run again alone";
    /// The start of the line a restart with a note writes; the note follows.
    pub(crate) const A_PERSON_RESTARTED: &str = "a person restarted the step";
}

/// Who a move is signed by, and the door it came through.
///
/// **A person's act that came through any door but Bridge is Helm's**: an
/// agent made the request, and Fleet cannot see the person behind it. A move
/// with no request behind it keeps the signer it was given and no door.
pub(crate) fn signed(by: Actor) -> (Actor, Option<Via>) {
    let via = api::via().map(|via| via.domain());
    match (by, via) {
        (Actor::Human, Some(via)) if !via.is_a_person_in_bridge() => (Actor::Helm, Some(via)),
        (Actor::Human | Actor::Helm, via) => (by, via),
        (by, _) => (by, None),
    }
}

/// Keep the door a move came through, under the store lock its write took.
/// **One that will not keep fails nothing**: the move has landed.
pub(crate) fn kept_via(store: &mut store::Store, job: &JobId, seq: i64, via: Option<Via>) {
    if let Some(via) = via {
        let _ = store.record_via(job, seq, via);
    }
}

/// Whether a retro is being written. **One at a time**: each is a model call,
/// and a Fleet that comes back to a dozen ended Jobs writes them in turn.
#[derive(Clone, Default)]
pub(crate) struct Reflecting(pub(crate) Arc<AtomicBool>);
