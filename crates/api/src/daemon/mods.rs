//! Mods: folders on this machine that change how Bridge looks. Its own surface,
//! for [`Sessions`](super::Sessions)' reason: it is whole on its own, and a
//! handler that takes only this cannot reach for a Job. `docs/concepts/mods.md`.

use std::future::Future;

use ipc::{
    ModChecked, ModList, ModPromoted, ModScaffolded, ModSummary, PromoteMod, ScaffoldMod,
    SetModEnabled,
};

use crate::daemon::Refusal;

pub trait Mods: Send + Sync + 'static {
    /// `list_mods` — every mod, a broken one as a row with `valid` false. Never
    /// refuses for what is in the folder.
    fn list_mods(&self) -> impl Future<Output = Result<ModList, Refusal>> + Send;

    /// `scaffold_mod` — a folder with a starter theme and one commit.
    /// [`Refusal::Unacceptable`] for a name that is not a slug or a mod that exists.
    fn scaffold_mod(
        &self,
        scaffold: ScaffoldMod,
    ) -> impl Future<Output = Result<ModScaffolded, Refusal>> + Send;

    /// `set_mod_enabled` — this machine's switch. [`Refusal::NoSuchJob`] (a 404)
    /// for a name no folder has.
    fn set_mod_enabled(
        &self,
        set: SetModEnabled,
    ) -> impl Future<Output = Result<ModSummary, Refusal>> + Send;

    /// `validate_mod` — every problem found. A mod that does not read is
    /// `valid` false, not a refusal; only a name that is not a slug or names no
    /// folder is refused.
    fn validate_mod(&self, name: String) -> impl Future<Output = Result<ModChecked, Refusal>> + Send;

    /// `promote_mod` — a valid mod copied onto a new branch, pushed nowhere.
    /// [`Refusal::IllegalMove`] where it cannot be: invalid, no `packages/`, no
    /// free slot.
    fn promote_mod(
        &self,
        promote: PromoteMod,
    ) -> impl Future<Output = Result<ModPromoted, Refusal>> + Send;
}
