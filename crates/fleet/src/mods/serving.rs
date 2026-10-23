//! Fleet's side of the mod operations: what the six answer, and the loop that
//! says the folder changed.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Mods, Refusal};
use ipc::{
    Event, Instant, ModChecked, ModKind, ModList, ModPromoted, ModScaffolded, ModSummary, PromoteMod, ScaffoldMod,
    SetModEnabled, TrackerTry, TryTrackerMod, WireError,
};
use store::ReadError;
use tokio::task::JoinHandle;

use super::{
    built_in, examine, examine_built_in, fleet_rules, scaffold, scan, slug_problem, tracker, words, BuiltIn, Payload,
};
use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// A name that is not a slug, a description that cannot be kept, or a mod that exists. A 422.
const UNACCEPTABLE_MOD: &str = "fleet.unacceptable_mod";
/// A name no folder has. A 404.
const NO_SUCH_MOD: &str = "fleet.no_such_mod";
/// A mod that cannot be promoted: invalid, built in, no `packages/` to put it in,
/// no free slot, or already there exactly. A 409.
pub(super) const MOD_NOT_PROMOTABLE: &str = "fleet.mod_not_promotable";
/// Git or the disk would not scaffold or promote. A 500: a scaffold leaves nothing behind, and a
/// promote leaves its Job holding the tree.
pub(super) const MOD_NOT_WRITTEN: &str = "fleet.mod_not_written";

/// Which mod a name names.
pub(super) enum Named {
    /// A folder in the machine's `mods/`.
    Local,
    BuiltIn(BuiltIn),
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
    pub(crate) fn mods_dir(&self) -> PathBuf {
        PathBuf::from(&self.host().mods_dir)
    }

    pub(super) fn mod_fault(&self, said: String) -> Refusal {
        Refusal::Fault(WireError::raised(MOD_NOT_WRITTEN, said, self.run_id()))
    }

    /// This machine's switches, by mod name.
    pub(crate) async fn mod_switches_read(&self) -> Result<std::collections::BTreeMap<String, bool>, Refusal> {
        self.store()
            .lock()
            .await
            .mod_switches()
            .map_err(|fault| self.refusal(Adrift::Reading(ReadError::Database(fault))))
    }

    /// The folder as it stands, read off the disk now.
    async fn mods_listed(&self) -> Result<ModList, Refusal> {
        let switches = self.mod_switches_read().await?;
        self.host().mod_switches.replace(&switches);
        let (dir, built_in) = (self.mods_dir(), self.host().built_in_mods.clone());
        tokio::task::spawn_blocking(move || scan(&dir, &switches, &built_in))
            .await
            .map_err(|why| self.mod_fault(format!("the mods folder could not be read: {why}")))
    }

    /// Say the list now, because something here just changed it. The rescan
    /// would find it in a moment; this is for the Bridge that is waiting.
    pub(crate) async fn mods_announced(&self) -> Result<ModList, Refusal> {
        let list = self.mods_listed().await?;
        self.mods().published(&list);
        self.publish(Event::ModsChanged(list.clone()));
        Ok(list)
    }

    /// `name` as a built-in mod's id, or a slug naming a folder that is there, or the refusal.
    pub(super) fn mod_named(&self, name: &str) -> Result<Named, Refusal> {
        let none = || {
            Refusal::NotFound(
                WireError::raised(NO_SUCH_MOD, format!("there is no mod called `{name}`"), self.run_id())
                    .with_field("name", ipc::WireValue::Str(name.to_string())),
            )
        };
        if name.starts_with(built_in::PREFIX) {
            return self.host().built_in_mods.named(name).cloned().map(Named::BuiltIn).ok_or_else(none);
        }
        if let Some(why) = slug_problem(name) {
            return Err(Refusal::Unacceptable(
                WireError::raised(UNACCEPTABLE_MOD, why, self.run_id())
                    .with_field("name", ipc::WireValue::Str(name.to_string())),
            ));
        }
        match std::fs::symlink_metadata(self.mods_dir().join(name)) {
            Ok(meta) if meta.is_dir() => Ok(Named::Local),
            _ => Err(none()),
        }
    }
}

