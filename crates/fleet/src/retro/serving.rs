//! `get_job_retro`, `list_lessons`, and the two acts on a Lesson.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Redirector, Refusal, Retros};
use ipc::{
    JobId, JobRetro, LandsIn, Lesson, LessonState, Lessons, ManifestId, RetroItem, RetroState,
};
use store::{KeptLesson, KeptRetro, Reflected, RetroLine};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// What names one item: its Job and its place in that Job's retro. **A retro
/// is written once**, so the place never moves. A ULID holds no hyphen, which
/// is what [`parsed`] splits on.
pub(crate) fn lesson_id(job: &str, ordinal: usize) -> String {
    format!("{job}-{ordinal}")
}

/// The Job and the place an id names, or `None` where it names neither.
pub(crate) fn parsed(id: &str) -> Option<(core_model::JobId, u32)> {
    let (job, ordinal) = id.rsplit_once('-')?;
    let ordinal = ordinal.parse().ok()?;
    let job = core_model::JobId::carried(core_model::Ulid::carried(job));
    (!job.as_str().is_empty()).then_some((job, ordinal))
}

fn item(job: &str, ordinal: usize, line: RetroLine) -> RetroItem {
    RetroItem {
        id: lesson_id(job, ordinal),
        who: line.whose.into(),
        title: line.title,
        what: line.what,
        fix: line.fix,
        statement: line.said,
        evidence: line.evidence,
        lands_in: line.lands_in.map(LandsIn::from),
    }
}

/// One row of the Lessons listing, with the handle of the Job it came from.
pub(crate) fn lesson_of(lesson: KeptLesson, handle: String) -> Lesson {
    let line = item(lesson.job_id.as_str(), lesson.ordinal as usize, lesson.line);
    Lesson {
        id: line.id,
        job_id: (&lesson.job_id).into(),
        handle,
        at: (&lesson.at).into(),
        who: line.who,
        title: line.title,
        what: line.what,
        fix: line.fix,
        statement: line.statement,
        evidence: line.evidence,
        lands_in: line.lands_in,
        state: LessonState::from(lesson.state),
        job_proposed: lesson.job_proposed.as_ref().map(JobId::from),
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
    /// The record as it stands, and the retro where one is kept.
    async fn get_job_retro(&self, job_id: JobId) -> Result<JobRetro, Refusal> {
        let id = job_id.to_domain();
        let gathered = self.retro_record(&id).await?;
        let kept = self.store().lock().await.retro_for(&id).map_err(|cause| {
            self.refusal(Adrift::Reading(store::LoadJobError::Unreadable(cause)))
        })?;
        let mut retro = JobRetro {
            job_id,
            state: RetroState::Pending,
            at: None,
            model: None,
            why: None,
            items: Vec::new(),
            record: gathered.record,
            annotations: gathered.annotations,
        };
        if let Some(KeptRetro { reflected, at }) = kept {
            retro.at = Some((&at).into());
            match reflected {
                Reflected::Written { model, items } => {
                    retro.state = RetroState::Written;
                    retro.model = Some(model);
                    retro.items = items
                        .into_iter()
                        .enumerate()
                        .map(|(ordinal, line)| item(id.as_str(), ordinal, line))
                        .collect();
                }
                Reflected::Failed { why } => {
                    retro.state = RetroState::Failed;
                    retro.why = Some(why);
                }
                Reflected::Skipped { why } => {
                    retro.state = RetroState::Skipped;
                    retro.why = Some(why);
                }
            }
        }
        Ok(retro)
    }

    /// Every written retro's items, newest first, narrowed to one repository's
    /// Jobs where one is named, to where each fix lands where that is, and to
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
        let owned = self.owned_by(within.as_ref())?;
        let kept = self
            .store()
            .lock()
            .await
            .lessons(
                most,
                lands_in.map(|lands| lands.domain()),
                Some(state.map_or(core_model::LessonState::Open, |state| state.domain())),
            )
            .map_err(|cause| {
                self.refusal(Adrift::Reading(store::LoadJobError::Unreadable(cause)))
            })?;
        let mut lessons = Vec::new();
        for lesson in kept {
            // **A Job that will not load keeps its items**, named by its id: a
            // listing that dropped them would say less went wrong than did.
            let handle = match self.load(&lesson.job_id).await {
                Ok(job) if !owned(&job) => continue,
                Ok(job) => job.handle(),
                Err(_) => lesson.job_id.as_str().to_string(),
            };
            lessons.push(lesson_of(lesson, handle));
        }
        Ok(Lessons { lessons })
    }

    /// Agree with an item. `agreeing` has it.
    async fn agree_lesson(
        self: Arc<Self>,
        lesson_id: String,
        by: Redirector,
    ) -> Result<Lesson, Refusal> {
        self.agreed_with(&lesson_id, by).await
    }

    /// Disagree with an item. `agreeing` has it.
    async fn disagree_lesson(&self, lesson_id: String) -> Result<Lesson, Refusal> {
        self.disagreed_with(&lesson_id).await
    }
}
