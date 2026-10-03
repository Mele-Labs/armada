//! `get_job_retro` and `list_lessons`.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Retros};
use ipc::{JobId, JobRetro, Lesson, Lessons, ManifestId, RetroItem, RetroState};
use store::{KeptRetro, Reflected, RetroLine};

use crate::adrift::Adrift;
use crate::daemon::Fleet;

fn item(line: RetroLine) -> RetroItem {
    RetroItem {
        who: line.whose.into(),
        statement: line.said,
        evidence: line.evidence,
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
                    retro.items = items.into_iter().map(item).collect();
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
    /// Jobs where one is named. **A Manifest Fleet does not serve is refused**,
    /// as every narrowed list is.
    async fn list_lessons(
        &self,
        manifest_id: Option<ManifestId>,
        most: u32,
    ) -> Result<Lessons, Refusal> {
        let within = manifest_id;
        let owned = self.owned_by(within.as_ref())?;
        let kept = self.store().lock().await.lessons(most).map_err(|cause| {
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
            let line = item(lesson.line);
            lessons.push(Lesson {
                job_id: (&lesson.job_id).into(),
                handle,
                at: (&lesson.at).into(),
                who: line.who,
                statement: line.statement,
                evidence: line.evidence,
            });
        }
        Ok(Lessons { lessons })
    }
}
