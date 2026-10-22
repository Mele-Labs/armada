//! `list_write_clashes` and `read_clash_file`, and beside them, as what the agents on a repository write and where:
//! `list_repository_areas`, `list_slot_touches` and `get_session_file_change`.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::answers::{answer, refused};
use crate::daemon::Clashing;
use crate::served::Served;

/// `?manifest_id=&holder=` on `list_write_clashes`.
#[derive(Deserialize)]
pub(crate) struct ClashQuery {
    #[serde(default)]
    manifest_id: Option<String>,
    #[serde(default)]
    holder: Option<String>,
}

pub(crate) async fn list_write_clashes<D: Clashing>(
    State(served): State<Served<D>>,
    Query(asked): Query<ClashQuery>,
) -> Response {
    let manifest = asked.manifest_id.map(ipc::ManifestId::carried);
    match served.daemon().list_write_clashes(manifest, asked.holder).await {
        Ok(clashes) => answer(StatusCode::OK, &clashes, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?session_id=&other_kind=&other_id=&path=` on `read_clash_file`.
#[derive(Deserialize)]
pub(crate) struct ClashFileQuery {
    session_id: String,
    other_kind: ipc::ClashSide,
    #[serde(default)]
    other_id: String,
    path: String,
}

pub(crate) async fn read_clash_file<D: Clashing>(
    State(served): State<Served<D>>,
    Query(asked): Query<ClashFileQuery>,
) -> Response {
    match served
        .daemon()
        .read_clash_file(asked.session_id, asked.other_kind, asked.other_id, asked.path)
        .await
    {
        Ok(file) => answer(StatusCode::OK, &file, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?session_id=&path=&seen=` on `get_session_file_change`.
#[derive(Deserialize)]
pub(crate) struct FileChangeQuery {
    session_id: String,
    path: String,
    #[serde(default)]
    seen: Option<String>,
}

pub(crate) async fn get_session_file_change<D: Clashing>(
    State(served): State<Served<D>>,
    Query(asked): Query<FileChangeQuery>,
) -> Response {
    match served.daemon().get_session_file_change(asked.session_id, asked.path, asked.seen).await {
        Ok(change) => answer(StatusCode::OK, &change, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `GET /repository_areas`: every served repository's directories.
pub(crate) async fn list_repository_areas<D: Clashing>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_repository_areas().await {
        Ok(areas) => answer(StatusCode::OK, &areas, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `GET /slot_touches`: what each holder of a slot has changed in it.
pub(crate) async fn list_slot_touches<D: Clashing>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_slot_touches().await {
        Ok(touches) => answer(StatusCode::OK, &touches, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
