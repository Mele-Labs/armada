//! Margin threads: his comments on a passage of a Job's page, and the agent's answers and suggested edits.
//! `docs/concepts/document.md`, *Margin threads*.
//!
//! **Its own surface**, for [`Watches`](super::Watches)' reason: whole on its own, and half of it is the agent's.

use std::future::Future;

use ipc::{ActOnMargin, AnswerThread, CommentMargin, GetMarginThreads, MarginThread, MarginThreads, SendHeldMargin, SuggestEdit};

use crate::daemon::Refusal;

pub trait Margins: Send + Sync + 'static {
    /// `comment_margin` — his comment on a passage, a new thread, or his reply on one, sent now or held. Answers the
    /// page's threads.
    fn comment_margin(self: std::sync::Arc<Self>, comment: CommentMargin) -> impl Future<Output = Result<MarginThreads, Refusal>> + Send;

    /// `send_held_margin` — every comment he holds on the page, sent as one message.
    fn send_held_margin(self: std::sync::Arc<Self>, send: SendHeldMargin) -> impl Future<Output = Result<MarginThreads, Refusal>> + Send;

    /// `act_on_margin_thread` — his Make this change, Accept, Reject or Settle on one thread.
    fn act_on_margin_thread(self: std::sync::Arc<Self>, act: ActOnMargin) -> impl Future<Output = Result<MarginThreads, Refusal>> + Send;

    /// `get_margin_threads` — a page's threads, oldest first.
    fn get_margin_threads(&self, get: GetMarginThreads) -> impl Future<Output = Result<MarginThreads, Refusal>> + Send;

    /// `answer_thread` — the agent's answer in a thread. `caller` places a hosted Job.
    fn answer_thread(
        self: std::sync::Arc<Self>,
        caller: Option<crate::Caller>,
        answer: AnswerThread,
    ) -> impl Future<Output = Result<MarginThread, Refusal>> + Send;

    /// `suggest_edit` — the agent's edit on a thread's passage, waiting on his Accept. `caller` places a hosted Job.
    fn suggest_edit(
        self: std::sync::Arc<Self>,
        caller: Option<crate::Caller>,
        suggest: SuggestEdit,
    ) -> impl Future<Output = Result<MarginThread, Refusal>> + Send;
}
