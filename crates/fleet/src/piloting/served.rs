//! What Fleet serves for taking a Job over. Since 23.49.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Piloting, Refusal};
use core_model::PilotReason;
use ipc::{HandoffBundle, JobId, JobSummary, PilotNote, PilotOutcome, TakeOver};

use crate::budget::budgeted_for;
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
    /// **The seam for the session host.** Once the Job is `piloted` and the
    /// ledger rows are the Session's, a Session is started by handing
    /// [`Fleet::handoff_bundle`] and the worktree inside it to the session
    /// host's own start. Nothing here starts one.
    async fn take_over(
        self: Arc<Self>,
        job_id: JobId,
        request: TakeOver,
    ) -> Result<JobSummary, Refusal> {
        let reason = match request.outcome {
            PilotOutcome::TakeOver => PilotReason::TakeOver,
            PilotOutcome::RestartStep => PilotReason::RestartStep,
        };
        let session = request.session_id.filter(|id| !id.trim().is_empty());
        let job = budgeted_for(self.command_budget(), job_id.clone(), {
            let fleet = Arc::clone(&self);
            async move {
                Fleet::take_over(&fleet, &job_id.to_domain(), reason, session.as_deref()).await
            }
        })
        .await
        .map_err(|why| self.refusal(why))?;
        self.summarised(&job).await
    }

    async fn get_handoff(&self, job_id: JobId) -> Result<HandoffBundle, Refusal> {
        self.handoff_bundle(job_id).await
    }

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
