//! What Fleet remembers of a pull request for the person, the change since he looked, and his
//! named Reviews layouts: Bridge's eight calls. `docs/concepts/pull-review.md`, *Since you looked*.
//! Their own file so `routes.rs` stays under the line count the gate reports, and read with it by
//! `xtask`'s protocol rule as `ROUTER_PARTS`.

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use ipc::ManifestId;
use serde::de::DeserializeOwned;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::PullRequestPanel;
use crate::served::Served;

/// The marks, the compare and the layouts, as one router the surface merges.
pub(crate) fn mounted<D: PullRequestPanel>() -> Router<Served<D>> {
    Router::new()
        .route("/pull_request_seen/:repository", post(mark_pull_request_seen::<D>))
        .route("/pull_request_viewed/:repository", post(mark_file_viewed::<D>))
        .route("/pull_request_viewed/:repository/unmark", post(unmark_file_viewed::<D>))
        .route(
            "/pull_request_compares/:repository/:number",
            get(get_pull_request_compare::<D>),
        )
        .route(
            "/review_layouts",
            get(list_review_layouts::<D>).post(save_review_layout::<D>),
        )
        .route("/review_layouts/delete", post(delete_review_layout::<D>))
        .route("/review_layouts/pick", post(pick_review_layout::<D>))
}

/// The body, or the 400 that says why it is not one.
fn body_of<T: DeserializeOwned>(expected: &'static str, body: &Bytes, run_id: &ipc::RunId) -> Result<T, Response> {
    ipc::decode_body(expected, body).map_err(|why| undecodable(&why.to_string(), run_id))
}

/// 422 for a head that is not a commit.
pub(crate) async fn mark_pull_request_seen<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
    body: Bytes,
) -> Response {
    let mark = match body_of("a pull request seen at a head", &body, served.run_id()) {
        Ok(mark) => mark,
        Err(said) => return said,
    };
    match served.daemon().mark_pull_request_seen(ManifestId::carried(repository), mark).await {
        Ok(seen) => answer(StatusCode::OK, &seen, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 422 for a blank path or a commit that is not one.
pub(crate) async fn mark_file_viewed<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
    body: Bytes,
) -> Response {
    let mark = match body_of("a file marked viewed", &body, served.run_id()) {
        Ok(mark) => mark,
        Err(said) => return said,
    };
    match served.daemon().mark_file_viewed(ManifestId::carried(repository), mark).await {
        Ok(viewed) => answer(StatusCode::OK, &viewed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn unmark_file_viewed<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
    body: Bytes,
) -> Response {
    let unmark = match body_of("a file to unmark", &body, served.run_id()) {
        Ok(unmark) => unmark,
        Err(said) => return said,
    };
    match served.daemon().unmark_file_viewed(ManifestId::carried(repository), unmark).await {
        Ok(unmarked) => answer(StatusCode::OK, &unmarked, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A lost base is an answer, `base_gone`; 422 for a name that is not a commit, 500 where the forge
/// would not compare.
pub(crate) async fn get_pull_request_compare<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path((repository, number)): Path<(String, u64)>,
    Query(of): Query<ipc::PullRequestCompareOf>,
) -> Response {
    match served.daemon().get_pull_request_compare(ManifestId::carried(repository), number, of).await {
        Ok(compared) => answer(StatusCode::OK, &compared, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn list_review_layouts<D: PullRequestPanel>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_review_layouts().await {
        Ok(layouts) => answer(StatusCode::OK, &layouts, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 422 for a blank or Default's name, and an arrangement that is not a JSON object.
pub(crate) async fn save_review_layout<D: PullRequestPanel>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let save = match body_of("a review layout to save", &body, served.run_id()) {
        Ok(save) => save,
        Err(said) => return said,
    };
    match served.daemon().save_review_layout(save).await {
        Ok(layouts) => answer(StatusCode::OK, &layouts, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 422 for Default, which cannot be deleted.
pub(crate) async fn delete_review_layout<D: PullRequestPanel>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let named = match body_of("a review layout to delete", &body, served.run_id()) {
        Ok(named) => named,
        Err(said) => return said,
    };
    match served.daemon().delete_review_layout(named).await {
        Ok(layouts) => answer(StatusCode::OK, &layouts, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 422 for a name no layout has.
pub(crate) async fn pick_review_layout<D: PullRequestPanel>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let named = match body_of("a review layout to pick", &body, served.run_id()) {
        Ok(named) => named,
        Err(said) => return said,
    };
    match served.daemon().pick_review_layout(named).await {
        Ok(layouts) => answer(StatusCode::OK, &layouts, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
