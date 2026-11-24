//! The Cockpit home's layout: one per user, kept as `bridge.homeLayout` in settings.json, so a save publishes
//! `settings.changed` and a hand edit is checked as any other key is. `docs/practices/protocol/projects.md`,
//! *The Cockpit home's layout*.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::settings::{self as keys, Key};
use ipc::{HomeLayout, SaveHomeLayout, SaveSettings};

use crate::daemon::Fleet;
use crate::preferences::json_change;

/// A `save_home_layout` carrying a block with a blank kind or view. A 422.
const UNACCEPTABLE_HOME_LAYOUT: &str = "fleet.unacceptable_home_layout";

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
    /// The layout settings.json holds, **none where it holds none** or holds what is not a layout.
    pub(crate) fn home_layout_in_force(&self) -> HomeLayout {
        let held = self.machine_settings().now().get(keys::HOME_LAYOUT);
        HomeLayout { layout: held.and_then(|value| ipc::home_layout::read(&value.to_json_text())) }
    }

    /// `save_home_layout`: kept whole, or forgotten where absent. Nothing else of the file changes.
    pub(crate) async fn home_layout_saved(&self, save: SaveHomeLayout) -> Result<HomeLayout, Refusal> {
        let value = match &save.layout {
            Some(layout) => {
                if let Some(first) = ipc::home_layout::layout_problems(layout).first() {
                    return Err(Refusal::Unacceptable(ipc::WireError::raised(
                        UNACCEPTABLE_HOME_LAYOUT,
                        format!("the home's layout could not be kept: {first}"),
                        self.run_id(),
                    )));
                }
                json_change(&ipc::home_layout::text(layout))
            }
            None => None,
        };
        let change = SaveSettings { changes: [(keys::HOME_LAYOUT.name().to_string(), value)].into_iter().collect() };
        self.save_settings_now(change).await?;
        Ok(self.home_layout_in_force())
    }
}
