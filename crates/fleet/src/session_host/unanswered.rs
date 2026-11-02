//! A turn the person began that reaches a question, or its end, with nothing written to him.
//! `docs/concepts/session/waiting-on-the-person.md`.
//!
//! **He reads only what the agent writes**, never its reasoning. An agent that works its answer out
//! in its reasoning and goes straight to `AskUserQuestion` leaves him the question alone: Job
//! 7f7f4ae4 did it twice on 24 Oct 2026, the second time after a lone "Status:" line. So the first
//! question after his message, and else the turn's end, is sent back to have the answer written.
//! **Once a message**: a second send-back would only loop.
//!
//! **A status line is a lone line the agent carries on past**, whatever its first word. Job 77d7b355
//! wrote "Found the main causes. I'm writing them up now." on 2 Nov 2026, ran a tool, and asked; and
//! asked again after his answer to that question with nothing written at all. So a lone line is held
//! until what follows it says what it was: a tool call makes it a status line, the question or the
//! turn's end makes it the answer. And his answer to a question is his message, as a typed one is.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use super::{Runtime, State};
use crate::daemon::Fleet;

/// What the agent reads when it asks a question before writing him anything since his message.
pub(crate) const UNANSWERED_QUESTION: &str = "Not asked: since his last message you have written him nothing but a status line, and he reads only what you write, never your reasoning. Write your answer to his message, then ask this question again.";

/// What the agent reads when its turn ends with nothing written to him since his message.
pub(crate) const UNANSWERED_TURN: &str = "Since his last message you have written him nothing but a status line, and he reads only what you write, never your reasoning. Write your answer to his message.";

/// How long the door waits for the stream to reach the question. The text before a call comes ahead
/// of it in the stream, and the stream is read apart from the door, so the door can be first.
const CAUGHT_UP_WITHIN: Duration = Duration::from_secs(2);

/// The person wrote, or answered a question: the agent owes him a reply.
pub(crate) fn owed(state: &mut State) {
    state.owes_reply = true;
    state.reply_line = false;
}

/// The agent wrote `text`. More than a line answers him; a lone line starting "Status:" never does;
/// any other lone line is held until what follows it says, and a second one with it is an answer.
pub(super) fn heard(state: &mut State, text: &str) {
    let text = text.trim();
    if text.contains('\n') || state.reply_line {
        state.owes_reply = false;
        state.reply_line = false;
    } else if !text.starts_with("Status:") {
        state.reply_line = true;
    }
}

/// The agent called a tool other than the question after a lone line: it was a status line.
pub(super) fn carried_past(state: &mut State) {
    state.reply_line = false;
}

/// The lone line was the last thing before the question or the turn's end: it was his answer.
fn line_settled(state: &mut State) {
    if std::mem::take(&mut state.reply_line) {
        state.owes_reply = false;
    }
}

/// Whether the turn that ended leaves him unanswered. Not while another turn waits to start, which
/// may be his message's own.
pub(super) fn turn_unanswered(runtime: &Runtime) -> bool {
    let mut state = runtime.state();
    line_settled(&mut state);
    !state.helm && state.queued == 0 && std::mem::take(&mut state.owes_reply)
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
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
        line_settled(&mut state);
        !state.helm && std::mem::take(&mut state.owes_reply)
    }

    /// Send the agent back to write the answer its turn left out.
    pub(super) async fn unanswered_reminded(self: &Arc<Self>, id: &str) {
        let _ = self.noticed_by_fleet(id, UNANSWERED_TURN, ipc::Notice::ReplyClaim).await;
    }
}
