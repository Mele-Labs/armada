//! `get_pull_request_detail`, `get_pull_request_file`, `get_pull_request_patches` and
//! `write_pull_request`. Since 23.83; the file read since 23.87. `get_review_lists` and `resolve_review_thread` beside them.

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use ipc::ManifestId;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::PullRequestPanel;
use crate::served::Served;

pub(crate) async fn get_pull_request_detail<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path((repository, number)): Path<(String, u64)>,
) -> Response {
    match served
        .daemon()
        .get_pull_request_detail(ManifestId::carried(repository), number)
        .await
    {
        Ok(page) => answer(StatusCode::OK, &page, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_pull_request_file<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path((repository, number)): Path<(String, u64)>,
    Query(of): Query<ipc::PullRequestFileOf>,
) -> Response {
    match served
        .daemon()
        .get_pull_request_file(ManifestId::carried(repository), number, of.path)
        .await
    {
        Ok(file) => answer(StatusCode::OK, &file, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_pull_request_patches<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path((repository, number)): Path<(String, u64)>,
) -> Response {
    match served
        .daemon()
        .get_pull_request_patches(ManifestId::carried(repository), number)
        .await
    {
        Ok(files) => answer(StatusCode::OK, &files, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 422 for a comment or a request for changes with no body, and for a Session
/// the ledger does not know.
pub(crate) async fn write_pull_request<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
    body: Bytes,
) -> Response {
    let write: ipc::WritePullRequest = match ipc::decode_body("a write on a pull request", &body) {
        Ok(write) => write,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .write_pull_request(ManifestId::carried(repository), write)
        .await
    {
        Ok(page) => answer(StatusCode::OK, &page, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_review_lists<D: PullRequestPanel>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_review_lists().await {
        Ok(lists) => answer(StatusCode::OK, &lists, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 422 for a repository Fleet does not serve; 409 with the forge's sentence where it would not
/// take the act.
pub(crate) async fn resolve_review_thread<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
    body: Bytes,
) -> Response {
    let resolve: ipc::ResolvePullThread = match ipc::decode_body("a resolve on a review thread", &body) {
        Ok(resolve) => resolve,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .resolve_review_thread(ManifestId::carried(repository), resolve)
        .await
    {
        Ok(resolved) => answer(StatusCode::OK, &resolved, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn list_review_tools<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
) -> Response {
    match served.daemon().list_review_tools(ManifestId::carried(repository)).await {
        Ok(tools) => answer(StatusCode::OK, &tools, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 422 for a blank or wordless name, a blank prompt, and an id the scope already defines.
pub(crate) async fn save_review_tool<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
    body: Bytes,
) -> Response {
    let save: ipc::SaveReviewTool = match ipc::decode_body("a review tool to save", &body) {
        Ok(save) => save,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .save_review_tool(ManifestId::carried(repository), save)
        .await
    {
        Ok(tools) => answer(StatusCode::OK, &tools, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 422 for a name no definition has or a blank question; 500 where the forge would not give the
/// pull request.
pub(crate) async fn start_review_run<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
    body: Bytes,
) -> Response {
    let start: ipc::StartReviewRun = match ipc::decode_body("a review run to start", &body) {
        Ok(start) => start,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .shared()
        .start_review_run(ManifestId::carried(repository), start)
        .await
    {
        Ok(run) => answer(StatusCode::OK, &run, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_review_runs<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path((repository, number)): Path<(String, u64)>,
) -> Response {
    match served
        .daemon()
        .get_review_runs(ManifestId::carried(repository), number)
        .await
    {
        Ok(runs) => answer(StatusCode::OK, &runs, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
