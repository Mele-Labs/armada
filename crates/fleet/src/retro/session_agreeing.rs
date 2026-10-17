//! A person's answer to an item of a Session's retro. `agreeing` has the
//! rules, and these are they, for an item named by a Session, its retro's
//! number and its place. **Where the Job goes** differs: a `manifest` fix is
//! proposed on the repository the Session was started in.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::LessonState;
use ipc::{Lesson, WireError};
use store::KeptSessionLesson;

use super::agreeing::NO_SUCH_LESSON;
use super::serving::session_lesson_of;
use super::session_writing::session_of;
use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// A `manifest` fix, and the Session was started in no repository Fleet knows.
pub(super) const SESSION_NAMES_NO_REPOSITORY: &str = "fleet.lesson_session_names_no_repository";

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

    pub(super) async fn disagreed_with_session(
        &self,
        lesson_id: &str,
        (session, retro, ordinal): (String, i64, u32),
    ) -> Result<Lesson, Refusal> {
        self.session_lesson_held(lesson_id, &session, retro, ordinal).await?;
        self.store()
            .lock()
            .await
            .answer_session_lesson(retro, ordinal, LessonState::Discarded)
            .map_err(|why| self.refusal(Adrift::Writing(why)))?;
        let held = self.session_lesson_held(lesson_id, &session, retro, ordinal).await?;
        self.session_lesson_standing(held).await
    }

    /// The item, only if the retro it names is the Session's.
    pub(super) async fn session_lesson_held(
        &self,
        lesson_id: &str,
        session: &str,
        retro: i64,
        ordinal: u32,
    ) -> Result<KeptSessionLesson, Refusal> {
        self.store()
            .lock()
            .await
            .session_lesson(retro, ordinal)
            .map_err(|cause| self.refusal(Adrift::Reading(store::ReadError::Unreadable(cause))))?
            .filter(|held| held.session_id == session)
            .ok_or_else(|| {
                Refusal::NotFound(WireError::raised(
                    NO_SUCH_LESSON,
                    format!("no retro item is named {lesson_id}"),
                    self.run_id(),
                ))
            })
    }

    pub(super) async fn session_lesson_standing(&self, held: KeptSessionLesson) -> Result<Lesson, Refusal> {
        let kept = self
            .store()
            .lock()
            .await
            .session(&held.session_id)
            .map_err(|why| self.ledger_fault(why))?;
        let named = kept.as_ref().map(session_of).unwrap_or_else(|| ipc::RetroSession {
            id: held.session_id.clone(),
            title: None,
        });
        Ok(session_lesson_of(held, named))
    }
}
