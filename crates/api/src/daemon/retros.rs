//! A Session's retro and the Lessons listing. `docs/concepts/retro.md`.
//!
//! **A seventh surface, for [`Studios`](super::Studios)' reason.** Both reads
//! are about what a Session's thread says once it is over.

use std::future::Future;
use std::sync::Arc;

use ipc::{
    AskLessonAnswer, Retro, LandsIn, Lesson, LessonAsk, LessonReview, LessonSessionStarted,
    LessonState,
    Lessons, ManifestId,
};

use crate::daemon::Refusal;

pub trait Retros: Send + Sync + 'static {

    /// `get_session_retro`: retro number `retro` of a Session, or its newest, `pending` with its
    /// record so far where it has none. [`Refusal::NotFound`] where no
    /// Session is `session_id`.
    fn get_session_retro(
        &self,
        session_id: String,
        retro: Option<i64>,
    ) -> impl Future<Output = Result<Retro, Refusal>> + Send;

    /// `write_session_retro`: write one now, covering what happened since the
    /// last, and answer when it is written. [`Refusal::IllegalMove`] while one
    /// is being written; [`Refusal::Unacceptable`] where there is nothing new
    /// and no retro to answer with.
    fn write_session_retro(
        self: Arc<Self>,
        session_id: String,
    ) -> impl Future<Output = Result<Retro, Refusal>> + Send;

    /// `list_lessons` — up to `most` items in `state`, newest retro first, and
    /// `state` absent is `open`.
    /// `manifest_id` absent is every repository served, and `lands_in` absent
    /// is every place a fix lands.
    fn list_lessons(
        &self,
        manifest_id: Option<ManifestId>,
        lands_in: Option<LandsIn>,
        state: Option<LessonState>,
        most: u32,
    ) -> impl Future<Output = Result<Lessons, Refusal>> + Send;


    /// `start_lesson_session`: agreeing by starting a hosted Session on the
    /// item's repository, and its first message sent at once, so the Session
    /// runs. A `kit` item is
    /// [`Refusal::Unacceptable`]. An item already answered answers with the
    /// Session started for it, and an answered item with none is
    /// [`Refusal::IllegalMove`]. A send that is refused leaves the item agreed
    /// with the Session, and is [`Refusal::Unacceptable`] naming it.
    fn start_lesson_session(
        self: Arc<Self>,
        lesson_id: String,
    ) -> impl Future<Output = Result<LessonSessionStarted, Refusal>> + Send;

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
    /// [`Refusal::NotFound`] where no item has the id; [`Refusal::Unacceptable`]
    /// for an empty or over-long question or history.
    fn ask_lesson(
        &self,
        lesson_id: String,
        ask: LessonAsk,
    ) -> impl Future<Output = Result<AskLessonAnswer, Refusal>> + Send;
}
