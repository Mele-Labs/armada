//! `list_issues`. #91, `docs/concepts/tracker.md`. Mounted with Autopilot's routes, which pick
//! from the same list: `crate::autopilots`.

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;

use ipc::ManifestId;

use crate::answers::{answer, refused, undecodable};
use crate::autopilots as ap;
use crate::daemon::{Autopilots, Issues};
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

pub(crate) async fn get_issue<D: Issues>(
    State(served): State<Served<D>>,
    Path((repository, reference)): Path<(String, String)>,
) -> Response {
    match served
        .daemon()
        .get_issue(ManifestId::carried(repository), reference)
        .await
    {
        Ok(read) => answer(StatusCode::OK, &read, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A query in the tracker's own language, run once. A POST because a query is typed text.
pub(crate) async fn search_issues<D: Issues>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
    body: Bytes,
) -> Response {
    let search: ipc::SearchIssues = match ipc::decode("an issue search", &body) {
        Ok(search) => search,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().search_issues(ManifestId::carried(repository), search).await {
        Ok(list) => answer(StatusCode::OK, &list, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Install the tool a refusal's `missing_tool` named. A POST because it changes the machine.
pub(crate) async fn install_tool<D: Issues>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
    body: Bytes,
) -> Response {
    let install: ipc::InstallTool = match ipc::decode("a tool to install", &body) {
        Ok(install) => install,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().install_tool(ManifestId::carried(repository), install).await {
        Ok(installed) => answer(StatusCode::OK, &installed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn list_issue_queries<D: Issues>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
) -> Response {
    match served.daemon().list_issue_queries(ManifestId::carried(repository)).await {
        Ok(queries) => answer(StatusCode::OK, &queries, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn write_issue_query<D: Issues>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
    body: Bytes,
) -> Response {
    let write: ipc::WriteIssueQuery = match ipc::decode("words for an issue query", &body) {
        Ok(write) => write,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().write_issue_query(ManifestId::carried(repository), write).await {
        Ok(written) => answer(StatusCode::OK, &written, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn save_issue_query<D: Issues>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
    body: Bytes,
) -> Response {
    let save: ipc::SaveIssueQuery = match ipc::decode("an issue query to save", &body) {
        Ok(save) => save,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().save_issue_query(ManifestId::carried(repository), save).await {
        Ok(saved) => answer(StatusCode::OK, &saved, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn forget_issue_query<D: Issues>(
    State(served): State<Served<D>>,
    Path((repository, id)): Path<(String, String)>,
) -> Response {
    match served.daemon().forget_issue_query(ManifestId::carried(repository), id).await {
        Ok(left) => answer(StatusCode::OK, &left, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The issues route and Autopilot's, as one router the surface merges.
pub(crate) fn mounted<D: Issues + Autopilots>() -> Router<Served<D>> {
    Router::new()
        .route("/issues/:repository", get(list_issues::<D>))
        .route("/issues/:repository/:reference", get(get_issue::<D>))
        .route("/issue_queries/:repository", get(list_issue_queries::<D>).post(save_issue_query::<D>))
        .route("/issue_queries/:repository/search", post(search_issues::<D>))
        .route("/issue_queries/:repository/install_tool", post(install_tool::<D>))
        .route("/issue_queries/:repository/write", post(write_issue_query::<D>))
        .route("/issue_queries/:repository/:id/forget", post(forget_issue_query::<D>))
        .route("/autopilots", get(ap::list_autopilots::<D>))
        .route(
            "/autopilots/:repository",
            get(ap::get_autopilot::<D>).post(ap::start_autopilot::<D>),
        )
        .route(
            "/autopilots/:repository/proposal",
            post(ap::propose_autopilot::<D>),
        )
        .route(
            "/autopilots/:repository/settings",
            post(ap::update_autopilot::<D>),
        )
        .route(
            "/autopilots/:repository/pause",
            post(ap::pause_autopilot::<D>),
        )
        .route(
            "/autopilots/:repository/resume",
            post(ap::resume_autopilot::<D>),
        )
        .route(
            "/autopilots/:repository/stop",
            post(ap::stop_autopilot::<D>),
        )
        .route(
            "/autopilots/:repository/picks/:pick_id/dispatch",
            post(ap::dispatch_set_aside::<D>),
        )
        .route(
            "/autopilots/:repository/picks/:pick_id/dismiss",
            post(ap::dismiss_set_aside::<D>),
        )
}
