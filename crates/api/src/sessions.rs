//! `report_session`, `list_sessions` and `who_owns`. Since 23.43.
//! `docs/concepts/session.md`.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Sessions;
use crate::served::Served;

/// What a harness tells Fleet. 200: the row is kept when this answers.
pub(crate) async fn report_session<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let report: ipc::SessionReport = match ipc::decode("a session report", &body) {
        Ok(report) => report,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().report_session(report).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?manifest_id=`, `?q=` and `?state=` on `list_sessions`.
#[derive(Deserialize)]
pub(crate) struct Listing {
    #[serde(default)]
    manifest_id: Option<String>,
    #[serde(default)]
    q: Option<String>,
    #[serde(default)]
    state: Option<ipc::SessionState>,
}

pub(crate) async fn list_sessions<D: Sessions>(
    State(served): State<Served<D>>,
    Query(listing): Query<Listing>,
) -> Response {
    let manifest = listing.manifest_id.map(ipc::ManifestId::carried);
    match served
        .daemon()
        .list_sessions(manifest, listing.q, listing.state)
        .await
    {
        Ok(sessions) => answer(StatusCode::OK, &sessions, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?kind=` and `?target=` on `who_owns`, beside `?manifest_id=`.
#[derive(Deserialize)]
pub(crate) struct Asked {
    kind: String,
    target: String,
    #[serde(default)]
    manifest_id: Option<String>,
}

pub(crate) async fn who_owns<D: Sessions>(
    State(served): State<Served<D>>,
    Query(asked): Query<Asked>,
) -> Response {
    let manifest = asked.manifest_id.map(ipc::ManifestId::carried);
    match served
        .daemon()
        .who_owns(asked.kind, asked.target, manifest)
        .await
    {
        Ok(owners) => answer(StatusCode::OK, &owners, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
