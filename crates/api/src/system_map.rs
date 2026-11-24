//! A project's system map over the wire. `docs/practices/protocol/projects.md`, *The system map*. A handler is a decode and one call.

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Projects;
use crate::served::Served;

/// `?part=&feature=` on `get_system_map`: neither is level one, `part` level two, `feature` level three.
#[derive(serde::Deserialize)]
pub(crate) struct Level {
    part: Option<String>,
    feature: Option<String>,
}

#[derive(serde::Deserialize)]
pub(crate) struct OfPart {
    part: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct OfPath {
    #[serde(default)]
    path: String,
}

pub(crate) async fn get_system_map<D: Projects>(State(served): State<Served<D>>, Path(project): Path<String>, Query(asked): Query<Level>) -> Response {
    match served.daemon().get_system_map(project, asked.part.filter(|one| !one.is_empty()), asked.feature.filter(|one| !one.is_empty())).await {
        Ok(map) => answer(StatusCode::OK, &map, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_system_part<D: Projects>(State(served): State<Served<D>>, Path(project): Path<String>, Query(asked): Query<OfPart>) -> Response {
    match served.daemon().get_system_part(project, asked.part).await {
        Ok(answers) => answer(StatusCode::OK, &answers, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn list_system_folder<D: Projects>(State(served): State<Served<D>>, Path(project): Path<String>, Query(asked): Query<OfPath>) -> Response {
    match served.daemon().list_system_folder(project, asked.path).await {
        Ok(folder) => answer(StatusCode::OK, &folder, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn map_system_with_librarian<D: Projects>(State(served): State<Served<D>>, Path(project): Path<String>) -> Response {
    match served.daemon().map_system_with_librarian(project).await {
        Ok(map) => answer(StatusCode::OK, &map, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn answer_system_addition<D: Projects>(State(served): State<Served<D>>, Path(project): Path<String>, body: Bytes) -> Response {
    let given: ipc::AnswerSystemAddition = match ipc::decode_body("an answer to a system addition", &body) {
        Ok(given) => given,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().answer_system_addition(project, given).await {
        Ok(map) => answer(StatusCode::OK, &map, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
