//! The Usage page's route: the waste in the burn, a request off the main thread on the port the Fleet connection read.
//! `crates/ipc/operations/get_usage_waste.toml`.

use gpui::{App, Task};
use ipc::UsageWaste;

use super::model::Span;
use crate::settings::wire::{request, Ask, Refused};

/// `GET /usage/waste?window=day|week|fortnight`.
pub fn waste(port: u16, span: Span, cx: &App) -> Task<Result<UsageWaste, Refused>> {
    request(port, Ask::Get(format!("/usage/waste?window={}", span.id())), "UsageWaste", cx)
}
