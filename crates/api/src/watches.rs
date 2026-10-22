//! Watches over the router: started, weighed, stopped and listed for one Job.
//!
//! **The Job is the connection's** where Fleet hosts it, as a request's is; a body's `session_id`
//! is read only where the connection places none.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Watches;
use crate::served::Served;

pub(crate) async fn start_watch<D: Watches>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let start: ipc::StartWatch = match ipc::decode("a watch to start", &body) {
        Ok(start) => start,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().start_watch(crate::acting::asking(), start).await {
        Ok(started) => answer(StatusCode::OK, &started, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn weigh_watch<D: Watches>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let weigh: ipc::WeighWatch = match ipc::decode("a watch weighed", &body) {
        Ok(weigh) => weigh,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().weigh_watch(crate::acting::asking(), weigh).await {
        Ok(weighed) => answer(StatusCode::OK, &weighed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn stop_watch<D: Watches>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let stop: ipc::StopWatch = match ipc::decode("a watch to stop", &body) {
        Ok(stop) => stop,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().stop_watch(crate::acting::asking(), stop).await {
        Ok(stopped) => answer(StatusCode::OK, &stopped, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?session_id=`, read only where the connection places no Job.
#[derive(Deserialize)]
pub(crate) struct ForWatches {
    #[serde(default)]
    session_id: Option<String>,
}

pub(crate) async fn list_watches<D: Watches>(
    State(served): State<Served<D>>,
    Query(asked): Query<ForWatches>,
) -> Response {
    let named = asked
        .session_id
        .filter(|id| !id.trim().is_empty())
        .map(|id| ipc::SessionId::carried(id.trim()));
    match served.daemon().list_watches(crate::acting::asking(), named).await {
        Ok(list) => answer(StatusCode::OK, &list, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
