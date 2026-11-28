//! A Job's live graphs over the router: shown, fed, withdrawn and listed.
//!
//! **The Job is the connection's** where Fleet hosts it, as a watch's is; a body's `session_id` is read only where
//! the connection places none.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Graphs;
use crate::served::Served;

pub(crate) async fn show_graph<D: Graphs>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let show: ipc::ShowGraph = match ipc::decode_body("a graph to show", &body) {
        Ok(show) => show,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().show_graph(crate::acting::asking(), show).await {
        Ok(shown) => answer(StatusCode::OK, &shown, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn graph_points<D: Graphs>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let points: ipc::GraphPoints = match ipc::decode_body("rows for a feed", &body) {
        Ok(points) => points,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().graph_points(crate::acting::asking(), points).await {
        Ok(taken) => answer(StatusCode::OK, &taken, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn withdraw_graph<D: Graphs>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let withdraw: ipc::WithdrawGraph = match ipc::decode_body("a graph to withdraw", &body) {
        Ok(withdraw) => withdraw,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().withdraw_graph(crate::acting::asking(), withdraw).await {
        Ok(shown) => answer(StatusCode::OK, &shown, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?session_id=`, read only where the connection places no Job.
#[derive(Deserialize)]
pub(crate) struct ForGraphs {
    #[serde(default)]
    session_id: Option<String>,
}

pub(crate) async fn list_graphs<D: Graphs>(State(served): State<Served<D>>, Query(asked): Query<ForGraphs>) -> Response {
    let named = asked.session_id.filter(|id| !id.trim().is_empty()).map(|id| ipc::SessionId::carried(id.trim()));
    match served.daemon().list_graphs(crate::acting::asking(), named).await {
        Ok(list) => answer(StatusCode::OK, &list, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
