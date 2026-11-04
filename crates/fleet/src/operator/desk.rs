//! The Operator's [`Desk`] on a running Fleet: its clock, its settings and its store.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use config::settings as keys;
use core_model::Timestamp;

use super::{Desk, Done, LiveJob, Logged};
use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// Fleet as the Operator reaches it. A store failure is said through `adrift`, and a count of
/// calls that cannot be read stands for the ceiling reached.
pub struct FleetDesk<H, V, W> {
    fleet: Arc<Fleet<H, V, W>>,
    adrift: Box<dyn Fn(Adrift) + Send + Sync>,
}

impl<H, V, W> FleetDesk<H, V, W> {
    pub fn on(fleet: Arc<Fleet<H, V, W>>, adrift: impl Fn(Adrift) + Send + Sync + 'static) -> FleetDesk<H, V, W> {
        FleetDesk { fleet, adrift: Box::new(adrift) }
    }
}

impl<H, V, W> Desk for FleetDesk<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    fn now(&self) -> Timestamp {
        self.fleet.now()
    }

    fn daily_calls(&self) -> u32 {
        let calls = self.fleet.machine_settings().now().get(keys::OPERATOR_DAILY_CALLS);
        u32::try_from(calls.max(0)).unwrap_or(u32::MAX)
    }

    /// Every hosted Job not ended, Helm's aside. **Titles only for now**: the plan paths, files
    /// written and dispatch words a call reads come with *Who does the work*.
    fn live(&self) -> Done<'_, Vec<LiveJob>> {
        Box::pin(async move {
            let store = self.fleet.store().lock().await;
            let hostings = match store.hostings() {
                Ok(hostings) => hostings,
                Err(why) => {
                    (self.adrift)(Adrift::Writing(why));
                    return Vec::new();
                }
            };
            let mut live = Vec::new();
            for hosting in hostings.into_iter().filter(|one| !one.helm) {
                let Ok(Some(session)) = store.session(&hosting.session_id) else { continue };
                if session.state == store::SessionState::Ended {
                    continue;
                }
                live.push(LiveJob {
                    title: session.title.unwrap_or_else(|| session.id.clone()),
                    session_id: session.id,
                    manifest_id: hosting.manifest_id,
                    words: String::new(),
                    paths: Vec::new(),
                });
            }
            live
        })
    }

    fn calls_since(&self, since: Timestamp) -> Done<'_, u32> {
        Box::pin(async move {
            match self.fleet.store().lock().await.operator_calls_since(since.as_str()) {
                Ok(calls) => calls,
                Err(why) => {
                    (self.adrift)(Adrift::Writing(why));
                    u32::MAX
                }
            }
        })
    }

    fn keep(&self, row: Logged) -> Done<'_, ()> {
        Box::pin(async move {
            if let Err(why) = self.fleet.store().lock().await.add_operator_ruling(&row.kept()) {
                (self.adrift)(Adrift::Writing(why));
            }
        })
    }
}
