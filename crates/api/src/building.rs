//! The build Fleet runs on: the read, and the restart onto another.
//!
//! **No `Resolved` extractor and no path parameter**, `limiting`'s reason:
//! the build is Fleet's own, and no Job's. The change answers 202, because it
//! starts a restart that stops the Fleet answering it.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use ipc::ChangeFleetBuild;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Commands, Queries};
use crate::served::Served;

/// Which build Fleet runs on, and where it stands against `origin/main`.
pub(crate) async fn get_fleet_build<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_fleet_build().await {
        Ok(report) => answer(StatusCode::OK, &report, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Start a restart onto `main` or the preview, and answer that it has begun.
pub(crate) async fn change_fleet_build<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: ChangeFleetBuild = match ipc::decode("build to change to", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().change_fleet_build(asked).await {
        Ok(changing) => answer(StatusCode::ACCEPTED, &changing, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
