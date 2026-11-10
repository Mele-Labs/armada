//! What Fleet does on the Operator's ruling, through the ways it already has: Fleet's own word
//! to a live Job (`told_by_fleet`), a note in a thread, a raised call, the person's Start press
//! and the close that parks a Job's slots. **The position of each role is read as a ruling comes
//! back** ([`Acting::position`]), so a call out when it is turned off acts on nothing.

use std::sync::{Arc, Weak};

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use config::settings::{self as keys, Key};
use ipc::operator_positions::{OperatorPosition, OperatorRepositories, OperatorRole};
use store::settings_file::SettingValue;

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
    fn position(&self, role: Option<OperatorRole>, manifest: &str) -> OperatorPosition {
        let Some(fleet) = self.fleet.upgrade() else { return OperatorPosition::ReadOnly };
        match role {
            Some(role) => fleet.operator_position(role, manifest),
            None if fleet.operator_on() => OperatorPosition::Decide,
            None => OperatorPosition::ReadOnly,
        }
    }

    fn suggest(&self, arrival: &Arrival, ruling: &Ruling) -> Acted<'_> {
        let (arrival, ruling) = (arrival.clone(), ruling.clone());
        self.doing(move |fleet| async move { fleet.operator_suggested(&arrival, &ruling).await })
    }

    /// A raise is its Log row.
    fn sourced(&self, arrival: &Arrival, ruling: &Ruling) -> Acted<'_> {
        if ruling.way == Way::Raise {
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

    /// How far `role` goes in the repository `manifest`, as settings.json reads now: Read-only under the
    /// Operator's own switch or a standing order, else what the repository sets, else the machine's.
    pub(crate) fn operator_position(&self, role: OperatorRole, manifest: &str) -> OperatorPosition {
        let settings = self.machine_settings().now();
        if !settings.get(keys::OPERATOR) || self.operator_seat().orders().role_off(role, manifest, &self.now()) {
            return OperatorPosition::ReadOnly;
        }
        // The Answer role is retired: a question or a card goes to the person, whatever the setting says.
        let key = match role {
            OperatorRole::Dispatch => keys::OPERATOR_DISPATCH,
            // Dispatch's old switch held Delegate too, so one saved On carries it across.
            OperatorRole::Delegate if settings.saved().get(keys::OPERATOR_DELEGATE.name()).is_none() && settings.saved().get(keys::OPERATOR_DISPATCH.name()) == Some(&SettingValue::Bool(true)) => keys::OPERATOR_DISPATCH,
            OperatorRole::Delegate => keys::OPERATOR_DELEGATE,
            OperatorRole::File => keys::OPERATOR_FILE,
            OperatorRole::Answer => return OperatorPosition::ReadOnly,
        };
        let repositories = settings.get(keys::OPERATOR_REPOSITORIES).map(|value| OperatorRepositories::read(&value.to_json_text())).unwrap_or_default();
        repositories.of(manifest, role).unwrap_or_else(|| OperatorPosition::read(&settings.get(key)))
    }

    pub(super) async fn title_of(&self, id: &str) -> String {
        let session = self.store().lock().await.session(id).ok().flatten();
        session.and_then(|one| one.title).unwrap_or_else(|| id.to_string())
    }
}
