//! What Fleet does when it starts: the boot read, and the cleanup a crashed Fleet leaves.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};

use super::Fleet;
use crate::adrift::Adrift;

/// What the boot read found, and what the reconciliation did about it.
#[derive(Debug, Default)]
pub struct Reconciled {
    /// Links on a Studio whose address an adapter recognised, written back as the Issue, Pull
    /// request or Epic they name. Nought on every boot after the one that converted them.
    pub recognised: usize,
}

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
    /// **The boot read, and the reconciliation.** No command is answered until it has: `armada
    /// serve` runs it in a task after the router is up, with reads and health answering meanwhile
    /// and commands held by `api::Reconciliation`.
    pub async fn reconcile(&self) -> Result<Reconciled, Adrift> {
        // First of all: the servers a crashed Fleet left running are ended, so the ports they
        // held are free before the probe below. See `crate::servers::left`.
        for repository in self.repositories().every() {
            self.reaped_left_servers(repository.records_root()).await;
        }
        // Before anything else: a main-checkout claim a crashed Fleet left behind is re-probed
        // before this process trusts it. See `crate::ports::Fleet::reconciled_checkout_ports`.
        self.reconciled_checkout_ports().await;
        // A scout does not outlive the Fleet reading it. `crate::scouting`.
        self.scouts_left_gathering().await;
        self.rescues_left_reading().await;
        // Housekeeping: a store that will not delete now leaves them listed.
        let _ = self.store().lock().await.forget_blank_sessions();
        // Before any Studio is read: the Links this build reads as an Issue, a Pull request or an
        // Epic. `crate::recognising`.
        let recognised = self.links_recognised().await;
        for served in self.repositories().hosted() {
            self.reconciled_in_part(&served).await;
        }
        Ok(Reconciled { recognised })
    }

    /// The same cleanup for one repository added after Fleet started: the servers it left.
    pub(crate) async fn reconciled_in(
        &self,
        served: &crate::repositories::Served,
    ) -> Result<Reconciled, Adrift> {
        self.reaped_left_servers(served.records_root()).await;
        self.reconciled_in_part(served).await;
        Ok(Reconciled::default())
    }

    /// One repository's cleanup.
    async fn reconciled_in_part(&self, served: &crate::repositories::Served) {
        // A slot still held by a session that ended: parked now, before anything looks for a
        // free one.
        self.freed_from_ended_sessions(served).await;
    }

    /// The stream Fleet publishes transitions on. Cloned for the listener.
    pub fn events(&self) -> api::Broadcaster {
        self.events.clone()
    }
}
