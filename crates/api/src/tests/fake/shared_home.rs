//! The shared home as the fake answers it: a machine with no copy of a home, so every row act is
//! refused by name, and What syncs with every kind off. What is read, compared and pushed is
//! `fleet::home`'s.

use ipc::{ChooseHomeSide, HomeKind, HomeKindSwitch, HomeKinds, HomeRowAt, SharedHomeRead};

use super::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{Refusal, SharedHome};

fn no_home() -> Refusal {
    Refusal::IllegalMove(ipc::WireError::raised("fleet.no_home_yet", "this machine holds no copy of the shared home", run_id()))
}

fn every_kind(on: Option<HomeKindSwitch>) -> HomeKinds {
    HomeKinds {
        kinds: HomeKind::ALL
            .iter()
            .map(|kind| HomeKindSwitch { kind: *kind, on: on.is_some_and(|switch| switch.kind == *kind && switch.on) })
            .collect(),
    }
}

impl SharedHome for FakeDaemon {
    async fn get_shared_home(&self) -> Result<SharedHomeRead, Refusal> {
        Ok(SharedHomeRead { home: None, rows: Vec::new(), unreadable: Vec::new() })
    }

    async fn sync_shared_home(&self) -> Result<SharedHomeRead, Refusal> {
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

    async fn get_home_kinds(&self) -> Result<HomeKinds, Refusal> {
        Ok(every_kind(None))
    }

    async fn set_home_kind(&self, switch: HomeKindSwitch) -> Result<HomeKinds, Refusal> {
        Ok(every_kind(Some(switch)))
    }
}
