//! The test map, read for a set of paths. `docs/concepts/test-map.md`.
//!
//! **A read on `GET`, with the paths in the query string**: the inventory holds
//! a query to `GET`, and a list has no other place on one. `paths` is
//! newline-separated, as `armada check test --changed` reads them on stdin, so
//! the caller that has them that way sends them that way.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::answers::{answer, refused};
use crate::daemon::Queries;
use crate::served::Served;

/// `?manifest_id=&paths=` on `get_tests_reaching`.
#[derive(Deserialize)]
pub(crate) struct Reaching {
    #[serde(default)]
    manifest_id: Option<String>,
    /// Newline-separated. A blank line is no path.
    #[serde(default)]
    paths: String,
}

/// The tests reaching each of the paths, and the paths nothing reaches.
pub(crate) async fn get_tests_reaching<D: Queries>(
    State(served): State<Served<D>>,
    Query(Reaching { manifest_id, paths }): Query<Reaching>,
) -> Response {
    let paths = paths
        .lines()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect();
    match served
        .daemon()
        .get_tests_reaching(manifest_id.map(ipc::ManifestId::carried), paths)
        .await
    {
        Ok(reaching) => answer(StatusCode::OK, &reaching, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
