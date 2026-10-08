//! Merge pressed while the forge's checks still run: the forge is asked to
//! merge when they pass, and the Job stays at its gate until it has.
//!
//! **The ask is recorded on the Job**, and only a merge asked for here lets the
//! sweep take the work off the gate. A pull request someone merged on the forge
//! is left for a person, as it always was: Fleet did not decide that one.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{PullRequests, Refusal};
use core_model::{Actor, Component, Envelope, FieldValue, JobId, Level};
use ipc::{ManifestId, PullRequestState};

use crate::daemon::Fleet;
use crate::pull_requesting::number_of;

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
    /// Ask the forge to merge this Job's pull request when its checks pass,
    /// and remember that the ask came from here. **Refused off the gate**, like
    /// the merge it stands in for, and by the forge's own rule once the checks
    /// have passed (merge it) or failed.
    pub(crate) async fn enable_job_auto_merge(
        &self,
        job_id: &JobId,
    ) -> Result<PullRequestState, Refusal> {
        let job = self.load(job_id).await.map_err(|why| self.refusal(why))?;
        self.at_the_gate(&job).map_err(|why| self.refusal(why))?;
        let url = self
            .pull_request_of(job_id)
            .await
            .map_err(|why| self.refusal(why))?;
        let number = number_of(&url).ok_or_else(|| {
            self.refusal(crate::Adrift::NothingToMerge {
                job: job_id.clone(),
            })
        })?;
        let manifest = ManifestId::carried(job.owner_manifest_id().as_str().to_string());
        let state = PullRequests::enable_auto_merge(self, manifest, number).await?;
        let recorded = self
            .store()
            .lock()
            .await
            .record_auto_merge_asked(job_id, &self.now());
        let _ = recorded;
        self.logged(
            job_id,
            Envelope::new(
                self.now(),
                Level::Warn,
                Component::Fleet,
                self.run().clone(),
                "a person asked the forge to merge the pull request when its checks pass",
            )
            .in_job(job_id.as_ulid().clone())
            .with_field("pull_request", FieldValue::Str(url)),
        );
        Ok(state)
    }

    /// The forge merged a pull request this Job's press asked it to: take the
    /// work, as a press on Merge would have. **Held, never raised**: this is a
    /// sweep, and a Job that moved on meanwhile is not a failure.
    pub(crate) async fn completed_by_auto_merge(&self, job_id: &JobId) {
        let Ok(job) = self.load(job_id).await else {
            return;
        };
        if self.at_the_gate(&job).is_err() {
            return;
        }
        if !matches!(self.store().lock().await.auto_merge_asked(job_id), Ok(true)) {
            return;
        }
        let _ = self.approved(job_id, Actor::Fleet).await;
    }
}
