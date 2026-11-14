//! What Fleet does on the Operator's ruling, through the ways it already has: Fleet's own word
//! to a live Job (`told_by_fleet`), a note in a thread, a raised call, the person's Start press
//! and the close that parks a Job's slots. **An act asks first** ([`Acting::suggest_once`]), and is done
//! only on the person's go-ahead or a rule he kept.

use std::sync::{Arc, Weak};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use config::settings as keys;

use super::{carried, Acted, Acting, Arrival, Ruling, Way};
use crate::daemon::Fleet;

/// Fleet as the Operator's rulings reach it.
pub struct FleetActing<H, V, W> {
    fleet: Weak<Fleet<H, V, W>>,
}

impl<H, V, W> FleetActing<H, V, W> {
    pub fn on(fleet: &Arc<Fleet<H, V, W>>) -> FleetActing<H, V, W> {
        FleetActing { fleet: Arc::downgrade(fleet) }
    }
}

impl<H, V, W> FleetActing<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Run `act` on Fleet, saying how it came out. Where Fleet is gone, nothing was done.
    fn doing<F, Fut>(&self, act: F) -> Acted<'static>
    where
        F: FnOnce(Arc<Fleet<H, V, W>>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), String>> + Send + 'static,
    {
        let fleet = self.fleet.upgrade();
        Box::pin(async move {
            match fleet {
                Some(fleet) => act(fleet).await,
                None => Err("Fleet stopped".to_string()),
            }
        })
    }
}

impl<H, V, W> Acting for FleetActing<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    fn on(&self) -> bool {
        self.fleet.upgrade().is_some_and(|fleet| fleet.operator_on())
    }

    fn suggest(&self, arrival: &Arrival, ruling: &Ruling) -> Acted<'_> {
        let (arrival, ruling) = (arrival.clone(), ruling.clone());
        self.doing(move |fleet| async move { fleet.operator_suggested(&arrival, &ruling).await.map(|_| ()) })
    }

    fn suggest_once(&self, arrival: &Arrival, ruling: &Ruling) -> super::Done<'_, Result<Option<String>, String>> {
        let (arrival, ruling) = (arrival.clone(), ruling.clone());
        let fleet = self.fleet.upgrade();
        Box::pin(async move {
            match fleet {
                Some(fleet) => fleet.operator_suggested(&arrival, &ruling).await,
                None => Err("Fleet stopped".to_string()),
            }
        })
    }

    /// A raise is its Log row, but for a dispatch's, which is asked of him.
    fn sourced(&self, arrival: &Arrival, ruling: &Ruling) -> Acted<'_> {
        if ruling.way == Way::Raise && !arrival.is_dispatch() && !arrival.is_job_ask() {
            return carried();
        }
        let (arrival, ruling) = (arrival.clone(), ruling.clone());
        self.doing(move |fleet| async move { fleet.operator_sourced(&arrival, &ruling).await.map(|_| ()) })
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
    /// The Operator's own switch, as settings.json reads now. Off is Fleet as it was before it.
    pub(crate) fn operator_on(&self) -> bool {
        self.machine_settings().now().get(keys::OPERATOR)
    }

    /// The week's plan against the share where the Operator pauses, as the setting and Autopilot's weekly
    /// reading stand now. **No reading is no pause**, and so is a week below the share.
    pub(crate) async fn operator_paused(&self) -> Option<super::Paused> {
        let at = u32::try_from(self.machine_settings().now().get(keys::OPERATOR_PLAN_PAUSE).clamp(0, 100)).unwrap_or(100);
        let week = self.weekly_reading().await?;
        (week >= at).then_some(super::Paused { week, at })
    }

    pub(super) async fn title_of(&self, id: &str) -> String {
        let session = self.store().lock().await.session(id).ok().flatten();
        session.and_then(|one| one.title).unwrap_or_else(|| id.to_string())
    }
}
