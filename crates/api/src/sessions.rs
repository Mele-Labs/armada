//! `report_session`, `list_sessions` and `who_owns`. Since 23.43. `rename_session`, since 23.52.
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

/// A person's name for a session. 200 with the row; 422 for a blank title or a
/// session Fleet does not know.
pub(crate) async fn rename_session<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let rename: ipc::RenameSession = match ipc::decode("a session rename", &body) {
        Ok(rename) => rename,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().rename_session(rename).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A session shows the person a page. 200 with the row; 422 for an address that is not `http` or
/// `https`, or a call that places no session. **The session is placed by the connection** where the
/// call arrived through the agent's door.
pub(crate) async fn show_window<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let show: ipc::ShowWindow = match ipc::decode("a window to show", &body) {
        Ok(show) => show,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .show_window(crate::acting::asking(), show)
        .await
    {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The agent sets what it is waiting on the person for. 200 with the row; 422 for a call that
/// places no session. **The session is placed by the connection**, as `show_window`'s is.
pub(crate) async fn waiting_for<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let set: ipc::SetWaitingFor = match ipc::decode("what a session waits on", &body) {
        Ok(set) => set,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .waiting_for(crate::acting::asking(), set)
        .await
    {
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

/// A session or Job takes an orphaned pull request. 200 with the claim; 422 with the reason.
pub(crate) async fn claim_pull_request<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let claim: ipc::ClaimPullRequest = match ipc::decode("a pull request to claim", &body) {
        Ok(claim) => claim,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .claim_pull_request(crate::acting::asking(), claim)
        .await
    {
        Ok(claimed) => answer(StatusCode::OK, &claimed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
