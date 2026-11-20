//! Kit across his Macs: over the Mac link, each kind's switch on this Mac, each row against every linked Mac, and
//! the acts that switch one or settle a clash; and the retired shared home, whose routes answer that Kit crosses
//! over the link now, so Classic Bridge still reads them. Its own surface, for [`Accounts`](super::Accounts)'
//! reason: Kit is the Machine's, and a handler taking only this cannot reach for a Job. `docs/concepts/kit.md`.

use std::future::Future;

use ipc::{
    ChooseHomeSide, HomeKinds, HomeProfile, HomeRowAt, HomeSynced, KitLink, SetHomeItem, SetHomeKind, SetHomeRepository, SetKitLinkItem,
    SetKitLinkKind, SettleKitClash, SharedHomeRead,
};

use crate::daemon::Refusal;

pub trait SharedHome: Send + Sync + 'static {
    /// `get_shared_home` — every row of every kind this machine syncs for one profile, against the
    /// copy here of its home. **Reaches nothing**: no copy here is an answer with no home.
    fn get_shared_home(&self, profile: HomeProfile) -> impl Future<Output = Result<SharedHomeRead, Refusal>> + Send;

    /// `sync_shared_home` — make the home where it does not exist, clone or pull it, add every row
    /// only on this machine in one push, then read, naming each row left here.
    fn sync_shared_home(&self, profile: HomeProfile) -> impl Future<Output = Result<HomeSynced, Refusal>> + Send;

    /// `add_to_shared_home` — put a row only on this machine into the home and push it.
    fn add_to_shared_home(&self, at: HomeRowAt) -> impl Future<Output = Result<SharedHomeRead, Refusal>> + Send;

    /// `bring_from_shared_home` — put a row only in the home on this machine.
    fn bring_from_shared_home(&self, at: HomeRowAt) -> impl Future<Output = Result<SharedHomeRead, Refusal>> + Send;

    /// `choose_shared_home_side` — settle a row that differs with the side picked.
    fn choose_shared_home_side(
        &self,
        choose: ChooseHomeSide,
    ) -> impl Future<Output = Result<SharedHomeRead, Refusal>> + Send;

    /// `get_home_kinds` — what this machine syncs for one profile, every kind off until it is turned on.
    fn get_home_kinds(&self, profile: HomeProfile) -> impl Future<Output = Result<HomeKinds, Refusal>> + Send;

    /// `set_home_kind` — switch one kind.
    fn set_home_kind(&self, switch: SetHomeKind) -> impl Future<Output = Result<HomeKinds, Refusal>> + Send;

    /// `set_home_item` — switch one item on its own, whatever its kind's switch says.
    fn set_home_item(&self, switch: SetHomeItem) -> impl Future<Output = Result<HomeKinds, Refusal>> + Send;

    /// `set_home_repository` — where one profile's home is made, before it exists.
    fn set_home_repository(&self, choice: SetHomeRepository) -> impl Future<Output = Result<SharedHomeRead, Refusal>> + Send;

    /// `get_kit_link` — each kind's switch on this Mac, and every row it crosses against every linked Mac. Reads
    /// this Mac's Kit again, and reaches no other Mac.
    fn get_kit_link(&self) -> impl Future<Output = Result<KitLink, Refusal>> + Send;

    /// `set_kit_link_kind` — switch one kind on or off for crossing, on this Mac.
    fn set_kit_link_kind(&self, switch: SetKitLinkKind) -> impl Future<Output = Result<KitLink, Refusal>> + Send;

    /// `set_kit_link_item` — switch one item on or off by itself.
    fn set_kit_link_item(&self, switch: SetKitLinkItem) -> impl Future<Output = Result<KitLink, Refusal>> + Send;

    /// `settle_kit_clash` — keep one side of a clash with a linked Mac, on both Macs.
    fn settle_kit_clash(&self, settle: SettleKitClash) -> impl Future<Output = Result<KitLink, Refusal>> + Send;
}
