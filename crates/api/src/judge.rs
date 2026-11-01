//! A Judge's four routes: ask one of an artifact, and send, dismiss or open what it found. The artifact
//! is in the body, never the path, because every kind of artifact of a Job is one.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{JudgeReadings, Refusal};
use crate::served::Served;

fn answered(done: Result<ipc::JudgeReading, Refusal>, served_run: &ipc::RunId) -> Response {
    match done {
        Ok(reading) => answer(StatusCode::OK, &reading, served_run),
        Err(refusal) => refused(refusal),
    }
}

/// The person presses Judge. 200 with the reading, going or already going; 404 for no such Job; 409 for a Job that ended.
pub(crate) async fn ask_judge<D: JudgeReadings>(State(served): State<Served<D>>, body: Bytes) -> Response {
    match ipc::decode_body::<ipc::AskJudge>("an artifact to judge", &body) {
        Ok(ask) => answered(served.shared().ask_judge(ask).await, served.run_id()),
        Err(why) => undecodable(&why.to_string(), served.run_id()),
    }
}

/// The person sends a note to the agent as his message. 200 with the reading; 404 for a reading or note it does not hold.
pub(crate) async fn send_judge_note<D: JudgeReadings>(State(served): State<Served<D>>, body: Bytes) -> Response {
    match ipc::decode_body::<ipc::SendJudgeNote>("a note to send", &body) {
        Ok(send) => answered(served.shared().send_judge_note(send).await, served.run_id()),
        Err(why) => undecodable(&why.to_string(), served.run_id()),
    }
}

/// The person dismisses a note; the agent is told nothing. 200 with the reading.
pub(crate) async fn dismiss_judge_note<D: JudgeReadings>(State(served): State<Served<D>>, body: Bytes) -> Response {
    match ipc::decode_body::<ipc::DismissJudgeNote>("a note to dismiss", &body) {
        Ok(dismiss) => answered(served.shared().dismiss_judge_note(dismiss).await, served.run_id()),
        Err(why) => undecodable(&why.to_string(), served.run_id()),
    }
}

/// The person opened a reading's notes. 200 with the reading, `read` set.
pub(crate) async fn mark_judge_read<D: JudgeReadings>(State(served): State<Served<D>>, body: Bytes) -> Response {
    match ipc::decode_body::<ipc::MarkJudgeRead>("a reading to mark read", &body) {
        Ok(read) => answered(served.shared().mark_judge_read(read).await, served.run_id()),
        Err(why) => undecodable(&why.to_string(), served.run_id()),
    }
}
