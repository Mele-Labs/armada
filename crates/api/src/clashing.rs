//! `list_write_clashes`, and beside it, as what the agents on a repository write and where:
//! `list_repository_areas` and `list_slot_touches`.

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
