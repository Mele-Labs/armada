//! What the owner said in a Job, sorted by kind, the asks that came of it, and a Job's story. Its own surface, for
//! [`Asides`](super::Asides)' reason: a handler taking only this reads and sets words and asks, and reaches no Job's thread.

use std::future::Future;

use ipc::{Ask, ChangeAsk, FeatureAsks, FileAsk, JobAsks, JobStory, MessageSorting, MessageSortings, SessionId, SetMessageKind};
use ipc::{JobQuestion, JobQuestions, MarkAnswerRead};

use crate::daemon::Refusal;

pub trait Story: Send + Sync + 'static {
    /// `list_message_kinds` — how each message of his in the Job was sorted. [`Refusal::NotFound`] (`fleet.no_such_session`).
    fn list_message_kinds(&self, session_id: SessionId) -> impl Future<Output = Result<MessageSortings, Refusal>> + Send;

    /// `set_message_kind` — his correction of a message, kept as an example for the next sort. [`Refusal::NotFound`]
    /// (`fleet.no_such_session`, `fleet.no_such_message`, `fleet.no_such_ask`), [`Refusal::Unacceptable`] (`fleet.ask_needed`),
    /// [`Refusal::IllegalMove`] (`fleet.ask_has_words`).
    fn set_message_kind(self: std::sync::Arc<Self>, set: SetMessageKind) -> impl Future<Output = Result<MessageSorting, Refusal>> + Send;

    /// `list_job_asks` — the asks said in, built by or handed through the Job. [`Refusal::NotFound`] (`fleet.no_such_session`).
    fn list_job_asks(&self, session_id: SessionId) -> impl Future<Output = Result<JobAsks, Refusal>> + Send;

    /// `get_job_story` — the Job's asks with what he was shown and decided, what was built and what was handed on.
    /// [`Refusal::NotFound`] (`fleet.no_such_session`).
    fn get_job_story(&self, session_id: SessionId) -> impl Future<Output = Result<JobStory, Refusal>> + Send;

    /// `change_ask` — he moves an ask to the backlog or drops it, changes its kind or title, or files it under features.
    /// [`Refusal::NotFound`] (`fleet.no_such_ask`), [`Refusal::Unacceptable`] (`fleet.ask_state_not_set`, `fleet.ask_title_blank`).
    fn change_ask(&self, change: ChangeAsk) -> impl Future<Output = Result<Ask, Refusal>> + Send;

    /// `list_feature_asks` — a project's asks, narrowed to one feature slug where one is given.
    fn list_feature_asks(&self, project: String, feature: Option<String>) -> impl Future<Output = Result<FeatureAsks, Refusal>> + Send;

    /// `file_ask` — the Operator files an ask said outside a Job, under features its project's library holds.
    /// [`Refusal::Unacceptable`] (`fleet.not_the_operator`, `fleet.feature_project`, `fleet.ask_words_blank`), [`Refusal::NotFound`]
    /// (`fleet.feature_unknown`).
    fn file_ask(&self, caller: Option<crate::Caller>, file: FileAsk) -> impl Future<Output = Result<Ask, Refusal>> + Send;

    /// `list_job_questions` — the questions he asked in the Job, each with the reply that answers it and whether he read it.
    /// [`Refusal::NotFound`] (`fleet.no_such_session`).
    fn list_job_questions(&self, session_id: SessionId) -> impl Future<Output = Result<JobQuestions, Refusal>> + Send;

    /// `mark_answer_read` — he read the answer to a question. [`Refusal::NotFound`] (`fleet.no_such_session`,
    /// `fleet.no_such_question`), [`Refusal::IllegalMove`] (`fleet.question_open`).
    fn mark_answer_read(&self, mark: MarkAnswerRead) -> impl Future<Output = Result<JobQuestion, Refusal>> + Send;
}
