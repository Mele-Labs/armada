//! Fleet's side of settings.json: the six operations, a save, and the re-read
//! the composition root's watch on the file asks for.
//!
//! **Every change goes through [`settings_adopted`](Fleet::settings_adopted)**, so a save and a
//! hand edit put a value in force the same way — the live dials, then the
//! limits under the roster — and say so once on the stream.

use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Refusal, Settings};
use config::settings::{entry, Refused, Resolved, Saved};
use core_model::{Component, Envelope, FieldValue, Level};
use ipc::{Event, FleetLimits, Preferences, SaveLimits, SavePreference, SaveSettings, SettingsList, WireError};
use store::settings_file;

use super::{checked_whole, file_value, keep_last_good, read_checked};
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
        let unacceptable = |key: Option<String>, reason: String| {
            let error = WireError::raised(UNACCEPTABLE_SETTINGS, reason, self.run_id());
            Refusal::Unacceptable(match key {
                Some(key) => error.with_field("key", ipc::WireValue::Str(key)),
                None => error,
            })
        };
        // **The file as it stands, never what is in force.** What is in force after a refusal is
        // not the file, and writing it with this change would lose whatever the file holds.
        let mut document = read_checked(settings.path(), &now)
            .map_err(|refused| {
                unacceptable(refused.key, format!("settings.json was not taken, so nothing is saved: {}", refused.reason))
            })?
            .document()
            .clone();
        for (key, value) in save.changes {
            // The file may carry a key another build wrote. A save may not add one.
            if entry(&key).is_none() {
                let reason = format!("settings.json names `{key}`, which is not a setting Armada has");
                return Err(unacceptable(Some(key), reason));
            }
            match value {
                Some(value) => document.insert(key, file_value(&value)),
                None => document.remove(&key),
            };
        }
        let saved = checked_whole(&document, &now).map_err(|refused| unacceptable(refused.key, refused.reason))?;
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

    /// Keep each setting the Usage page changes that `after` holds differently from `before`, so what the change
    /// did to the waste it cuts can be read later. A save and a hand edit are kept alike.
    async fn adjustments_kept(&self, before: &Resolved, after: &Resolved) {
        let changed = crate::usage_adjust::changed(before, after);
        if changed.is_empty() {
            return;
        }
        let at = self.now().as_str().to_string();
        {
            let mut store = self.store().lock().await;
            for (key, from, to) in changed {
                if let Err(why) = store.keep_settings_change(key, &at, &from, &to) {
                    let envelope = Envelope::new(self.now(), Level::Warn, Component::Fleet, self.run().clone(), "a change to a setting was not kept")
                        .with_field("key", FieldValue::Str(key.to_string()))
                        .with_field("why", FieldValue::Str(why.to_string()));
                    crate::transcript::note_aloud(&envelope);
                }
            }
        }
        // After the row is kept, so an answer made from here on holds the change, and none made before it is served again.
        self.usage_memo().adjustments_changed().await;
    }

    /// Put `saved` in force, clear any refusal, and say so.
    async fn settings_adopted(&self, saved: Saved) -> SettingsList {
        let settings = self.machine_settings();
        let was_on = self.models_on();
        let operator_was_on = self.operator_on();
        if let Err(why) = keep_last_good(settings.path(), &saved) {
            let envelope = Envelope::new(self.now(), Level::Warn, Component::Fleet, self.run().clone(), "the last good copy of settings.json was not kept")
                .with_field("why", FieldValue::Str(why.to_string()));
            crate::transcript::note_aloud(&envelope);
        }
        let (before, resolved): (Resolved, Resolved) = {
            let mut held = settings.held();
            let before = held.now.clone();
            held.now = held.now.with(saved);
            held.refused = None;
            (before, held.now.clone())
        };
        self.adjustments_kept(&before, &resolved).await;
        self.retuned(&resolved);
        self.relimited(&resolved).await;
        self.downshift_settings_moved(&resolved).await;
        self.models_switched_off(&was_on).await;
        self.operator_switched(operator_was_on).await;
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

    /// A change to how he works is told to the Jobs running and to the Operator, each its own new note as
    /// Fleet's word, so the next result and question follow it. A save that leaves a note as it was tells none.
    async fn preferences_saved(self: Arc<Self>, before: Preferences, now: Preferences) {
        let job = crate::ways::note(&now);
        let operator = crate::ways::operator_note(&now);
        let job = (job != crate::ways::note(&before)).then_some(job);
        let operator = (operator != crate::ways::operator_note(&before)).then_some(operator);
        if job.is_some() || operator.is_some() {
            self.ways_told(job.as_deref(), operator.as_deref()).await;
        }
    }

    async fn get_home_layout(&self) -> Result<ipc::HomeLayout, Refusal> {
        Ok(self.home_layout_in_force())
    }

    async fn save_home_layout(&self, save: ipc::SaveHomeLayout) -> Result<ipc::HomeLayout, Refusal> {
        self.home_layout_saved(save).await
    }

    async fn get_phone(&self) -> Result<ipc::PhoneGateway, Refusal> {
        self.phone_now()
    }

    async fn set_phone(&self, set: ipc::SetPhoneGateway) -> Result<ipc::PhoneGateway, Refusal> {
        self.phone_set(set.on)
    }

    async fn get_mac_setup(&self) -> Result<ipc::MacSetup, Refusal> {
        self.mac_setup_now()
    }

    async fn set_up_mac(&self) -> Result<ipc::MacSetup, Refusal> {
        self.mac_set_up().await
    }
}
