//! A Judge on demand, and the person's three acts on what it read. Its own surface, for
//! [`SharedHome`](super::SharedHome)'s reason: a handler taking only this cannot reach for a Job's record.
//! `docs/concepts/judge.md`; `docs/practices/protocol/a-job-is-judged-on-demand.md`.

use std::future::Future;

use ipc::{AskJudge, DismissJudgeNote, JudgeReading, MarkJudgeRead, SendJudgeNote};

use crate::daemon::Refusal;

pub trait JudgeReadings: Send + Sync + 'static {
    /// `ask_judge` — one read-only model call that reads one artifact of a Job. **Answers at once**, with the
    /// reading `reading`; the call runs in a task. A reading already going for the same artifact is answered,
    /// not started twice. [`Refusal::NotFound`] (`fleet.no_such_session`) for no such Job,
    /// [`Refusal::IllegalMove`] (`fleet.session_closed`) for one that ended.
    fn ask_judge(
        self: std::sync::Arc<Self>,
        ask: AskJudge,
    ) -> impl Future<Output = Result<JudgeReading, Refusal>> + Send;

    /// `send_judge_note` — a note goes to the agent as the person's message, then is settled. A note already
    /// settled is answered as it stands. [`Refusal::NotFound`] (`fleet.no_such_session`,
    /// `fleet.no_such_judge_reading` for a reading the Job does not hold, `fleet.no_such_judge_note`).
    fn send_judge_note(
        self: std::sync::Arc<Self>,
        send: SendJudgeNote,
    ) -> impl Future<Output = Result<JudgeReading, Refusal>> + Send;

    /// `dismiss_judge_note` — a note is settled and the agent is told nothing. The same refusals.
    fn dismiss_judge_note(
        self: std::sync::Arc<Self>,
        dismiss: DismissJudgeNote,
    ) -> impl Future<Output = Result<JudgeReading, Refusal>> + Send;

    /// `mark_judge_read` — the person opened the reading's notes. The agent is told nothing.
    /// [`Refusal::NotFound`] for no such Job or reading.
    fn mark_judge_read(
        self: std::sync::Arc<Self>,
        read: MarkJudgeRead,
    ) -> impl Future<Output = Result<JudgeReading, Refusal>> + Send;
}
