//! What the fleet has spent, and which Jobs a ceiling is holding.
//!
//! **Summed here and stored nowhere.** A total written down is wrong the
//! moment a Drone ends, and the per-Job rows it sums are the record.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::FleetUsage;

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
    /// `get_usage` — the fleet's spend, and what the ceilings are holding.
    ///
    /// **No Job is held by a ceiling any more**, so `over_budget` is always empty.
    pub(crate) async fn usage(&self) -> Result<FleetUsage, Refusal> {
        let (loaded, _) = self.every_job().await.map_err(|why| self.refusal(why))?;
        let mut usage = FleetUsage {
            cost_micros: 0,
            turns: 0,
            drones: 0,
            unpriced: 0,
            jobs: loaded.jobs.len() as u64,
            over_budget: Vec::new(),
        };
        for job in &loaded.jobs {
            let spent = self
                .spend_of(job.id())
                .await
                .map_err(|why| self.refusal(why))?;
            usage.cost_micros = usage.cost_micros.saturating_add(spent.cost_micros);
            usage.turns = usage.turns.saturating_add(spent.turns);
            usage.drones = usage.drones.saturating_add(spent.drones);
            usage.unpriced = usage.unpriced.saturating_add(spent.unpriced);
        }
        Ok(usage)
    }
}
