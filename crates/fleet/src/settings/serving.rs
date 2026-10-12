//! Fleet's side of settings.json: the six operations, a save, and the re-read
//! the composition root's watch on the file asks for.
//!
//! **Every change goes through [`settings_adopted`](Fleet::settings_adopted)**, so a save and a
//! hand edit put a value in force the same way — the live dials, then the
//! limits under the roster — and say so once on the stream.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Settings};
use config::settings::{Refused, Resolved, Saved};
use ipc::{Event, FleetLimits, Preferences, SaveLimits, SavePreference, SaveSettings, SettingsList, WireError};
use store::settings_file;

use super::{checked_whole, file_value, read_checked};
use crate::daemon::Fleet;

/// A save naming a key the table does not have, or a value it will not take.
/// A 422 naming the key, and nothing is written.
const UNACCEPTABLE_SETTINGS: &str = "fleet.unacceptable_settings";
/// The file would not be written. A 500, and nothing in force moved.
const SETTINGS_NOT_WRITTEN: &str = "fleet.settings_not_written";

/// What one re-read of the file came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reread {
    /// What Fleet already holds, or the refusal it already holds. Nothing said.
    Quiet,
    /// A different file, taken and in force.
    Taken,
    /// Refused whole. The last good settings stay in force.
    Refused(Refused),
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
    /// Change keys of settings.json, checked whole against the file it would
    /// write, then put it in force. Nothing is written on a refusal.
    pub(crate) async fn save_settings_now(&self, save: SaveSettings) -> Result<SettingsList, Refusal> {
        let settings = self.machine_settings();
        let _one = settings.saving.lock().await;
        let now = settings.now();
        let mut document = now.saved().document().clone();
        for (key, value) in save.changes {
            match value {
                Some(value) => document.insert(key, file_value(&value)),
                None => document.remove(&key),
            };
        }
        let saved = checked_whole(&document, &now).map_err(|refused| {
            let error = WireError::raised(UNACCEPTABLE_SETTINGS, refused.reason, self.run_id());
            Refusal::Unacceptable(match refused.key {
                Some(key) => error.with_field("key", ipc::WireValue::Str(key)),
                None => error,
            })
        })?;
        settings_file::write(settings.path(), saved.document()).map_err(|why| {
            Refusal::Fault(WireError::raised(
                SETTINGS_NOT_WRITTEN,
                format!("settings.json could not be written: {why}"),
                self.run_id(),
            ))
        })?;
        Ok(self.settings_adopted(saved).await)
    }

    /// Read the file again, because the watch on it saw it change.
    pub async fn settings_reread(&self) -> Reread {
        let settings = self.machine_settings();
        let _one = settings.saving.lock().await;
        let now = settings.now();
        match read_checked(settings.path(), &now) {
            Ok(saved) if saved == *now.saved() && settings.refused().is_none() => Reread::Quiet,
            Ok(saved) => {
                self.settings_adopted(saved).await;
                Reread::Taken
            }
            Err(refused) if settings.refused().as_ref() == Some(&refused) => Reread::Quiet,
            Err(refused) => {
                settings.held().refused = Some(refused.clone());
                self.publish(Event::SettingsChanged(settings.listed()));
                Reread::Refused(refused)
            }
        }
    }

    /// Put `saved` in force, clear any refusal, and say so.
    async fn settings_adopted(&self, saved: Saved) -> SettingsList {
        let settings = self.machine_settings();
        let resolved: Resolved = {
            let mut held = settings.held();
            held.now = held.now.with(saved);
            held.refused = None;
            held.now.clone()
        };
        self.retuned(&resolved);
        self.relimited(&resolved).await;
        let list = settings.listed();
        self.publish(Event::SettingsChanged(list.clone()));
        list
    }
}

impl<H, V, W> Settings for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn get_settings(&self) -> Result<SettingsList, Refusal> {
        Ok(self.machine_settings().listed())
    }

    /// **Not `budgeted`**, `save_limits`' reason: it writes one small file and
    /// waits on the roster at most, and no Job is touched.
    async fn save_settings(&self, save: SaveSettings) -> Result<SettingsList, Refusal> {
        self.save_settings_now(save).await
    }

    async fn get_limits(&self) -> Result<FleetLimits, Refusal> {
        Ok(self.limits_in_force().await)
    }

    async fn save_limits(&self, save: SaveLimits) -> Result<FleetLimits, Refusal> {
        self.save_limits_now(save).await
    }

    async fn get_preferences(&self) -> Result<Preferences, Refusal> {
        Ok(self.preferences_in_force())
    }

    async fn save_preferences(&self, save: SavePreference) -> Result<Preferences, Refusal> {
        self.save_preference_now(save).await
    }
}
