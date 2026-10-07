//! What Fleet serves for running one Check or Command by hand.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Rehearsing};
use ipc::JobId;

use crate::daemon::Fleet;

impl<H, V, W> Rehearsing for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// A person's run in a Job's worktree. **The `Arc` is handed on**, so the
    /// run is a task of its own — `crate::rehearsing`.
    ///
    /// **Not [`budgeted`](crate::budget::budgeted)**: a rehearsal answers through its own refusal type,
    /// never [`Adrift`], because it is not a move on the Job at all.
    async fn start_run(
        self: std::sync::Arc<Self>,
        job_id: JobId,
        run: ipc::StartRun,
    ) -> Result<ipc::RunUnderway, Refusal> {
        Fleet::start_rehearsal(self, &job_id.to_domain(), run).await
    }

    /// **Not [`budgeted`](crate::budget::budgeted)**, for [`Rehearsing::start_run`]'s reason.
    async fn stop_run(&self, job_id: JobId, run: ipc::NamedRun) -> Result<ipc::RunRecord, Refusal> {
        self.stop_rehearsal(&job_id.to_domain(), run.id).await
    }

    /// **Not [`budgeted`](crate::budget::budgeted)**, for [`Rehearsing::start_run`]'s reason.
    async fn undo_run(&self, job_id: JobId, run: ipc::NamedRun) -> Result<ipc::RunRecord, Refusal> {
        self.undo_rehearsal(&job_id.to_domain(), run.id).await
    }

    /// A person's run in the main checkout. **The `Arc` is handed on**, for
    /// [`Rehearsing::start_run`]'s reason — `crate::rehearsing::checkout`.
    async fn start_checkout_run(
        self: std::sync::Arc<Self>,
        run: ipc::StartCheckoutRun,
        manifest_id: Option<ipc::ManifestId>,
        repository: Option<String>,
    ) -> Result<ipc::CheckoutRunUnderway, Refusal> {
        let checkout = self.checkout_named(manifest_id.as_ref(), repository.as_deref())?;
        Fleet::start_checkout_rehearsal(self, run, checkout).await
    }

    /// **Not [`budgeted`](crate::budget::budgeted)**, for [`Rehearsing::start_run`]'s reason.
    async fn stop_checkout_run(
        &self,
        run: ipc::NamedRun,
        manifest_id: Option<ipc::ManifestId>,
        repository: Option<String>,
    ) -> Result<ipc::CheckoutRunRecord, Refusal> {
        let checkout = self.checkout_named(manifest_id.as_ref(), repository.as_deref())?;
        self.stop_checkout_rehearsal(run.id, checkout).await
    }

    /// **Not [`budgeted`](crate::budget::budgeted)**, for [`Rehearsing::start_run`]'s reason.
    async fn undo_checkout_run(
        &self,
        run: ipc::NamedRun,
        manifest_id: Option<ipc::ManifestId>,
        repository: Option<String>,
    ) -> Result<ipc::CheckoutRunRecord, Refusal> {
        let checkout = self.checkout_named(manifest_id.as_ref(), repository.as_deref())?;
        self.undo_checkout_rehearsal(run.id, checkout).await
    }

    /// Verify in the main checkout. **The `Arc` is handed on**: its steps
    /// outlive the call — `crate::rehearsing::verifying`.
    async fn start_checkout_verify(
        self: std::sync::Arc<Self>,
        asked: ipc::StartCheckoutVerify,
        manifest_id: Option<ipc::ManifestId>,
        repository: Option<String>,
    ) -> Result<ipc::CheckoutVerify, Refusal> {
        let checkout = self.checkout_named(manifest_id.as_ref(), repository.as_deref())?;
        Fleet::begin_checkout_verify(self, checkout, asked.workspace.as_deref()).await
    }
}
