//! `get_pull_request`, the three acts on one and `review_pull_request`. Since
//! 23.48. `docs/concepts/session.md`, *Acts on a pull request*.

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::Extension;
use ipc::ManifestId;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{PullRequests, Redirector};
use crate::door::HelmCalled;
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
        .daemon()
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
        .daemon()
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
        .daemon()
        .enable_auto_merge(ManifestId::carried(repository), number)
        .await
    {
        Ok(state) => answer(StatusCode::OK, &state, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A Code Review Job at the approval gate. 201: the Job exists.
pub(crate) async fn review_pull_request<D: PullRequests>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
    helm: Option<Extension<HelmCalled>>,
    body: Bytes,
) -> Response {
    let review: ipc::ReviewPullRequest = match ipc::decode("a pull request to review", &body) {
        Ok(review) => review,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    let by = match helm {
        Some(_) => Redirector::Helm,
        None => Redirector::Person,
    };
    match served
        .daemon()
        .review_pull_request(ManifestId::carried(repository), review, by)
        .await
    {
        Ok(dispatched) => answer(StatusCode::CREATED, &dispatched, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
