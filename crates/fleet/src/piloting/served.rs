//! What Fleet serves for ending a pilot. Since 23.49.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Piloting, Refusal};
use ipc::{JobId, JobSummary, PilotNote};

use crate::daemon::Fleet;

impl<H, V, W> Piloting for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// **Not budgeted.** The request waits for the Checks, however long they
    /// take, as `rerun_checks` does.
    async fn submit_for_verification(
        self: Arc<Self>,
        job_id: JobId,
    ) -> Result<JobSummary, Refusal> {
        let job = Fleet::submit_for_verification(Arc::clone(&self), &job_id.to_domain())
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    async fn attest_complete(
        self: Arc<Self>,
        job_id: JobId,
        note: PilotNote,
    ) -> Result<JobSummary, Refusal> {
        let job = Fleet::attest_complete(&self, &job_id.to_domain(), note.note.as_deref())
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    async fn close_as_superseded(
        self: Arc<Self>,
        job_id: JobId,
        note: PilotNote,
    ) -> Result<JobSummary, Refusal> {
        let job = Fleet::close_as_superseded(&self, &job_id.to_domain(), note.note.as_deref())
            .await
            .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }
}
