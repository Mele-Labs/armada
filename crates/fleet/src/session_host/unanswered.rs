//! A question asked in a turn the person began, with nothing written to him since his message.
//! `docs/concepts/session/waiting-on-the-person.md`.
//!
//! **He reads only what the agent writes**, never its reasoning. An agent that works its answer out
//! in its reasoning and goes straight to `AskUserQuestion` leaves him the question alone: Job
//! 7f7f4ae4 did it twice on 24 Oct 2026. So the first question after his message, with not one
//! word written to him since, is sent back to have the answer written. **Once a message**: a second
//! send-back would only loop. Fleet reads nothing into what was written: any text at all answers.

use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use super::State;
use crate::daemon::Fleet;

/// What the agent reads when it asks a question before writing him anything since his message.
pub(crate) const UNANSWERED_QUESTION: &str = "Not asked: since his last message you have written him nothing, and he reads only what you write, never your reasoning. Write your answer to his message, then ask this question again.";

/// How long the door waits for the stream to reach the question. The text before a call comes ahead
/// of it in the stream, and the stream is read apart from the door, so the door can be first.
const CAUGHT_UP_WITHIN: Duration = Duration::from_secs(2);

/// The person wrote, or answered a question: the agent owes him a reply.
pub(crate) fn owed(state: &mut State) {
    state.owes_reply = true;
}

impl Fleet
{
    /// Whether the question `call` is to be sent back for the answer, read once the stream has
    /// reached it. Taking the debt, so the same question asked again is put to him.
    pub(crate) async fn question_unanswered(&self, id: &str, call: Option<&str>) -> bool {
        let runtime = self.hosts().of(id);
        if !runtime.state().owes_reply {
            return false;
        }
        if let Some(call) = call {
            let started = std::time::Instant::now();
            while runtime.state().question_heard.as_deref() != Some(call) && started.elapsed() < CAUGHT_UP_WITHIN {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }
        let mut state = runtime.state();
        std::mem::take(&mut state.owes_reply)
    }
}
