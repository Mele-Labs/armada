//! The Features tab's route: a project's library, a request off the main thread on the port the Fleet
//! connection read. `crates/ipc/operations/list_features.toml`.

use gpui::{App, Task};
use ipc::FeatureLibrary;

use crate::settings::wire::{request, Ask, Refused};

/// `GET /features?project=`.
pub fn library(port: u16, project: &str, cx: &App) -> Task<Result<FeatureLibrary, Refused>> {
    request(port, Ask::Get(format!("/features?project={}", crate::link::percent_encoded(project))), "FeatureLibrary", cx)
}
