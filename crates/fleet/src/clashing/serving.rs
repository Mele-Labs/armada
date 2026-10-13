//! `list_write_clashes`: what Fleet last found, drawn as a person would name it.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{ClashHolder, ClashSide, ManifestId, WriteClash, WriteClashes};

use super::{Side, Who};
use crate::daemon::Fleet;

fn kind_and_id(who: &Who) -> (ClashSide, String) {
    match who {
        Who::Job(id) => (ClashSide::Job, id.as_str().to_string()),
        Who::Session(id) => (ClashSide::Session, id.clone()),
    }
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
    async fn clash_holder(&self, who: &Who) -> ClashHolder {
        let (kind, id) = kind_and_id(who);
        let label = match self.peer_of(who).await {
            Some(peer) => peer.label(),
            None => id.clone(),
        };
        ClashHolder { kind, id, label }
    }

    /// Every clash standing in one repository, oldest first, or those `holder` is in.
    pub(crate) async fn write_clashes(
        &self,
        manifest_id: Option<ManifestId>,
        holder: Option<String>,
    ) -> Result<WriteClashes, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        let base = served.manifest().base().unwrap_or("main").to_string();
        let mut readings = self.writes().readings_in(served.manifest().id().as_str());
        readings.sort_by(|(_, one), (_, other)| one.since.as_str().cmp(other.since.as_str()));
        let mut clashes = Vec::new();
        for (opposed, reading) in readings {
            let one = self.clash_holder(&opposed.0).await;
            let other = match &opposed.1 {
                Side::Holder(other) => self.clash_holder(other).await,
                Side::Main => ClashHolder {
                    kind: ClashSide::Main,
                    id: base.clone(),
                    label: base.clone(),
                },
            };
            if holder.as_deref().is_some_and(|wanted| one.id != wanted && other.id != wanted) {
                continue;
            }
            clashes.push(WriteClash {
                one,
                other,
                paths: reading.paths.into_iter().collect(),
                since: ipc::Instant::carried(reading.since.as_str()),
            });
        }
        Ok(WriteClashes { clashes })
    }
}
