//! `tell_session`: a Job on your words passes words to a live Job. `crates/ipc/operations/tell_session.toml`.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{HostedSessions, Refusal};
use ipc::{Instant, ManifestId, SendSessionMessage, SessionRecord, TellSession, ThoughtKind, ThoughtTarget, WordsSent};

use crate::daemon::Fleet;

/// Where words sent to this Session went, named so a person can tell which: a Session that was sent to has a
/// row to point at, titled or not.
fn session_target(record: &SessionRecord) -> ThoughtTarget {
    let slot = record
        .attachments
        .iter()
        .filter(|held| held.state != ipc::AttachmentState::GivenBack && held.kind == "slot")
        // The ledger's own slot comes first, ahead of those its subagents leased.
        .find_map(|held| held.target.parse::<u32>().ok());
    ThoughtTarget {
        kind: ThoughtKind::Session,
        id: record.id.as_str().to_string(),
        title: record.title.as_deref().filter(|title| !title.trim().is_empty()).map_or_else(|| record.id.as_str().to_string(), str::to_string),
        manifest_id: record.manifest_id.clone(),
        slot,
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
    /// `tell_session`. **The send is Bridge's own**, so a terminal Job that is not listening is refused in the
    /// same words, and `operator.words_sent` is published once it is sent. **A refused send publishes nothing**.
    pub(crate) async fn session_told(self: Arc<Self>, tell: TellSession, manifest_id: Option<ManifestId>) -> Result<ThoughtTarget, Refusal> {
        let served = self.served_named(manifest_id.as_ref())?;
        let record = Arc::clone(&self)
            .send_session_message(SendSessionMessage { session_id: tell.session_id, text: tell.text, attachments: Vec::new(), mentions: Vec::new(), sketches: Vec::new() })
            .await?;
        let target = session_target(&record);
        self.events().publish(ipc::Event::WordsSent(WordsSent {
            manifest_id: ManifestId::carried(served.manifest().id().as_str()),
            to: target.kind,
            id: target.id.clone(),
            at: Instant::from(&self.now()),
        }));
        Ok(target)
    }
}
