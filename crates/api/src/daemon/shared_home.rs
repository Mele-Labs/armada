//! The shared home: Kit's rows against it, the acts that settle one, and what this machine
//! syncs. Its own surface, for [`Accounts`](super::Accounts)' reason: the home is the Machine's,
//! and a handler taking only this cannot reach for a Job. `docs/concepts/kit.md`.

use std::future::Future;

use ipc::{ChooseHomeSide, HomeKindSwitch, HomeKinds, HomeRowAt, SharedHomeRead};

use crate::daemon::Refusal;

pub trait SharedHome: Send + Sync + 'static {
    /// `get_shared_home` — every row of every kind this machine syncs, against the copy here.
    /// **Reaches nothing**: no copy here is an answer with no home.
    fn get_shared_home(&self) -> impl Future<Output = Result<SharedHomeRead, Refusal>> + Send;

    /// `sync_shared_home` — make the home where it does not exist, clone or pull it, then read.
    fn sync_shared_home(&self) -> impl Future<Output = Result<SharedHomeRead, Refusal>> + Send;

    /// `add_to_shared_home` — put a row only on this machine into the home and push it.
    fn add_to_shared_home(&self, at: HomeRowAt) -> impl Future<Output = Result<SharedHomeRead, Refusal>> + Send;

    /// `bring_from_shared_home` — put a row only in the home on this machine.
    fn bring_from_shared_home(&self, at: HomeRowAt) -> impl Future<Output = Result<SharedHomeRead, Refusal>> + Send;

    /// `choose_shared_home_side` — settle a row that differs with the side picked.
    fn choose_shared_home_side(
        &self,
        choose: ChooseHomeSide,
    ) -> impl Future<Output = Result<SharedHomeRead, Refusal>> + Send;

    /// `get_home_kinds` — what this machine syncs, every kind off until it is turned on.
    fn get_home_kinds(&self) -> impl Future<Output = Result<HomeKinds, Refusal>> + Send;

    /// `set_home_kind` — switch one kind.
    fn set_home_kind(&self, switch: HomeKindSwitch) -> impl Future<Output = Result<HomeKinds, Refusal>> + Send;
}
