//! The two lines that tell a Drone to settle a question itself. One wording,
//! read by a person's `best` or `quick` on a question, a command or a Judge
//! refusal, and by the night's own answers to a session.

use ipc::{AnswerMode, WaitingMode};

/// The line that tells the agent to settle an item itself.
pub(crate) fn mode_line(mode: AnswerMode) -> &'static str {
    match mode {
        AnswerMode::Best => "Decide this yourself. Think it through and go with the best option.",
        AnswerMode::Quick => "Just get it done. Take the reasonable path, finish the work and land it. Skip the formalities.",
    }
}

/// A session's [`WaitingMode`] as the wire's [`AnswerMode`]: the same two words.
pub(crate) fn of_waiting(mode: WaitingMode) -> AnswerMode {
    match mode {
        WaitingMode::Best => AnswerMode::Best,
        WaitingMode::Quick => AnswerMode::Quick,
    }
}
