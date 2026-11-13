//! The build Fleet runs on: the read, the restart onto another, the revert to the one before, and the
//! snapshot of picked Jobs with its swap and keep.
//!
//! **No `Resolved` extractor and no path parameter**, `limiting`'s reason:
//! the build is Fleet's own, and no Job's. The change answers 202, because it
//! starts a restart that stops the Fleet answering it.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use ipc::{ChangeFleetBuild, PickFleetBuildJob, SkipPreviewRefresh, UpdateMac};

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

/// The branches Jobs offered for the preview and the one Refresh row.
pub(crate) async fn get_preview_offers<D: Queries>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_preview_offers().await {
        Ok(offers) => answer(StatusCode::OK, &offers, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Skip the Refresh row with this id until the set of ready offers changes.
pub(crate) async fn skip_preview_refresh<D: Commands>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: SkipPreviewRefresh = match ipc::decode_body("a Refresh row to skip", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().skip_preview_refresh(asked).await {
        Ok(skipped) => answer(StatusCode::OK, &skipped, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Start a restart onto `main` or the preview, and answer that it has begun.
pub(crate) async fn change_fleet_build<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: ChangeFleetBuild = match ipc::decode_body("build to change to", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().change_fleet_build(asked).await {
        Ok(changing) => answer(StatusCode::ACCEPTED, &changing, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Move this Mac, another of his Macs, or both onto one build, keeping running Jobs, and answer that it has begun.
pub(crate) async fn update_mac<D: Commands>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: UpdateMac = match ipc::decode_body("a Mac to update", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().update_mac(asked).await {
        Ok(changing) => answer(StatusCode::ACCEPTED, &changing, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Start the undo of the last restart, and answer that it has begun. No body: the build reverted
/// to is the one `builds/previous` names.
pub(crate) async fn revert_fleet_build<D: Commands>(State(served): State<Served<D>>) -> Response {
    match served.daemon().revert_fleet_build().await {
        Ok(reverting) => answer(StatusCode::ACCEPTED, &reverting, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Pick a Job's branch for the next snapshot, or take it out, and start building the snapshot.
pub(crate) async fn pick_fleet_build_job<D: Commands>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: PickFleetBuildJob = match ipc::decode_body("a branch to pick for the snapshot", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().pick_fleet_build_job(asked).await {
        Ok(snapshotting) => answer(StatusCode::ACCEPTED, &snapshotting, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Start the swap onto the kept snapshot, and answer that it has begun. No body: the build swapped
/// onto is the one `snapshot` names.
pub(crate) async fn swap_fleet_build<D: Commands>(State(served): State<Served<D>>) -> Response {
    match served.daemon().swap_fleet_build().await {
        Ok(swapping) => answer(StatusCode::ACCEPTED, &swapping, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Keep the running build's tried Jobs, and answer the titles of the Jobs told to land them.
pub(crate) async fn keep_fleet_build<D: Commands>(State(served): State<Served<D>>) -> Response {
    match served.shared().keep_fleet_build().await {
        Ok(kept) => answer(StatusCode::OK, &kept, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
