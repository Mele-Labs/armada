//! The Features tab's route: a project's library, a request off the main thread on the port the Fleet
//! connection read. `crates/ipc/operations/list_features.toml`.

use gpui::{App, Task};
use ipc::{FeatureAsks, FeatureLibrary, FeaturePage};

use crate::settings::wire::{request, Ask, Refused};

/// `GET /features?project=`.
pub fn library(port: u16, project: &str, cx: &App) -> Task<Result<FeatureLibrary, Refused>> {
    request(port, Ask::Get(format!("/features?project={}", crate::link::percent_encoded(project))), "FeatureLibrary", cx)
}

/// `GET /features/page?feature=<slug>&project=`: one feature, whole. `crates/ipc/operations/get_feature_page.toml`.
pub fn page(port: u16, project: &str, slug: &str, cx: &App) -> Task<Result<FeaturePage, Refused>> {
    let path = format!("/features/page?feature={}&project={}", crate::link::percent_encoded(slug), crate::link::percent_encoded(project));
    request(port, Ask::Get(path), "FeaturePage", cx)
}

/// `GET /projects/asks?project=&feature=`: the asks filed under one feature. `crates/ipc/operations/list_feature_asks.toml`.
pub fn asks(port: u16, project: &str, slug: &str, cx: &App) -> Task<Result<FeatureAsks, Refused>> {
    let path = format!("/projects/asks?project={}&feature={}", crate::link::percent_encoded(project), crate::link::percent_encoded(slug));
    request(port, Ask::Get(path), "FeatureAsks", cx)
}
