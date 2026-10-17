//! The three exits from a pilot. Since 23.50.
//! `docs/concepts/pilot.md`.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Piloting;
use crate::reference::Resolved;
use crate::served::Served;

pub(crate) async fn submit_for_verification<D: Piloting>(
    State(served): State<Served<D>>,
    job: Resolved,
) -> Response {
    match served.shared().submit_for_verification(job.id()).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn attest_complete<D: Piloting>(
    State(served): State<Served<D>>,
    job: Resolved,
    body: Bytes,
) -> Response {
    let note = match noted(&body) {
        Ok(note) => note,
        Err(why) => return undecodable(&why, served.run_id()),
    };
    match served.shared().attest_complete(job.id(), note).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn close_as_superseded<D: Piloting>(
    State(served): State<Served<D>>,
    job: Resolved,
    body: Bytes,
) -> Response {
    let note = match noted(&body) {
        Ok(note) => note,
        Err(why) => return undecodable(&why, served.run_id()),
    };
    match served.shared().close_as_superseded(job.id(), note).await {
        Ok(job) => answer(StatusCode::OK, &job, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The person's words, where they gave any. An empty body gave none.
fn noted(body: &Bytes) -> Result<ipc::PilotNote, String> {
    if body.is_empty() {
        return Ok(ipc::PilotNote::default());
    }
    ipc::decode("a note", body).map_err(|why| why.to_string())
}
