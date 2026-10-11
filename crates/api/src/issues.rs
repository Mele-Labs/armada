//! `list_issues`. #91, `docs/concepts/tracker.md`. Mounted with Autopilot's routes, which pick
//! from the same list: `crate::autopilots`.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;

use ipc::ManifestId;

use crate::answers::{answer, refused};
use crate::autopilots as ap;
use crate::daemon::{Autopilots, Issues};
use crate::served::Served;

/// `?refresh=true` asks the tracker again whatever is held.
#[derive(Deserialize)]
pub(crate) struct Refresh {
    #[serde(default)]
    refresh: bool,
}

pub(crate) async fn list_issues<D: Issues>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
    Query(asked): Query<Refresh>,
) -> Response {
    match served
        .daemon()
        .list_issues(ManifestId::carried(repository), asked.refresh)
        .await
    {
        Ok(list) => answer(StatusCode::OK, &list, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The issues route and Autopilot's, as one router the surface merges.
pub(crate) fn mounted<D: Issues + Autopilots>() -> Router<Served<D>> {
    Router::new()
        .route("/issues/:repository", get(list_issues::<D>))
        .route("/autopilots", get(ap::list_autopilots::<D>))
        .route(
            "/autopilots/:repository",
            get(ap::get_autopilot::<D>).post(ap::start_autopilot::<D>),
        )
        .route("/autopilots/:repository/proposal", post(ap::propose_autopilot::<D>))
        .route("/autopilots/:repository/pause", post(ap::pause_autopilot::<D>))
        .route("/autopilots/:repository/resume", post(ap::resume_autopilot::<D>))
        .route("/autopilots/:repository/stop", post(ap::stop_autopilot::<D>))
        .route(
            "/autopilots/:repository/picks/:pick_id/dispatch",
            post(ap::dispatch_set_aside::<D>),
        )
        .route(
            "/autopilots/:repository/picks/:pick_id/dismiss",
            post(ap::dismiss_set_aside::<D>),
        )
}
