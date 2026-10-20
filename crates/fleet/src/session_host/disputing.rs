//! `dispute_fleet`: a hosted job says Fleet's latest notice in it was wrong.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{DisputeFleet, FleetDisputed};

use crate::daemon::Fleet;

const DISPUTE_REFUSED: &str = "fleet.dispute_refused";

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
    pub(crate) async fn fleet_disputed(
        self: &Arc<Self>,
        _caller: Option<api::Caller>,
        _dispute: DisputeFleet,
    ) -> Result<FleetDisputed, Refusal> {
        Err(self.hosted_refusal(DISPUTE_REFUSED, "not built yet"))
    }
}
