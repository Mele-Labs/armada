//! Natural Search and the Walk tool: the routes beside a pull request's runs. `docs/concepts/
//! pull-review.md`. Their own file so `routes.rs` stays under the line count the gate reports, and
//! read with it by `xtask`'s protocol rule as `ROUTER_PARTS`.

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use ipc::ManifestId;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::PullRequestPanel;
use crate::served::Served;

/// Search, remembered places and held walks, as one router the surface merges.
pub(crate) fn mounted<D: PullRequestPanel>() -> Router<Served<D>> {
    Router::new()
        .route(
            "/review_searches",
            get(get_review_searches::<D>).post(start_review_search::<D>),
        )
        .route("/review_searches/forget", post(forget_review_search::<D>))
        .route("/walk_finds/:repository/:number", get(find_walk_place::<D>))
        .route("/walk_places", get(list_walk_places::<D>))
        .route("/walk_places/:repository", post(save_walk_place::<D>))
        .route(
            "/walk_places/:repository/:id/forget",
            post(forget_walk_place::<D>),
        )
        .route("/review_walks/:repository", post(open_review_walk::<D>))
        .route(
            "/review_walks/:repository/:id/close",
            post(close_review_walk::<D>),
        )
}

/// 422 for blank or over-long words; 500 where the forge would not list the open pull requests.
pub(crate) async fn start_review_search<D: PullRequestPanel>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let start: ipc::StartReviewSearch = match ipc::decode_body("a review search to start", &body) {
        Ok(start) => start,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().start_review_search(start).await {
        Ok(run) => answer(StatusCode::OK, &run, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_review_searches<D: PullRequestPanel>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_review_searches().await {
        Ok(runs) => answer(StatusCode::OK, &runs, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn forget_review_search<D: PullRequestPanel>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let forget: ipc::ForgetReviewSearch = match ipc::decode_body("a review search to forget", &body) {
        Ok(forget) => forget,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().forget_review_search(forget).await {
        Ok(runs) => answer(StatusCode::OK, &runs, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 422 for a repository Fleet does not serve; 500 where the forge would not give the pull request.
pub(crate) async fn find_walk_place<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path((repository, number)): Path<(String, u64)>,
) -> Response {
    match served
        .daemon()
        .find_walk_place(ManifestId::carried(repository), number)
        .await
    {
        Ok(found) => answer(StatusCode::OK, &found, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn list_walk_places<D: PullRequestPanel>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_walk_places().await {
        Ok(places) => answer(StatusCode::OK, &places, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 422 for a place that cannot be walked or an area that is not a directory of the repository.
pub(crate) async fn save_walk_place<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
    body: Bytes,
) -> Response {
    let save: ipc::SaveWalkPlace = match ipc::decode_body("a walk place to save", &body) {
        Ok(save) => save,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .daemon()
        .save_walk_place(ManifestId::carried(repository), save)
        .await
    {
        Ok(place) => answer(StatusCode::OK, &place, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn forget_walk_place<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path((repository, id)): Path<(String, String)>,
) -> Response {
    match served
        .daemon()
        .forget_walk_place(ManifestId::carried(repository), id)
        .await
    {
        Ok(places) => answer(StatusCode::OK, &places, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 422 for a place that cannot be walked; 500 where the forge would not give the head to check out.
pub(crate) async fn open_review_walk<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
    body: Bytes,
) -> Response {
    let open: ipc::OpenReviewWalk = match ipc::decode_body("a review walk to open", &body) {
        Ok(open) => open,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .shared()
        .open_review_walk(ManifestId::carried(repository), open)
        .await
    {
        Ok(walk) => answer(StatusCode::OK, &walk, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn close_review_walk<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path((repository, id)): Path<(String, String)>,
) -> Response {
    match served
        .daemon()
        .close_review_walk(ManifestId::carried(repository), id)
        .await
    {
        Ok(walk) => answer(StatusCode::OK, &walk, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
