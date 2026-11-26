//! A Judge's four routes: ask one of an artifact, and send, dismiss or open what it found. The artifact
//! is in the body, never the path, because every kind of artifact of a Job is one. The judge types an ask
//! reads as are listed, saved and deleted beside them.

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use ipc::ManifestId;
use serde::Deserialize;

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

#[derive(Deserialize)]
pub(crate) struct InProject {
    project: Option<String>,
}

fn types_answered(done: Result<ipc::JudgeTypes, Refusal>, served_run: &ipc::RunId) -> Response {
    match done {
        Ok(types) => answer(StatusCode::OK, &types, served_run),
        Err(refusal) => refused(refusal),
    }
}

/// The types a repository's project sees. 422 for a repository Fleet does not serve.
pub(crate) async fn list_judge_types<D: JudgeReadings>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
    Query(scope): Query<InProject>,
) -> Response {
    types_answered(served.shared().list_judge_types(Some(ManifestId::carried(repository)), scope.project).await, served.run_id())
}

/// The types a Job in a folder with no Manifest sees: Armada's and Kit's.
pub(crate) async fn list_judge_types_unplaced<D: JudgeReadings>(
    State(served): State<Served<D>>,
    Query(scope): Query<InProject>,
) -> Response {
    types_answered(served.shared().list_judge_types(None, scope.project).await, served.run_id())
}

/// 422 for a blank or wordless name, a blank prompt, an id the place already defines, and a project named
/// with the repository's scope.
pub(crate) async fn save_judge_type<D: JudgeReadings>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
    body: Bytes,
) -> Response {
    match ipc::decode_body::<ipc::SaveJudgeType>("a judge type to save", &body) {
        Ok(save) => types_answered(served.shared().save_judge_type(ManifestId::carried(repository), save).await, served.run_id()),
        Err(why) => undecodable(&why.to_string(), served.run_id()),
    }
}

/// 422 where the place holds no type of that id.
pub(crate) async fn delete_judge_type<D: JudgeReadings>(
    State(served): State<Served<D>>,
    Path(repository): Path<String>,
    body: Bytes,
) -> Response {
    match ipc::decode_body::<ipc::DeleteJudgeType>("a judge type to delete", &body) {
        Ok(delete) => types_answered(served.shared().delete_judge_type(ManifestId::carried(repository), delete).await, served.run_id()),
        Err(why) => undecodable(&why.to_string(), served.run_id()),
    }
}
