//! The owner's notes on a pull request's lines: Bridge's four calls on them. A Job resolving one is
//! `sessions::resolve_pull_request_note`'s, since the Job is placed by its connection.

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use ipc::ManifestId;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::PullRequestPanel;
use crate::served::Served;

pub(crate) async fn list_pull_request_notes<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path((repository, number)): Path<(String, u32)>,
) -> Response {
    match served.daemon().list_pull_request_notes(ManifestId::carried(repository), number).await {
        Ok(notes) => answer(StatusCode::OK, &notes, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 422 for a blank body, a note missing where it is, or one that can no longer be edited.
pub(crate) async fn save_pull_request_note<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path((repository, number)): Path<(String, u32)>,
    body: Bytes,
) -> Response {
    let save: ipc::SavePullRequestNote = match ipc::decode_body("a pull request note to save", &body) {
        Ok(save) => save,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().save_pull_request_note(ManifestId::carried(repository), number, save).await {
        Ok(note) => answer(StatusCode::OK, &note, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn remove_pull_request_note<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path((repository, number, id)): Path<(String, u32, String)>,
) -> Response {
    match served.daemon().remove_pull_request_note(ManifestId::carried(repository), number, id).await {
        Ok(notes) => answer(StatusCode::OK, &notes, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 422 for an id that names no note on the pull request.
pub(crate) async fn send_pull_request_notes<D: PullRequestPanel>(
    State(served): State<Served<D>>,
    Path((repository, number)): Path<(String, u32)>,
    body: Bytes,
) -> Response {
    let send: ipc::SendPullRequestNotes = match ipc::decode_body("notes sent to a job", &body) {
        Ok(send) => send,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().send_pull_request_notes(ManifestId::carried(repository), number, send).await {
        Ok(notes) => answer(StatusCode::OK, &notes, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
