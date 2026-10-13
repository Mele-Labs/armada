//! `get_fleet_timeline`: what happened across the Fleet in a window of time.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use ipc::TimelineAsk;

use crate::answers::{answer, refused};
use crate::daemon::Queries;
use crate::served::Served;

/// `?since=&until=&limit=`, each optional. Never refuses an ask it can read.
pub(crate) async fn get_fleet_timeline<D: Queries>(
    State(served): State<Served<D>>,
    Query(ask): Query<TimelineAsk>,
) -> Response {
    match served.daemon().get_fleet_timeline(ask).await {
        Ok(timeline) => answer(StatusCode::OK, &timeline, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
