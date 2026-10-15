//! `get_pull_request_detail`, `get_pull_request_file` and `write_pull_request`. Since 23.83;
//! the file read since 23.87.

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

/// 422 for a comment or a request for changes with no body, and for a Session
/// the ledger does not know.
pub(crate) async fn write_pull_request<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
    body: Bytes,
) -> Response {
    let write: ipc::WritePullRequest = match ipc::decode("a write on a pull request", &body) {
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
