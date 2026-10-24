//! Mods: folders on this machine that change how Bridge looks. Its own surface,
//! for [`Sessions`](super::Sessions)' reason: it is whole on its own, and a
//! handler that takes only this cannot reach for a Job. `docs/concepts/mods.md`.

use std::future::Future;
use std::sync::Arc;

use ipc::{
    ModChecked, ModList, ModPromoted, ModScaffolded, ModSessionStarted, ModSummary, PromoteMod,
    ScaffoldMod, SetModEnabled, StartModSession, TrackerTry, TryTrackerMod,
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

    /// `set_mod_enabled` — this machine's switch. [`Refusal::NotFound`] (a 404)
    /// for a name no folder has.
    fn set_mod_enabled(
        &self,
        set: SetModEnabled,
    ) -> impl Future<Output = Result<ModSummary, Refusal>> + Send;

    /// `validate_mod` — every problem found. A mod that does not read is
    /// `valid` false, not a refusal; only a name that is not a slug or names no
    /// folder is refused.
    fn validate_mod(&self, name: String) -> impl Future<Output = Result<ModChecked, Refusal>> + Send;

    /// `try_tracker_mod` — a tracker mod's Open query run, and one issue it listed read, against
    /// a repository Fleet serves, and the outcome kept beside the mod. A try that fails is an
    /// answer with `passed` false; [`Refusal::Unacceptable`] for a mod that is not a tracker.
    fn try_tracker_mod(&self, asked: TryTrackerMod) -> impl Future<Output = Result<TrackerTry, Refusal>> + Send;

    /// `promote_mod` — a valid mod put into the repository by a Job of its own,
    /// which pushes its branch, opens its pull request and is told to see it merged.
    /// [`Refusal::IllegalMove`] where it cannot be: invalid, built in, no
    /// `packages/`, no free slot.
    fn promote_mod(
        self: Arc<Self>,
        promote: PromoteMod,
    ) -> impl Future<Output = Result<ModPromoted, Refusal>> + Send;

    /// `start_mod_session` — a Mod Job on Armada's own repository, sent the brief and the note as
    /// its first message. [`Refusal::Unacceptable`] where this Fleet does not serve that repository.
    fn start_mod_session(
        self: Arc<Self>,
        start: StartModSession,
    ) -> impl Future<Output = Result<ModSessionStarted, Refusal>> + Send;
}
