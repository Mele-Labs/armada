//! Fleet's side of the mod operations: what the five answer, and the loop that
//! says the folder changed.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Mods, Refusal};
use ipc::{
    Event, ModChecked, ModList, ModPromoted, ModScaffolded, ModSummary, PromoteMod, ScaffoldMod,
    SetModEnabled, WireError,
};
use store::LoadJobError;
use tokio::task::JoinHandle;

use super::{examine, scaffold, scan, slug_problem};
use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// A name that is not a slug, a description that cannot be kept, or a mod that exists. A 422.
const UNACCEPTABLE_MOD: &str = "fleet.unacceptable_mod";
/// A name no folder has. A 404.
const NO_SUCH_MOD: &str = "fleet.no_such_mod";
/// A mod that cannot go onto a branch: invalid, no `packages/` to put it in, no
/// free slot, or already there exactly. A 409.
pub(super) const MOD_NOT_PROMOTABLE: &str = "fleet.mod_not_promotable";
/// Git or the disk would not scaffold or promote. A 500, with nothing left behind.
pub(super) const MOD_NOT_WRITTEN: &str = "fleet.mod_not_written";

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
    pub(super) fn mods_dir(&self) -> PathBuf {
        PathBuf::from(&self.host().mods_dir)
    }

    pub(super) fn mod_fault(&self, said: String) -> Refusal {
        Refusal::Fault(WireError::raised(MOD_NOT_WRITTEN, said, self.run_id()))
    }

    /// The folder as it stands, read off the disk now.
    async fn mods_listed(&self) -> Result<ModList, Refusal> {
        let switches = self
            .store()
            .lock()
            .await
            .mod_switches()
            .map_err(|fault| self.refusal(Adrift::Reading(LoadJobError::Database(fault))))?;
        let dir = self.mods_dir();
        tokio::task::spawn_blocking(move || scan(&dir, &switches))
            .await
            .map_err(|why| self.mod_fault(format!("the mods folder could not be read: {why}")))
    }

    /// Say the list now, because something here just changed it. The rescan
    /// would find it in a moment; this is for the Bridge that is waiting.
    async fn mods_announced(&self) -> Result<ModList, Refusal> {
        let list = self.mods_listed().await?;
        self.mods().published(&list);
        self.publish(Event::ModsChanged(list.clone()));
        Ok(list)
    }

    /// `name` as a slug naming a folder that is there, or the refusal.
    pub(super) fn mod_named(&self, name: &str) -> Result<(), Refusal> {
        if let Some(why) = slug_problem(name) {
            return Err(Refusal::Unacceptable(
                WireError::raised(UNACCEPTABLE_MOD, why, self.run_id())
                    .with_field("name", ipc::WireValue::Str(name.to_string())),
            ));
        }
        match std::fs::symlink_metadata(self.mods_dir().join(name)) {
            Ok(meta) if meta.is_dir() => Ok(()),
            _ => Err(Refusal::NoSuchJob(
                WireError::raised(NO_SUCH_MOD, format!("there is no mod called `{name}`"), self.run_id())
                    .with_field("name", ipc::WireValue::Str(name.to_string())),
            )),
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
            scaffold::make(&dir, &name, asked.description.as_deref())
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
        self.mod_named(&name)?;
        let dir = self.mods_dir();
        let asked = name.clone();
        let found = tokio::task::spawn_blocking(move || examine(&dir, &asked))
            .await
            .map_err(|why| self.mod_fault(format!("the mod could not be read: {why}")))?;
        Ok(ModChecked {
            name,
            valid: found.valid(),
            css: found.files.map(|files| files.css),
            problems: found.problems,
        })
    }

    async fn promote_mod(&self, promote: PromoteMod) -> Result<ModPromoted, Refusal> {
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
