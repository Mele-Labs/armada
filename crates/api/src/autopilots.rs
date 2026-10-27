//! Autopilot's routes. #2092, `docs/concepts/autopilot.md`.

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;

use ipc::ManifestId;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Autopilots, Refusal};
use crate::served::Served;

fn answered<T: serde::Serialize>(
    status: StatusCode,
    result: Result<T, Refusal>,
    run_id: &ipc::RunId,
) -> Response {
    match result {
        Ok(value) => answer(status, &value, run_id),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn propose_autopilot<D: Autopilots>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
    body: Bytes,
) -> Response {
    let propose: ipc::ProposeAutopilot = match ipc::decode_body("a sentence for Autopilot", &body) {
        Ok(propose) => propose,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    let read = served
        .daemon()
        .propose_autopilot(ManifestId::carried(repository), propose)
        .await;
    answered(StatusCode::OK, read, served.run_id())
}

/// 201 with the Autopilot, which is picking from the next interval.
pub(crate) async fn start_autopilot<D: Autopilots>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
    body: Bytes,
) -> Response {
    let settings: ipc::AutopilotSettings = match ipc::decode_body("Autopilot's settings", &body) {
        Ok(settings) => settings,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    let started = served
        .daemon()
        .start_autopilot(ManifestId::carried(repository), settings)
        .await;
    answered(StatusCode::CREATED, started, served.run_id())
}

/// New settings for one running or paused. 200 with the Autopilot, in the state it was in.
pub(crate) async fn update_autopilot<D: Autopilots>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
    body: Bytes,
) -> Response {
    let settings: ipc::AutopilotSettings = match ipc::decode_body("Autopilot's settings", &body) {
        Ok(settings) => settings,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    let updated = served
        .daemon()
        .update_autopilot(ManifestId::carried(repository), settings)
        .await;
    answered(StatusCode::OK, updated, served.run_id())
}

pub(crate) async fn pause_autopilot<D: Autopilots>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
) -> Response {
    let paused = served
        .daemon()
        .pause_autopilot(ManifestId::carried(repository))
        .await;
    answered(StatusCode::OK, paused, served.run_id())
}

pub(crate) async fn resume_autopilot<D: Autopilots>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
) -> Response {
    let resumed = served
        .daemon()
        .resume_autopilot(ManifestId::carried(repository))
        .await;
    answered(StatusCode::OK, resumed, served.run_id())
}

pub(crate) async fn stop_autopilot<D: Autopilots>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
) -> Response {
    let stopped = served
        .daemon()
        .stop_autopilot(ManifestId::carried(repository))
        .await;
    answered(StatusCode::OK, stopped, served.run_id())
}

pub(crate) async fn list_autopilots<D: Autopilots>(State(served): State<Served<D>>) -> Response {
    let listed = served.daemon().list_autopilots().await;
    answered(StatusCode::OK, listed, served.run_id())
}

pub(crate) async fn get_autopilot<D: Autopilots>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
) -> Response {
    let held = served
        .daemon()
        .get_autopilot(ManifestId::carried(repository))
        .await;
    answered(StatusCode::OK, held, served.run_id())
}

pub(crate) async fn dispatch_set_aside<D: Autopilots>(
    State(served): State<Served<D>>,
    Path((repository, pick_id)): Path<(String, String)>,
) -> Response {
    let dispatched = served
        .shared()
        .dispatch_set_aside(ManifestId::carried(repository), pick_id)
        .await;
    answered(StatusCode::OK, dispatched, served.run_id())
}

pub(crate) async fn dismiss_set_aside<D: Autopilots>(
    State(served): State<Served<D>>,
    Path((repository, pick_id)): Path<(String, String)>,
) -> Response {
    let dismissed = served
        .daemon()
        .dismiss_set_aside(ManifestId::carried(repository), pick_id)
        .await;
    answered(StatusCode::OK, dismissed, served.run_id())
}
