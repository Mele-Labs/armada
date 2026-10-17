//! `list_lessons`, the Session retro reads, and the acts on a Lesson.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Retros};
use ipc::{LandsIn, Lesson, LessonState, Lessons, ManifestId, Retro, RetroChange, RetroItem};
use store::RetroLine;

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// What names one item of a Session's retro: the Session, its retro's number
/// and the item's place. A Session's id holds hyphens, so the number is
/// marked `r`, which [`parsed_session`] reads back.
pub(crate) fn session_lesson_id(session: &str, retro: i64, ordinal: usize) -> String {
    format!("{session}-r{retro}-{ordinal}")
}

/// The Session, retro and place an id names, or `None` where it names none.
pub(crate) fn parsed_session(id: &str) -> Option<(String, i64, u32)> {
    let (rest, ordinal) = id.rsplit_once('-')?;
    let (session, retro) = rest.rsplit_once('-')?;
    let retro = retro.strip_prefix('r')?.parse().ok()?;
    (!session.is_empty()).then_some((session.to_string(), retro, ordinal.parse().ok()?))
}

pub(crate) fn item_named(id: String, line: RetroLine) -> RetroItem {
    RetroItem {
        id,
        who: line.whose.into(),
        title: line.title,
        what: line.what,
        fix: line.fix,
        statement: line.said,
        evidence: line.evidence,
        lands_in: line.lands_in.map(LandsIn::from),
        state: None,
        session_proposed: None,
        change: line.change.as_ref().map(RetroChange::from),
        applied: None,
    }
}

/// One row of the Lessons listing for a Session's item. Its `handle` is the
/// Session's address.
pub(crate) fn session_lesson_of(
    lesson: store::KeptSessionLesson,
    session: ipc::RetroSession,
) -> Lesson {
    let id = session_lesson_id(&lesson.session_id, lesson.retro_id, lesson.ordinal as usize);
    let line = item_named(id, lesson.line);
    Lesson {
        id: line.id,
        handle: super::session_writing::handle_of(&lesson.session_id),
        at: (&lesson.at).into(),
        who: line.who,
        title: line.title,
        what: line.what,
        fix: line.fix,
        statement: line.statement,
        evidence: line.evidence,
        lands_in: line.lands_in,
        state: LessonState::from(lesson.state),
        session_proposed: lesson.session_proposed,
        session,
        applied: lesson.applied.then(|| line.change.clone()).flatten(),
        change: line.change,
    }
}

impl<H, V, W> Retros for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Every written retro's items, newest first, narrowed to one repository's
    /// Sessions where one is named, to where each fix lands where that is, and to
    /// one state. **A Manifest Fleet does not serve is refused**, as every
    /// narrowed list is.
    async fn list_lessons(
        &self,
        manifest_id: Option<ManifestId>,
        lands_in: Option<LandsIn>,
        state: Option<LessonState>,
        most: u32,
    ) -> Result<Lessons, Refusal> {
        let within = manifest_id;
        let mut lessons = Vec::new();
        let sessions = self
            .store()
            .lock()
            .await
            .session_lessons(
                most,
                lands_in.map(|lands| lands.domain()),
                Some(state.map_or(core_model::LessonState::Open, |state| state.domain())),
            )
            .map_err(|cause| {
                self.refusal(Adrift::Reading(store::ReadError::Unreadable(cause)))
            })?;
        for lesson in sessions {
            let kept = self
                .store()
                .lock()
                .await
                .session(&lesson.session_id)
                .ok()
                .flatten();
            let Some(kept) = kept else { continue };
            if let Some(within) = &within {
                if kept.manifest_id.as_deref() != Some(within.as_str()) {
                    continue;
                }
            }
            lessons.push(session_lesson_of(lesson, super::session_writing::session_of(&kept)));
        }
        lessons.sort_by(|one, other| other.at.as_str().cmp(one.at.as_str()));
        lessons.truncate(most as usize);
        Ok(Lessons { lessons })
    }

    async fn get_session_retro(
        &self,
        session_id: String,
        retro: Option<i64>,
    ) -> Result<Retro, Refusal> {
        self.read_session_retro(&session_id, retro).await
    }

    async fn write_session_retro(
        self: Arc<Self>,
        session_id: String,
    ) -> Result<Retro, Refusal> {
        Fleet::write_session_retro(&self, &session_id).await
    }

    /// Start a Session for an item. `starting` has it.
    async fn start_lesson_session(
        self: Arc<Self>,
        lesson_id: String,
    ) -> Result<ipc::LessonSessionStarted, Refusal> {
        self.started_from(&lesson_id).await
    }

    /// Disagree with an item. `agreeing` has it.
    async fn disagree_lesson(&self, lesson_id: String) -> Result<Lesson, Refusal> {
        self.disagreed_with(&lesson_id).await
    }

    /// Read the open items together. `reviewing` has it.
    async fn review_lessons(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> Result<ipc::LessonReview, Refusal> {
        self.reviewed(manifest_id).await
    }

    /// Answer a question about an item. `asking` has it.
    async fn ask_lesson(
        &self,
        lesson_id: String,
        ask: ipc::LessonAsk,
    ) -> Result<ipc::AskLessonAnswer, Refusal> {
        self.asked_about(&lesson_id, ask).await
    }
}
