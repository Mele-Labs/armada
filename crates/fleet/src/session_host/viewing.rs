//! When the person was last in a Job: `view_session`, kept as `viewed_at` and read by the row mark.
//! `crates/ipc/operations/view_session.toml`.

use std::collections::HashMap;
use std::sync::Mutex;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{SessionRecord, ViewSession, WireError};

use super::NO_SUCH_SESSION;
use crate::daemon::Fleet;

/// The places that have each Job on screen now, by Job: an `on` counts one, an `off` lets one go.
/// Memory only, so a Fleet restart forgets it and keeps `viewed_at`.
#[derive(Default)]
pub(crate) struct OnScreen(Mutex<HashMap<String, u32>>);

impl OnScreen {
    fn moved(&self, session: &str, on: bool) {
        let mut held = self.0.lock().expect("held across no panic");
        let count = held.get(session).copied().unwrap_or(0);
        match (on, count) {
            (true, _) => {
                held.insert(session.to_string(), count + 1);
            }
            (false, 0 | 1) => {
                held.remove(session);
            }
            (false, _) => {
                held.insert(session.to_string(), count - 1);
            }
        }
    }

    /// Some place has the Job on screen now.
    pub(crate) fn now(&self, session: &str) -> bool {
        self.0.lock().expect("held across no panic").contains_key(session)
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
    /// `view_session`: Fleet's clock kept as the instant he was last in the Job, the count of places
    /// showing it moved, and the row published. Any Job Fleet knows; 404 for one it does not.
    pub(crate) async fn session_viewed(&self, asked: ViewSession) -> Result<SessionRecord, Refusal> {
        let id = asked.session_id.as_str();
        {
            let mut store = self.store().lock().await;
            if store.session(id).map_err(|why| self.ledger_fault(why))?.is_none() {
                return Err(Refusal::NotFound(WireError::raised(NO_SUCH_SESSION, format!("no job is named {id}"), self.run_id())));
            }
            store.keep_session_viewed(id, self.now().as_str()).map_err(|why| self.ledger_fault(why))?;
        }
        self.hosts().on_screen.moved(id, asked.on);
        self.published_hosted(id).await
    }
}
