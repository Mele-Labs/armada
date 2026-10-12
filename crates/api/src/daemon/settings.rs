//! The Machine's settings: settings.json read and saved, and the older limits
//! and preferences that are four and five of its keys. Its own surface, for
//! [`Mods`](super::Mods)' reason: whole on its own, and a handler taking only
//! this cannot reach for a Job. `docs/concepts/machine.md`, *settings.json*.

use std::future::Future;

use ipc::{FleetLimits, Preferences, SaveLimits, SavePreference, SaveSettings, SettingsList};

use crate::daemon::Refusal;

pub trait Settings: Send + Sync + 'static {
    /// `get_settings` — every setting: its kind, default, what the file holds
    /// and what is in force, and the last hand edit refused, if one stands.
    fn get_settings(&self) -> impl Future<Output = Result<SettingsList, Refusal>> + Send;

    /// `save_settings` — change keys, `null` removing one. Checked whole
    /// against the file it would write: [`Refusal::Unacceptable`] names the
    /// key and nothing is written.
    fn save_settings(
        &self,
        save: SaveSettings,
    ) -> impl Future<Output = Result<SettingsList, Refusal>> + Send;

    /// `get_limits` — the four limits in force, and what ships. **What
    /// `get_capacity` is measured against.** The only `Refusal` is a fault.
    fn get_limits(&self) -> impl Future<Output = Result<FleetLimits, Refusal>> + Send;

    /// `save_limits` — any of the four, an omitted one keeping its value.
    /// Nothing out of range reaches this: `ipc::SaveLimits` cannot hold one.
    fn save_limits(&self, save: SaveLimits) -> impl Future<Output = Result<FleetLimits, Refusal>> + Send;

    /// `get_preferences` — every Bridge preference in force. The only
    /// `Refusal` is a fault.
    fn get_preferences(&self) -> impl Future<Output = Result<Preferences, Refusal>> + Send;

    /// `save_preferences` — one preference by name. [`Refusal::Unacceptable`]
    /// for a name outside the closed set, or a value it will not take.
    fn save_preferences(
        &self,
        save: SavePreference,
    ) -> impl Future<Output = Result<Preferences, Refusal>> + Send;
}
