//! `get_pull_request`, the three acts on one, its refresh and `review_pull_request`. Since
//! 23.48. `docs/concepts/session.md`, *Acts on a pull request*.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use ipc::ManifestId;

use crate::answers::{answer, refused};
use crate::daemon::PullRequests;
use crate::served::Served;

pub(crate) async fn get_pull_request<D: PullRequests>(
    State(served): State<Served<D>>,
    Path((repository, number)): Path<(String, u64)>,
) -> Response {
    match served
        .daemon()
        .get_pull_request(ManifestId::carried(repository), number)
        .await
    {
        Ok(state) => answer(StatusCode::OK, &state, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Draft to ready. 409 where it is not a draft, 422 for a number nothing names.
pub(crate) async fn ready_pull_request<D: PullRequests>(
    State(served): State<Served<D>>,
    Path((repository, number)): Path<(String, u64)>,
) -> Response {
    match served
        .shared()
        .ready_pull_request(ManifestId::carried(repository), number)
        .await
    {
        Ok(state) => answer(StatusCode::OK, &state, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Merge it. 409 with `fleet.merge_checks_not_passed` while the checks run or
/// have failed, and with the forge's own kind where it refused.
pub(crate) async fn merge_pull_request_by_number<D: PullRequests>(
    State(served): State<Served<D>>,
    Path((repository, number)): Path<(String, u64)>,
) -> Response {
    match served
        .shared()
        .merge_pull_request_by_number(ManifestId::carried(repository), number)
        .await
    {
        Ok(state) => answer(StatusCode::OK, &state, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Ask the forge to merge it when the checks pass. 409 once they have.
pub(crate) async fn enable_auto_merge<D: PullRequests>(
    State(served): State<Served<D>>,
    Path((repository, number)): Path<(String, u64)>,
) -> Response {
    match served
        .shared()
        .enable_auto_merge(ManifestId::carried(repository), number)
        .await
    {
        Ok(state) => answer(StatusCode::OK, &state, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Read it now and do what the pull reading would. Bridge's press, never an agent's.
pub(crate) async fn refresh_pull_request<D: PullRequests>(
    State(served): State<Served<D>>,
    Path((repository, number)): Path<(String, u64)>,
) -> Response {
    match served
        .shared()
        .refresh_pull_request(ManifestId::carried(repository), number)
        .await
    {
        Ok(state) => answer(StatusCode::OK, &state, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
