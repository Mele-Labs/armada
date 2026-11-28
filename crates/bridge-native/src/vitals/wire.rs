//! The Vitals page's one seam to Fleet. Fleet has no route for the machine's readings yet, so the live page reads the fixture
//! too, and the settings it changes are kept in memory on the page (`mod.rs`). The group that lands the route swaps these bodies
//! and nothing else.

use gpui::{App, Entity, Task};
use ipc::SettingsList;

use super::model::Reading;
use crate::fleet::FleetConnection;

/// The machine over the last day, and the changes made to its limits.
pub fn reading(_fleet: &Entity<FleetConnection>, _cx: &App) -> Task<Result<Reading, String>> {
    Task::ready(Ok(super::mock::reading()))
}

/// The settings the page draws, with what is saved and what ships.
pub fn settings(_fleet: &Entity<FleetConnection>, _cx: &App) -> SettingsList {
    super::mock::settings()
}
