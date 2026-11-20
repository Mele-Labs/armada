//! Fleet's side of Kit across his Macs: the folders the composition root hands in, and the routes Bridge calls.
//! The shared home's routes answer for Classic Bridge with nothing to show and a refusal that names the link; Kit
//! over the link is `crate::kit_link`.

use std::sync::{Arc, OnceLock};

use adapter_traits::{AgentHarness, Delivery, HarnessHome, KeptFolder, Vcs, WorkProduct};
use api::{Refusal, SharedHome};
use ipc::{
    ChooseHomeSide, HomeKind, HomeKindSwitch, HomeKinds, HomeProfile, HomeRowAt, HomeSynced, KitLink, SetHomeItem, SetHomeKind,
    SetHomeRepository, SetKitLinkItem, SetKitLinkKind, SettleKitClash, SharedHomeRead, WireError,
};

use crate::daemon::Fleet;

/// Any act on the shared home, which Fleet no longer reads or writes.
const SHARED_HOME_RETIRED: &str = "fleet.shared_home_retired";
const RETIRED: &str = "Kit now crosses to your other Macs over the link.";

/// What the composition root hands Fleet for Kit's pieces on this Mac.
pub struct HomeFittings {
    /// The machine's mods folder.
    pub mods: Arc<dyn KeptFolder>,
    /// Kit's home.
    pub kit: Arc<dyn KeptFolder>,
    pub harness: Arc<dyn HarnessHome>,
}

/// The fittings, once handed in.
#[derive(Default)]
pub(crate) struct HomeHeld {
    fittings: OnceLock<Arc<HomeFittings>>,
    /// What Kit last told Bridge and the linked Macs.
    pub(crate) told: crate::kit_link::Told,
    /// Held while an exchange plans and puts, so two never put the same piece twice.
    pub(crate) exchanging: tokio::sync::Mutex<()>,
}

impl HomeHeld {
    pub(crate) fn fittings(&self) -> Option<Arc<HomeFittings>> {
        self.fittings.get().cloned()
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
    /// Hand Fleet the folders Kit's pieces are read from and put in. Only the first is kept.
    pub fn sharing_home_with(&self, fittings: HomeFittings) {
        let _ = self.home_held().fittings.set(Arc::new(fittings));
    }

    fn shared_home_retired(&self) -> Refusal {
        Refusal::IllegalMove(WireError::raised(SHARED_HOME_RETIRED, RETIRED, self.run_id()))
    }
}

/// Every kind off: the shared home syncs nothing now.
fn none_sync(login: String) -> HomeKinds {
    HomeKinds { login, kinds: HomeKind::ALL.iter().map(|kind| HomeKindSwitch { kind: *kind, on: false }).collect(), items: Vec::new() }
}

impl<H, V, W> SharedHome for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Nothing to show: no home is read, so Classic draws no rows.
    async fn get_shared_home(&self, _profile: HomeProfile) -> Result<SharedHomeRead, Refusal> {
        Ok(SharedHomeRead { home: None, planned: None, rows: Vec::new(), unreadable: Vec::new() })
    }

    async fn sync_shared_home(&self, _profile: HomeProfile) -> Result<HomeSynced, Refusal> {
        Err(self.shared_home_retired())
    }

    async fn add_to_shared_home(&self, _at: HomeRowAt) -> Result<SharedHomeRead, Refusal> {
        Err(self.shared_home_retired())
    }

    async fn bring_from_shared_home(&self, _at: HomeRowAt) -> Result<SharedHomeRead, Refusal> {
        Err(self.shared_home_retired())
    }

    async fn choose_shared_home_side(&self, _choose: ChooseHomeSide) -> Result<SharedHomeRead, Refusal> {
        Err(self.shared_home_retired())
    }

    async fn get_home_kinds(&self, profile: HomeProfile) -> Result<HomeKinds, Refusal> {
        Ok(none_sync(profile.login))
    }

    async fn set_home_kind(&self, _switch: SetHomeKind) -> Result<HomeKinds, Refusal> {
        Err(self.shared_home_retired())
    }

    async fn set_home_item(&self, _switch: SetHomeItem) -> Result<HomeKinds, Refusal> {
        Err(self.shared_home_retired())
    }

    async fn set_home_repository(&self, _choice: SetHomeRepository) -> Result<SharedHomeRead, Refusal> {
        Err(self.shared_home_retired())
    }

    async fn get_kit_link(&self) -> Result<KitLink, Refusal> {
        self.kit_link_read().await
    }

    async fn set_kit_link_kind(&self, switch: SetKitLinkKind) -> Result<KitLink, Refusal> {
        self.kit_link_kind_switched(switch).await
    }

    async fn set_kit_link_item(&self, switch: SetKitLinkItem) -> Result<KitLink, Refusal> {
        self.kit_link_item_switched(switch).await
    }

    async fn settle_kit_clash(&self, settle: SettleKitClash) -> Result<KitLink, Refusal> {
        self.kit_clash_settled(settle).await
    }
}
