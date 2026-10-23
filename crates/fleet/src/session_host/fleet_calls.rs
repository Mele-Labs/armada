//! A notice a Fleet mod's rule says to call: told to the Session as it always is, and raised on the
//! Job's Needs you list for the person. `docs/concepts/mods.md`, *fleet.toml*.
//!
//! **Derived from the store's call on every read**, so a restart keeps it; dismissing it keeps its id
//! among the dismissed, as for every item Fleet derives, and tells the agent nothing.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use ipc::{Instant, WaitingAct, WaitingActKind, WaitingItem, WaitingSource};

use crate::daemon::Fleet;

/// The item a call is on the Needs you list: the words the Session was told, and the Job to open.
pub(crate) fn call_item(call: &store::KeptCall) -> WaitingItem {
    WaitingItem {
        id: call.id.clone(),
        text: call.text.clone(),
        since: Instant::carried(&call.raised_at),
        source: WaitingSource::FleetCall,
        act: Some(WaitingAct { kind: WaitingActKind::Message, target: call.session_id.clone() }),
        options: Vec::new(),
        steps: Vec::new(),
        place: None,
        concern: None,
        cites: Vec::new(),
        confirmed: false,
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
    /// Raise `notice`, as the Session was told it in `text`, on the Session's Needs you list.
    pub(crate) async fn call_raised(&self, session: &str, notice: ipc::Notice, text: &str) {
        let at = self.now().as_str().to_string();
        let kept = self.store().lock().await.keep_call(session, notice.word(), text.trim(), &at);
        if kept.is_ok() {
            let _ = self.published_hosted(session).await;
        }
    }
}
