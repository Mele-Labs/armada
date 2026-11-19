//! The Usage page's routes: the waste in the burn, the settings that change it, and a change to one of them, each a request
//! off the main thread on the port the Fleet connection read.
//! `crates/ipc/operations/get_usage_waste.toml`, `get_usage_adjustments.toml` and `save_settings.toml`.

use std::collections::BTreeMap;

use gpui::{App, Entity, Task};
use ipc::{SettingValue, SettingsList, UsageAdjustments, UsageWaste};

use super::model::Span;
use crate::fleet::FleetConnection;
use crate::settings::wire::{request, Ask, Refused, Wire};

/// `GET /usage/waste?window=day|week|fortnight`.
pub fn waste(port: u16, span: Span, cx: &App) -> Task<Result<UsageWaste, Refused>> {
    request(port, Ask::Get(format!("/usage/waste?window={}", span.id())), "UsageWaste", cx)
}

/// `GET /usage/adjustments?window=day|week|fortnight`.
pub fn adjustments(port: u16, span: Span, cx: &App) -> Task<Result<UsageAdjustments, Refused>> {
    request(port, Ask::Get(format!("/usage/adjustments?window={}", span.id())), "UsageAdjustments", cx)
}

/// `POST /settings/save` with one change, by the Settings overlay's own route. Fleet answers with the whole list.
pub fn save(fleet: &Entity<FleetConnection>, key: &str, value: SettingValue, cx: &App) -> Task<Result<SettingsList, Refused>> {
    Wire::Live(fleet.clone()).save(BTreeMap::from([(key.to_string(), Some(value))]), cx)
}
