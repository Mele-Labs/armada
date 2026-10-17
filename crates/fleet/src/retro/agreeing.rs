//! A person's answer to a retro item: agree or disagree.
//! `docs/concepts/retro.md`, *Agree and disagree*.
//!
//! **Agreeing is the one place a retro acts**, and it acts by proposing: a fix
//! that lands in Armada or in a Manifest becomes a Job at the approval gate,
//! where a person approves it like any other. A fix that lands in Kit has
//! nothing to dispatch and is kept as accepted. Disagreeing keeps the row and
//! lists it nowhere a person has not asked for.
//!
//! **The first press wins, and the write is the claim.** An item moves off
//! `open` in one conditional update, so two presses make one Job.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use core_model::LessonState;
use ipc::{Lesson, ManifestId, WireError};
use store::KeptLesson;

use super::serving::{lesson_of, parsed};
use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// No item answers to that id. A 404.
pub(super) const NO_SUCH_LESSON: &str = "fleet.no_such_lesson";
/// An item written before 23.15, which names no place its fix lands. A 422:
/// where its Job would go is not Fleet's to guess.
pub(super) const LESSON_NAMES_NO_PLACE: &str = "fleet.lesson_names_no_place";
/// A fix that lands in Armada, and the repository Armada is built in is not
/// one this Fleet serves. A 422.
const ARMADA_NOT_SERVED: &str = "fleet.lesson_armada_not_served";

/// The Manifest id of the repository Armada itself is built in. **A name and
/// not a probe**: Fleet holds no marker for which repository it is, and a
/// machine that serves Armada serves it under this id.
const ARMADAS_OWN: &str = "armada";

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

    /// Disagree with an item. The row stays, `discarded`. **An item already
    /// answered answers with where it stands**: a Job proposed is never
    /// taken back by pressing this.
    pub(crate) async fn disagreed_with(&self, lesson_id: &str) -> Result<Lesson, Refusal> {
        if let Some(session) = super::serving::parsed_session(lesson_id) {
            return self.disagreed_with_session(lesson_id, session).await;
        }
        let (job_id, ordinal) = parsed(lesson_id).ok_or_else(|| self.no_such_lesson(lesson_id))?;
        self.lesson_held(lesson_id, &job_id, ordinal).await?;
        self.store()
            .lock()
            .await
            .answer_lesson(&job_id, ordinal, LessonState::Discarded)
            .map_err(|why| self.refusal(Adrift::Writing(why)))?;
        let held = self.lesson_held(lesson_id, &job_id, ordinal).await?;
        self.lesson_standing(held).await
    }

    pub(super) fn no_such_lesson(&self, lesson_id: &str) -> Refusal {
        Refusal::NoSuchJob(WireError::raised(
            NO_SUCH_LESSON,
            format!("no retro item is named {lesson_id}"),
            self.run_id(),
        ))
    }

    pub(super) async fn lesson_held(
        &self,
        lesson_id: &str,
        job_id: &core_model::JobId,
        ordinal: u32,
    ) -> Result<KeptLesson, Refusal> {
        self.store()
            .lock()
            .await
            .lesson(job_id, ordinal)
            .map_err(|cause| self.refusal(Adrift::Reading(store::LoadJobError::Unreadable(cause))))?
            .ok_or_else(|| self.no_such_lesson(lesson_id))
    }

    /// The item as it stands, with its Job's handle.
    pub(super) async fn lesson_standing(&self, held: KeptLesson) -> Result<Lesson, Refusal> {
        let handle = self.lesson_handle(&held.job_id).await;
        Ok(lesson_of(held, handle))
    }

    /// A Job's handle, or its id where it will not load.
    async fn lesson_handle(&self, job_id: &core_model::JobId) -> String {
        match self.load(job_id).await {
            Ok(job) => job.handle(),
            Err(_) => job_id.as_str().to_string(),
        }
    }

    /// Armada's own repository, where this Fleet serves it.
    pub(super) fn armada_served(&self, lesson_id: &str) -> Result<ManifestId, Refusal> {
        let own = ManifestId::carried(ARMADAS_OWN);
        self.served_named(Some(&own)).map(|_| own).map_err(|_| {
            Refusal::Unacceptable(WireError::raised(
                ARMADA_NOT_SERVED,
                format!(
                    "{lesson_id} is a fix in Armada, and this Fleet does not serve Armada's own \
                     repository (`{ARMADAS_OWN}`), so there is nowhere to propose its Job. \
                     Serving it, then agreeing again, proposes it"
                ),
                self.run_id(),
            ))
        })
    }
}

