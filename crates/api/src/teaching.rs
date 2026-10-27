//! Teaching's reads and acts: something a person pressed Teach on read into a proposal, the Logbook
//! entry and mods they keep from it, and the patterns Bridge links.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Teaching;
use crate::served::Served;

/// `?repository=&session_id=` on `list_word_rules`.
#[derive(Deserialize)]
pub(crate) struct Listing {
    #[serde(default)]
    repository: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
}

/// Read what a person pressed Teach on into a proposal. Nothing is kept.
pub(crate) async fn read_teaching<D: Teaching>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let teach: ipc::ReadTeaching = match ipc::decode_body("something to teach", &body) {
        Ok(teach) => teach,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().read_teaching(teach).await {
        Ok(proposal) => answer(StatusCode::OK, &proposal, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Keep a teaching: the Logbook entry for its note and the mods it changes, and tell a live Session.
pub(crate) async fn save_teaching<D: Teaching>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let save: ipc::SaveTeaching = match ipc::decode_body("a teaching to keep", &body) {
        Ok(save) => save,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().save_teaching(save).await {
        Ok(saved) => answer(StatusCode::OK, &saved, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The patterns that have an address for a repository.
pub(crate) async fn list_word_rules<D: Teaching>(State(served): State<Served<D>>, Query(listing): Query<Listing>) -> Response {
    match served.shared().list_word_rules(listing.repository, listing.session_id).await {
        Ok(rules) => answer(StatusCode::OK, &rules, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
