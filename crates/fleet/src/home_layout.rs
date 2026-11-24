//! The Cockpit home's layout: one per user, kept as `bridge.homeLayout` in settings.json, so a save publishes
//! `settings.changed` and a hand edit is checked as any other key is. `docs/practices/protocol/projects.md`,
//! *The Cockpit home's layout*.
//!
//! **It follows him across his Macs through the Gateways, the latest save winning.** A save is stamped and tells the
//! linked Macs with `home_layout.changed`; each reads `share_home_layout` and hands it to its Fleet, which takes it
//! only where its stamp is the later. A Mac that was asleep reads it again when its link opens, so it catches up. A
//! layout taken is not told on, so two Macs do not echo it. The stamp is a wall-clock reading, so two Macs whose clocks
//! differ by more than the time between his saves can take the earlier save for the later.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use config::settings::{self as keys, Key};
use ipc::{Event, HomeLayout, HomeLayoutChanged, Instant, MacNamed, OtherMacHomeLayout, SaveHomeLayout, SaveSettings};

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
    /// The layout settings.json holds and when its save was made, **none where it holds none** or holds what is not a layout.
    pub(crate) fn home_layout_in_force(&self) -> HomeLayout {
        match self.machine_settings().now().get(keys::HOME_LAYOUT) {
            Some(value) => ipc::home_layout::read(&value.to_json_text()),
            None => HomeLayout::default(),
        }
    }

    fn unacceptable_home_layout(&self, problem: &str) -> Refusal {
        Refusal::Unacceptable(ipc::WireError::raised(
            UNACCEPTABLE_HOME_LAYOUT,
            format!("the home's layout could not be kept: {problem}"),
            self.run_id(),
        ))
    }

    /// `home` kept as it is, stamp and all, or the key removed where there is nothing to keep.
    async fn home_layout_kept(&self, home: &HomeLayout) -> Result<(), Refusal> {
        let value = ipc::home_layout::text_of(home).and_then(|text| json_change(&text));
        let change = SaveSettings { changes: [(keys::HOME_LAYOUT.name().to_string(), value)].into_iter().collect() };
        self.save_settings_now(change).await.map(drop)
    }

    /// `save_home_layout`: kept whole and stamped, or forgotten where absent, and the linked Macs told. A forgetting
    /// keeps its stamp, so it beats an earlier layout on another Mac. Nothing else of the file changes.
    pub(crate) async fn home_layout_saved(&self, save: SaveHomeLayout) -> Result<HomeLayout, Refusal> {
        if let Some(first) = save.layout.as_ref().and_then(|layout| ipc::home_layout::layout_problems(layout).into_iter().next()) {
            return Err(self.unacceptable_home_layout(&first));
        }
        let home = HomeLayout { layout: save.layout, saved_at: Some(Instant::from(&self.now())) };
        self.home_layout_kept(&home).await?;
        self.publish(Event::HomeLayoutChanged(HomeLayoutChanged {}));
        Ok(self.home_layout_in_force())
    }

    /// `share_home_layout`: what this Mac keeps, for a linked Mac to take where it is the later.
    pub(crate) fn home_layout_shared(&self) -> HomeLayout {
        self.home_layout_in_force()
    }

    /// `hold_other_mac_home_layout`: Mac `held.mac`'s layout, taken where its save is later than the one here. **Kept
    /// with its stamp and not told on**, so the two Macs settle on one layout and stop.
    pub(crate) async fn home_layout_held(&self, held: OtherMacHomeLayout) -> Result<MacNamed, Refusal> {
        let mac = self.mac_named(&held.mac)?;
        if let Some(first) = held.home.layout.as_ref().and_then(|layout| ipc::home_layout::layout_problems(layout).into_iter().next()) {
            return Err(self.unacceptable_home_layout(&first));
        }
        if held.home.supersedes(&self.home_layout_in_force()) {
            self.home_layout_kept(&held.home).await?;
        }
        Ok(MacNamed { mac })
    }
}
