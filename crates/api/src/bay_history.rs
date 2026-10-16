//! `get_slot_history`: who held one slot and what was checked out in it.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::answers::{answer, refused};
use crate::daemon::Queries;
use crate::served::Served;

/// `?manifest_id=&slot=`, both asked for. A query missing either is refused by the extractor as any
/// malformed query is.
#[derive(Deserialize)]
pub(crate) struct Asked {
    manifest_id: String,
    slot: u32,
}

pub(crate) async fn get_slot_history<D: Queries>(
    State(served): State<Served<D>>,
    Query(asked): Query<Asked>,
) -> Response {
    let ask = ipc::BayHistoryAsk { manifest_id: ipc::ManifestId::carried(asked.manifest_id), slot: asked.slot };
    match served.daemon().get_slot_history(ask).await {
        Ok(history) => answer(StatusCode::OK, &history, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
