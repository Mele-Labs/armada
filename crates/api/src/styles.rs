//! `get_style_sources` and `get_kept_styles`: the stylesheets a `mock` node in a sketch is drawn
//! with, and the style set Fleet kept when it drew it.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::answers::{answer, refused};
use crate::daemon::Sessions;
use crate::served::Served;

/// `?path=` to search from, and `?session_id=` read only where the connection places no Job.
#[derive(Deserialize)]
pub(crate) struct WhichStyles {
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
}

pub(crate) async fn get_style_sources<D: Sessions>(
    State(served): State<Served<D>>,
    Query(which): Query<WhichStyles>,
) -> Response {
    let named = which
        .session_id
        .filter(|id| !id.trim().is_empty())
        .map(|id| ipc::SessionId::carried(id.trim()));
    let path = which.path.map(|path| path.trim().to_string()).filter(|path| !path.is_empty());
    match served.daemon().get_style_sources(crate::acting::asking(), named, path).await {
        Ok(sources) => answer(StatusCode::OK, &sources, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?hash=`, the content hash a `mock` node's `kept` names.
#[derive(Deserialize)]
pub(crate) struct WhichKept {
    hash: String,
}

pub(crate) async fn get_kept_styles<D: Sessions>(
    State(served): State<Served<D>>,
    Query(which): Query<WhichKept>,
) -> Response {
    match served.daemon().get_kept_styles(which.hash).await {
        Ok(kept) => answer(StatusCode::OK, &kept, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
