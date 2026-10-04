//! The two acts on a Job's walk notes: what a person pointed at while walking
//! its served mock, and taking one back. Since 23.16.
//!
//! **Beside `commands` rather than in it**, which is at the size the gate asks
//! about. Both answer with every note on the Job, so Bridge redraws the list
//! from the reply rather than from a second read.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use ipc::{CaptureWalkNote, RemoveWalkNote};

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Commands;
use crate::reference::Resolved;
use crate::served::Served;

/// A person points at something in the walk window and says what is wrong.
///
/// 422 on a blank `said` and on a frame Fleet would not keep. 409 on a
/// terminal Job.
pub(crate) async fn capture_walk_note<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
    body: Bytes,
) -> Response {
    let capture: CaptureWalkNote = match ipc::decode("a walk note", &body) {
        Ok(capture) => capture,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().capture_walk_note(job.id(), capture).await {
        Ok(notes) => answer(StatusCode::OK, &notes, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A walk note taken back before any Drone was handed it.
///
/// 409 on a sent note and on a terminal Job. 422 on an id the Job has no note
/// by.
pub(crate) async fn remove_walk_note<D: Commands>(
    State(served): State<Served<D>>,
    job: Resolved,
    body: Bytes,
) -> Response {
    let remove: RemoveWalkNote = match ipc::decode("a walk note to remove", &body) {
        Ok(remove) => remove,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().remove_walk_note(job.id(), remove).await {
        Ok(notes) => answer(StatusCode::OK, &notes, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
