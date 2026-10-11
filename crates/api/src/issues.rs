//! `list_issues`. #91, `docs/concepts/tracker.md`.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use ipc::ManifestId;

use crate::answers::{answer, refused};
use crate::daemon::Issues;
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
