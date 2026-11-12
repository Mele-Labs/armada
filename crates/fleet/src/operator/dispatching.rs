//! STUB for the red run.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;

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
    pub(crate) async fn job_dispatched(self: &Arc<Self>, _dispatch: ipc::DispatchJob) -> Result<ipc::JobDispatched, Refusal> {
        Err(Refusal::Unacceptable(ipc::WireError::raised("fleet.not_built", "not built", self.run_id())))
    }

    pub async fn dispatches_resumed(self: &Arc<Self>) {}
}
