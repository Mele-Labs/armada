//! `list_write_clashes`: what Fleet last found, drawn as a person would name it.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Clashing, Refusal};
use ipc::{
    ClashFile, ClashHolder, ClashSide, ManifestId, RepositoryAreas, SessionFileChange, SlotTouches, WriteClash,
    WriteClashes,
};

use super::{Side, Who};
use crate::daemon::Fleet;

fn kind_and_id(who: &Who) -> (ClashSide, String) {
    match who {
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

impl<H, V, W> Clashing for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn list_write_clashes(
        &self,
        manifest_id: Option<ManifestId>,
        holder: Option<String>,
    ) -> Result<WriteClashes, Refusal> {
        self.write_clashes(manifest_id, holder).await
    }

    async fn read_clash_file(
        &self,
        session_id: String,
        other_kind: ClashSide,
        other_id: String,
        path: String,
    ) -> Result<ClashFile, Refusal> {
        self.clash_file(&session_id, other_kind, &other_id, &path).await
    }

    async fn get_session_file_change(
        &self,
        session_id: String,
        path: String,
        seen: Option<String>,
    ) -> Result<SessionFileChange, Refusal> {
        self.session_file_change(&session_id, &path, seen).await
    }

    async fn list_repository_areas(&self) -> Result<RepositoryAreas, Refusal> {
        self.repository_areas().await
    }

    async fn list_slot_touches(&self) -> Result<SlotTouches, Refusal> {
        self.slot_touches().await
    }
}
