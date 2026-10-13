//! Reading a request into the Jobs it proposes. `Commands::propose_from_request` is one line into
//! here, because a trait impl takes one block and `commanding` reached the 1200-line rule.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;

use crate::adrift::Adrift;
use crate::daemon::Fleet;

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
    /// Read a request and draft the Job it proposes. **The same gate, and the
    /// same `job.created`** — the workflow is the only thing filled in
    /// differently.
    ///
    /// **On a task of its own**, because the Job exists from the press: a
    /// caller that stops waiting drops this future, and the call, the answer
    /// and the move it makes on the Job have to go on without it.
    pub(super) async fn proposed_from_request(
        self: Arc<Self>,
        request: ipc::JobRequest,
        manifest_id: Option<ipc::ManifestId>,
        by: api::Redirector,
    ) -> Result<ipc::ProposedPlan, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        // Refused before any Job exists, so a setting nothing could keep never
        // leaves a proposal half set. `approving` has both halves.
        if let Some(settings) = &request.settings {
            self.settable(&served, settings)
                .map_err(|why| self.refusal(why))?;
        }
        let fleet = Arc::clone(&self);
        let proposing = tokio::spawn(async move {
            let made = fleet
                .propose_from_with_attachments(
                    &request.request,
                    request.client_ref,
                    request.attachments,
                    &served,
                    by,
                    // A composer dispatch is *Dispatched by you*, not Fleet's find.
                    crate::proposal::requested(by),
                )
                .await?;
            if let Some(settings) = &request.settings {
                fleet.dispatched_as_set(&made, settings).await?;
            }
            Ok(made)
        });
        let made = proposing
            .await
            .unwrap_or(Err(Adrift::ProposalAbandoned))
            .map_err(|why| self.refusal(why))?;
        let mut jobs = Vec::with_capacity(made.len());
        for job in &made {
            jobs.push(self.summarised(job).await?);
        }
        Ok(ipc::ProposedPlan { jobs })
    }
}
