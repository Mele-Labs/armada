//! The shared home as the fake answers it: a machine with no copy of a home, so every row act is
//! refused by name, and What syncs with every kind off. What is read, compared and pushed is
//! `fleet::home`'s.

use ipc::{ChooseHomeSide, HomeKind, HomeKindSwitch, HomeKinds, HomePlanned, HomeProfile, HomeRowAt, HomeSynced, SetHomeItem, SetHomeKind, SetHomeRepository, SharedHomeRead};

use super::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{Refusal, SharedHome};

fn no_home() -> Refusal {
    Refusal::IllegalMove(ipc::WireError::raised("fleet.no_home_yet", "this machine holds no copy of the shared home", run_id()))
}

fn every_kind(login: String, on: Option<(HomeKind, bool)>) -> HomeKinds {
    HomeKinds {
        login,
        kinds: HomeKind::ALL
            .iter()
            .map(|kind| HomeKindSwitch { kind: *kind, on: on.is_some_and(|(switched, on)| switched == *kind && on) })
            .collect(),
        items: Vec::new(),
    }
}

impl SharedHome for FakeDaemon {
    async fn get_shared_home(&self, _profile: HomeProfile) -> Result<SharedHomeRead, Refusal> {
        Ok(SharedHomeRead { home: None, planned: None, rows: Vec::new(), unreadable: Vec::new() })
    }

    async fn sync_shared_home(&self, _profile: HomeProfile) -> Result<HomeSynced, Refusal> {
        Err(no_home())
    }

    async fn add_to_shared_home(&self, _at: HomeRowAt) -> Result<SharedHomeRead, Refusal> {
        Err(no_home())
    }

    async fn bring_from_shared_home(&self, _at: HomeRowAt) -> Result<SharedHomeRead, Refusal> {
        Err(no_home())
    }

    async fn choose_shared_home_side(&self, _choose: ChooseHomeSide) -> Result<SharedHomeRead, Refusal> {
        Err(no_home())
    }

    async fn get_home_kinds(&self, profile: HomeProfile) -> Result<HomeKinds, Refusal> {
        Ok(every_kind(profile.login, None))
    }

    async fn set_home_kind(&self, switch: SetHomeKind) -> Result<HomeKinds, Refusal> {
        Ok(every_kind(switch.login, Some((switch.kind, switch.on))))
    }

    async fn set_home_item(&self, switch: SetHomeItem) -> Result<HomeKinds, Refusal> {
        let mut kinds = every_kind(switch.login, None);
        if switch.on {
            kinds.items.push(switch.item);
        }
        Ok(kinds)
    }

    async fn set_home_repository(&self, choice: SetHomeRepository) -> Result<SharedHomeRead, Refusal> {
        let planned = HomePlanned { account: choice.owner, repository: choice.name, exists: false };
        Ok(SharedHomeRead { home: None, planned: Some(planned), rows: Vec::new(), unreadable: Vec::new() })
    }
}
