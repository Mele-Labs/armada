//! An agent's `approves` item about an artifact that already asks him is the artifact's own Approve said twice.
//! `waiting_for` refuses it as the agent writes it, and one kept comes off his list when he approves the artifact.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{ArtifactState, SessionRecord};

use crate::daemon::Fleet;

/// What the agent reads when its list is refused for an `approves` item naming an artifact that asks him itself.
pub(crate) fn artifact_asks_refusal(id: &str, artifact: &str) -> String {
    format!(
        "Refused: item \"{id}\" asks him to approve \"{artifact}\", which already asks him with its own Approve, so the \
         item is not needed. Call waiting_for again without it: Fleet lists the artifact on his Needs you."
    )
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
    /// Whether the artifact the Job holds as `named` is put to him and not yet answered.
    pub(crate) async fn artifact_asks_him(&self, id: &str, named: &str) -> bool {
        self.artifact_held(id, named).await.is_ok_and(|artifact| artifact.state == ArtifactState::Asking)
    }

    /// **His approval of an artifact settles the agent's `approves` items that name it**, as his answer to one settles
    /// it: the item goes and the Job is told nothing more, since the approval reached it already. An item naming no
    /// artifact stands. Returns the Job's row as it reads now, or `record` where nothing went.
    pub(crate) async fn approved_items_settled(self: &Arc<Self>, id: &str, record: SessionRecord) -> SessionRecord {
        let kept = {
            let store = self.store().lock().await;
            store.waiting_for(id).unwrap_or_default()
        };
        let mut gone = Vec::new();
        for item in kept.iter().filter(|item| item.approves && !item.artifact.is_empty()) {
            if self.artifact_held(id, &item.artifact).await.is_ok_and(|artifact| artifact.state == ArtifactState::Approved) {
                gone.push(item.item_id.clone());
            }
        }
        if gone.is_empty() {
            return record;
        }
        let row = {
            let mut store = self.store().lock().await;
            let Ok(Some(session)) = store.session(id) else { return record };
            if gone.iter().any(|item| store.dismiss_waiting(id, item).is_err()) {
                return record;
            }
            match self.ledger_row(&store, &session) {
                Ok(row) => row,
                Err(_) => return record,
            }
        };
        self.publish(ipc::Event::SessionChanged(row.clone()));
        row
    }
}
