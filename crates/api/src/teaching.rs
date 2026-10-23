//! Teaching's reads and acts: a person's selected words read into a proposal, and the link rules
//! they keep from it.

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Teaching;
use crate::served::Served;

/// `?repository=` on `list_taught_links`.
#[derive(Deserialize)]
pub(crate) struct Listing {
    #[serde(default)]
    repository: Option<String>,
}

/// The rule an act names.
#[derive(Deserialize)]
pub(crate) struct NamedLink {
    link_id: String,
}

/// Read what a person taught into a proposal. Nothing is kept.
pub(crate) async fn read_teaching<D: Teaching>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let teach: ipc::ReadTeaching = match ipc::decode("words to teach", &body) {
        Ok(teach) => teach,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().read_teaching(teach).await {
        Ok(proposal) => answer(StatusCode::OK, &proposal, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Keep a link rule, and tell a live Session what it was taught.
pub(crate) async fn save_teaching<D: Teaching>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let save: ipc::SaveTeaching = match ipc::decode("a teaching to keep", &body) {
        Ok(save) => save,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().save_teaching(save).await {
        Ok(saved) => answer(StatusCode::OK, &saved, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The rules for a repository and for every one.
pub(crate) async fn list_taught_links<D: Teaching>(State(served): State<Served<D>>, Query(listing): Query<Listing>) -> Response {
    match served.daemon().list_taught_links(listing.repository).await {
        Ok(links) => answer(StatusCode::OK, &links, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Forget a rule for good.
pub(crate) async fn forget_taught_link<D: Teaching>(
    State(served): State<Served<D>>,
    Path(NamedLink { link_id }): Path<NamedLink>,
) -> Response {
    match served.daemon().forget_taught_link(link_id).await {
        Ok(forgotten) => answer(StatusCode::OK, &forgotten, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
