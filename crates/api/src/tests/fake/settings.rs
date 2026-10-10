//! The Machine's settings, as the fake answers them: one setting, kept in
//! memory, and an unknown key refused by name. What is checked, written and
//! watched is `fleet::settings`', against a real file.

use ipc::{
    FleetLimits, Preferences, SaveLimits, SavePreference, SaveSettings, SettingApplies, SettingKind,
    SettingValue, SettingsList,
};

use super::FakeDaemon;
use crate::{Refusal, Settings};

/// The fake's one setting, at what it ships.
pub(crate) fn settings() -> SettingsList {
    SettingsList {
        path: "/machine/settings.json".to_string(),
        refused: None,
        settings: vec![ipc::Setting {
            key: "limits.dronesAtOnce".to_string(),
            group: "Fleet".to_string(),
            section: "Limits".to_string(),
            title: "Drones at once".to_string(),
            description: "How many Jobs have a Drone working at the same time.".to_string(),
            kind: SettingKind::Integer { min: 1, max: 8, unit: Some("Drones".to_string()) },
            default: SettingValue::Integer(2),
            value: SettingValue::Integer(2),
            saved: None,
            applies: SettingApplies::Live,
            pending_restart: false,
            overridden_by_env: None,
        }],
    }
}

impl Settings for FakeDaemon {
    async fn get_settings(&self) -> Result<SettingsList, Refusal> {
        Ok(self.settings.lock().expect("not poisoned").clone())
    }

    async fn save_settings(&self, save: SaveSettings) -> Result<SettingsList, Refusal> {
        let mut list = self.settings.lock().expect("not poisoned");
        for (key, value) in save.changes {
            let Some(setting) = list.settings.iter_mut().find(|one| one.key == key) else {
                return Err(Refusal::Unacceptable(ipc::WireError::raised(
                    "fleet.unacceptable_settings",
                    format!("settings.json names `{key}`, which is not a setting Armada has"),
                    crate::tests::shapes::run_id(),
                )));
            };
            setting.value = value.clone().unwrap_or_else(|| setting.default.clone());
            setting.saved = value;
        }
        Ok(list.clone())
    }

    /// Whatever the fake's own saves have left, so a route test can read back
    /// what it saved.
    async fn get_limits(&self) -> Result<FleetLimits, Refusal> {
        Ok(*self.limits.lock().expect("not poisoned"))
    }

    /// Each field the save names replaces the fake's value. What a save does to
    /// admission is `fleet::limits`' and tested there.
    async fn save_limits(&self, save: SaveLimits) -> Result<FleetLimits, Refusal> {
        let mut limits = self.limits.lock().expect("not poisoned");
        if let Some(v) = save.concurrency {
            limits.values.concurrency = v.get();
        }
        if let Some(v) = save.memory_spare_percent {
            limits.values.memory_spare_percent = v.get();
        }
        if let Some(v) = save.disk_floor_gib {
            limits.values.disk_floor_gib = v.get();
        }
        if let Some(v) = save.checks_at_once {
            limits.values.checks_at_once = v.get();
        }
        Ok(*limits)
    }

    /// Whatever the fake's own saves have left, `get_limits`' reason.
    async fn get_preferences(&self) -> Result<Preferences, Refusal> {
        Ok(self.preferences.lock().expect("not poisoned").clone())
    }

    /// The one preference name this fake knows; anything else is the 422 a
    /// route test tells apart from a missing route by its code.
    async fn save_preferences(&self, save: SavePreference) -> Result<Preferences, Refusal> {
        let mut preferences = self.preferences.lock().expect("not poisoned");
        match save.name.as_str() {
            "where_things_are_open" => {
                preferences.where_things_are_open = save.value;
                Ok(preferences.clone())
            }
            other => Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fleet.unknown_preference",
                format!("`{other}` is not a preference this build reads"),
                crate::tests::shapes::run_id(),
            ))),
        }
    }
}
