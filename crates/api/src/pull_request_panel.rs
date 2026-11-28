//! `get_pull_request_detail`, `get_pull_request_file`, `get_pull_request_patches`,
//! `get_pull_request_commit_patches` and `write_pull_request`. Since 23.83; the file read since 23.87. `get_review_lists` and `resolve_review_thread` beside them.

use std::net::SocketAddr;

use axum::body::Bytes;
use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use ipc::ManifestId;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Admitting, PullRequestPanel};
use crate::mcp::Caller;
use crate::served::Served;

/// **A page answered to a caller no Job holds is him opening the pull request**, since Bridge reads it
/// when he does; a Job reading the pull request it reviews says nothing of whether he has.
pub(crate) async fn get_pull_request_detail<D: PullRequestPanel + Admitting>(
    State(served): State<Served<D>>,
    peer: Option<ConnectInfo<SocketAddr>>,
    Path((repository, number)): Path<(String, u64)>,
) -> Response {
    let manifest_id = ManifestId::carried(repository);
    match served.daemon().get_pull_request_detail(manifest_id.clone(), number).await {
        Ok(page) => {
            let by_a_job = match peer {
                Some(ConnectInfo(peer)) => served.daemon().job_at(Caller::at(peer)).await,
                None => false,
            };
            if !by_a_job {
                served.daemon().pull_request_opened(manifest_id, number).await;
            }
            answer(StatusCode::OK, &page, served.run_id())
        }
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

pub(crate) async fn get_pull_request_commit_patches<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path((repository, number, commit)): Path<(String, u64, String)>,
) -> Response {
    match served
        .daemon()
        .get_pull_request_commit_patches(ManifestId::carried(repository), number, commit)
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

/// `?project=` on `GET /review_lists`: the lists keep only that project's members.
#[derive(serde::Deserialize)]
pub(crate) struct InProject {
    #[serde(default)]
    project: Option<String>,
}

pub(crate) async fn get_review_lists<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Query(scope): Query<InProject>,
) -> Response {
    match served.shared().get_review_lists(scope.project).await {
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
