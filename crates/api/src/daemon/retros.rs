//! A Job's retro and the Lessons listing. Since 23.12. `docs/concepts/retro.md`.
//!
//! **A seventh surface, for [`Studios`](super::Studios)' reason.** Both reads
//! are about what a Job's record says once it is over, and neither is a fact
//! `get_job` or the Board reads. The two acts on an item came with 23.26.

use std::future::Future;
use std::sync::Arc;

use ipc::{
    AskLessonAnswer, JobId, JobRetro, LandsIn, Lesson, LessonAsk, LessonReview, LessonState,
    Lessons, ManifestId,
};

use crate::daemon::{Redirector, Refusal};

pub trait Retros: Send + Sync + 'static {
    /// `get_job_retro`. [`Refusal::NoSuchJob`] where no Job is `job_id`; a Job
    /// that has not ended answers `pending`, with its record as it stands.
    fn get_job_retro(
        &self,
        job_id: JobId,
    ) -> impl Future<Output = Result<JobRetro, Refusal>> + Send;

    /// `get_session_retro`: retro number `retro` of a Session, or its newest, `pending` with its
    /// record so far where it has none. [`Refusal::NoSuchJob`] where no
    /// Session is `session_id`.
    fn get_session_retro(
        &self,
        session_id: String,
        retro: Option<i64>,
    ) -> impl Future<Output = Result<JobRetro, Refusal>> + Send;

    /// `write_session_retro`: write one now, covering what happened since the
    /// last, and answer when it is written. [`Refusal::IllegalMove`] while one
    /// is being written; [`Refusal::Unacceptable`] where there is nothing new
    /// and no retro to answer with.
    fn write_session_retro(
        self: Arc<Self>,
        session_id: String,
    ) -> impl Future<Output = Result<JobRetro, Refusal>> + Send;

    /// `list_lessons` — up to `most` items in `state`, newest retro first, and
    /// `state` absent is `open`.
    /// `manifest_id` absent is every repository served, and `lands_in` absent
    /// is every place a fix lands, an item kept before 23.15 included.
    fn list_lessons(
        &self,
        manifest_id: Option<ManifestId>,
        lands_in: Option<LandsIn>,
        state: Option<LessonState>,
        most: u32,
    ) -> impl Future<Output = Result<Lessons, Refusal>> + Send;

    /// `agree_lesson`. An item whose fix lands in Armada or in a Manifest
    /// gets a Job proposed at the approval gate, and one whose fix lands in Kit
    /// is kept. **An item already answered answers with where it stands** and
    /// proposes nothing. [`Refusal::NoSuchJob`] where no item has the id.
    ///
    /// `by` is [`Commands::propose_from_request`](super::Commands)' own word:
    /// it is the Job's origin.
    fn agree_lesson(
        self: Arc<Self>,
        lesson_id: String,
        by: Redirector,
    ) -> impl Future<Output = Result<Lesson, Refusal>> + Send;

    /// `disagree_lesson`. The item is kept, `discarded`.
    fn disagree_lesson(
        &self,
        lesson_id: String,
    ) -> impl Future<Output = Result<Lesson, Refusal>> + Send;

    /// `review_lessons`: one model call over the open items, read together.
    /// Nothing is stored. [`Refusal::Unacceptable`] where the Manifest is not
    /// one Fleet serves; an empty open set answers empty and makes no call.
    fn review_lessons(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<LessonReview, Refusal>> + Send;

    /// `ask_lesson`: one model call about one item, answered from its record.
    /// [`Refusal::NoSuchJob`] where no item has the id; [`Refusal::Unacceptable`]
    /// for an empty or over-long question or history.
    fn ask_lesson(
        &self,
        lesson_id: String,
        ask: LessonAsk,
    ) -> impl Future<Output = Result<AskLessonAnswer, Refusal>> + Send;
}