impl<H, V, W> Mods for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn list_mods(&self) -> Result<ModList, Refusal> {
        self.mods_listed().await
    }

    async fn scaffold_mod(&self, asked: ScaffoldMod) -> Result<ModScaffolded, Refusal> {
        let dir = self.mods_dir();
        let name = asked.name.clone();
        let made = tokio::task::spawn_blocking(move || {
            scaffold::make(&dir, &name, asked.description.as_deref(), asked.kind.unwrap_or(ModKind::Theme))
        })
        .await
        .map_err(|why| self.mod_fault(format!("the mod could not be made: {why}")))?;
        let path = made.map_err(|refused| match refused {
            scaffold::Refused::Unacceptable(why) => Refusal::Unacceptable(
                WireError::raised(UNACCEPTABLE_MOD, why, self.run_id())
                    .with_field("name", ipc::WireValue::Str(asked.name.clone())),
            ),
            scaffold::Refused::Failed(why) => self.mod_fault(why),
        })?;
        self.mods_announced().await?;
        Ok(ModScaffolded {
            name: asked.name,
            path: path.to_string_lossy().into_owned(),
        })
    }

    async fn set_mod_enabled(&self, set: SetModEnabled) -> Result<ModSummary, Refusal> {
        self.mod_named(&set.name)?;
        self.store()
            .lock()
            .await
            .switch_mod(&set.name, set.enabled)
            .map_err(|why| self.refusal(Adrift::Writing(why)))?;
        let list = self.mods_announced().await?;
        list.mods
            .into_iter()
            .find(|one| one.name == set.name)
            .ok_or_else(|| self.mod_fault(format!("`{}` went away while it was switched", set.name)))
    }

    async fn validate_mod(&self, name: String) -> Result<ModChecked, Refusal> {
        let named = self.mod_named(&name)?;
        let dir = self.mods_dir();
        let asked = name.clone();
        let found = tokio::task::spawn_blocking(move || match named {
            Named::Local => examine(&dir, &asked),
            Named::BuiltIn(one) => examine_built_in(&one),
        })
        .await
        .map_err(|why| self.mod_fault(format!("the mod could not be read: {why}")))?;
        let valid = found.valid();
        let (mut css, mut layout, mut taught_words, mut taught_fleet) = (None, None, None, None);
        match found.files.map(|files| files.payload) {
            // A tweak is token values like a theme's, and Bridge injects it the same way.
            Some(Payload::Css(text) | Payload::Tweak(text)) => css = Some(text),
            Some(Payload::Layout(text)) => layout = Some(text),
            // A taught mod hands back what it holds, typed, for Bridge to show. The file passed its
            // checks, so reading it again cannot fail. Words are linked through `list_word_rules`
            // and Fleet's own are acted on by Fleet.
            Some(Payload::Words(text)) => taught_words = words::read(&text).ok(),
            Some(Payload::Fleet(text)) => taught_fleet = fleet_rules::read(&text).ok(),
            // A tracker is not Bridge's to apply; what it is checked against is `try_tracker_mod`.
            Some(Payload::Tracker(_)) | None => {}
        }
        Ok(ModChecked {
            name,
            valid,
            css,
            layout,
            words: taught_words,
            fleet: taught_fleet,
            problems: found.problems,
        })
    }

    async fn try_tracker_mod(&self, asked: TryTrackerMod) -> Result<TrackerTry, Refusal> {
        if let Named::BuiltIn(_) = self.mod_named(&asked.name)? {
            let why = format!("`{}` is built in, and only a mod in this machine's mods folder has a place to keep a try", asked.name);
            return Err(Refusal::Unacceptable(
                WireError::raised(UNACCEPTABLE_MOD, why, self.run_id()).with_field("name", ipc::WireValue::Str(asked.name.clone())),
            ));
        }
        // No repository named: one whose Manifest names this mod as its tracker, since a try against a repository that uses
        // another tracker proves nothing; the first served only where none does.
        let using = || self.repositories().served().into_iter().find(|one| one.manifest().tracker().and_then(|block| block.provider()) == Some(asked.name.as_str()));
        let served = match (asked.manifest_id.as_ref(), using()) {
            (None, Some(using)) => using,
            (named, _) => self.served_named(named)?,
        };
        let (dir, root) = (self.mods_dir(), served.root().to_string());
        let at = Instant::carried(self.now().as_str());
        let manifest_id = ipc::ManifestId::carried(served.manifest().id().as_str());
        let name = asked.name.clone();
        let tried = tokio::task::spawn_blocking(move || tracker::try_it(&dir, &name, &root, manifest_id, at))
            .await
            .map_err(|why| self.mod_fault(format!("the try did not finish: {why}")))?;
        let tried = tried.map_err(|not| match not {
            tracker::NotTried::NotATracker(why) => Refusal::Unacceptable(
                WireError::raised(UNACCEPTABLE_MOD, why, self.run_id())
                    .with_field("name", ipc::WireValue::Str(asked.name.clone())),
            ),
            tracker::NotTried::Failed(why) => self.mod_fault(why),
        })?;
        self.mods_announced().await?;
        Ok(tried)
    }

    async fn promote_mod(self: Arc<Self>, promote: PromoteMod) -> Result<ModPromoted, Refusal> {
        self.mod_promoted(promote).await
    }
}

/// Rescan the folder every `every` and publish `mods.changed` when the list
/// moved. **At most one event a scan and only on a change**, so a quiet folder
/// publishes nothing and the shared drop-oldest backlog is still the only queue.
pub fn keep_reading<H, V, W>(fleet: Arc<Fleet<H, V, W>>, every: Duration) -> JoinHandle<()>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(every);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            // A scan that fails is tried again at the next tick. Nothing here
            // may take the daemon down, so there is nothing to say to anyone.
            if let Ok(list) = fleet.mods_listed().await {
                if fleet.mods().moved(&list) {
                    fleet.publish(Event::ModsChanged(list));
                }
            }
        }
    })
}
