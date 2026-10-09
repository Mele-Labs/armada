//! The phone's Session routes and one-line dispatch: each a Fleet call whose
//! body is built here field by field from a body that denies unknown fields.

use std::collections::BTreeSet;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ipc::{
    AnswerSessionAsk, HelmCallAnswer, JobRequest, ManifestSummary, ProposedPlan, QuestionAnswer,
    SessionList, SessionOrigin, SessionState, WireError,
};
use serde::{Deserialize, Serialize};

use crate::phone::PhoneJob;
use crate::phone_sessions::{held_ask, PhoneSession};
use crate::reads::{json, port, refusal, repositories, required, NOT_ANSWERING, NOT_READABLE};
use crate::{fleet_client, Gateway};

const NO_SUCH_SESSION: &str = "That Session is not here.";
const NO_LONGER_HELD: &str = "Armada is no longer holding that question. Answer it in the terminal.";
const NOT_ASKING: &str = "That Session is not waiting on an answer.";
const ASK_MOVED_ON: &str = "That question has been answered or has changed. Open it again.";
const ANSWER_DOES_NOT_FIT: &str = "That answer does not fit the question. Open it again.";
const UNREADABLE_REQUEST: &str = "Armada could not read that from this phone. Update the app.";
const NO_SUCH_REPOSITORY: &str = "Armada does not know that repository. Pick one from the list.";

pub async fn sessions(State(gateway): State<Gateway>) -> Response {
    let list = match required::<SessionList>(&gateway, "/sessions?state=live", "a session list").await {
        Ok(list) => list.sessions,
        Err(refused) => return refused,
    };
    let labels = repositories(&gateway).await;
    let rows: Vec<PhoneSession> = list
        .iter()
        .filter(|record| record.state == SessionState::Live)
        .map(|record| {
            let repository = record
                .manifest_id
                .as_ref()
                .and_then(|id| labels.get(id.as_str()))
                .map(String::as_str);
            PhoneSession::of(record, repository, gateway.pairing.now())
        })
        .collect();
    json(&Sessions { sessions: rows })
}

#[derive(Serialize)]
struct Sessions {
    sessions: Vec<PhoneSession>,
}

/// Fleet's refusal as a sentence the phone shows: its status and its message,
/// and not its cause chain, fields or ids.
fn passed(answer: &fleet_client::Answer) -> Response {
    let status = StatusCode::from_u16(answer.status).unwrap_or(StatusCode::BAD_GATEWAY);
    if !status.is_client_error() {
        return refusal(StatusCode::BAD_GATEWAY, NOT_ANSWERING);
    }
    match ipc::decode::<WireError>("a refusal", &answer.body) {
        Ok(error) => (status, error.message).into_response(),
        Err(_) => refusal(status, NOT_ANSWERING),
    }
}

async fn post(gateway: &Gateway, path: &str, body: String) -> Result<fleet_client::Answer, Response> {
    let port = port(gateway)?;
    fleet_client::send(port, "POST", path, Some(body.as_bytes()))
        .await
        .map_err(|_| refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_ANSWERING))
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Decision {
    AllowOnce,
    Refuse,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Chosen {
    question: String,
    chosen: Vec<String>,
}

/// A decision for a permission ask, or the choices for a question.
#[derive(Deserialize)]
#[serde(untagged)]
enum Said {
    Decision(Decision),
    Chosen(Vec<Chosen>),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Answering {
    session_id: String,
    #[serde(default)]
    ask_id: Option<String>,
    answer: Said,
}

pub async fn answer(State(gateway): State<Gateway>, body: Bytes) -> Response {
    let Ok(answering) = ipc::decode::<Answering>("an answer", &body) else {
        return refusal(StatusCode::BAD_REQUEST, UNREADABLE_REQUEST);
    };
    // Whether it is hosted is Fleet's word, never the phone's.
    let list = match required::<SessionList>(&gateway, "/sessions?state=live", "a session list").await {
        Ok(list) => list.sessions,
        Err(refused) => return refused,
    };
    let Some(record) = list.iter().find(|record| record.id.as_str() == answering.session_id) else {
        return refusal(StatusCode::NOT_FOUND, NO_SUCH_SESSION);
    };
    let Some(held) = held_ask(record, gateway.pairing.now()) else {
        let gone = if record.origin == SessionOrigin::Terminal { NO_LONGER_HELD } else { NOT_ASKING };
        return refusal(StatusCode::CONFLICT, gone);
    };
    if answering.ask_id.as_deref().is_some_and(|named| named != held.call) {
        return refusal(StatusCode::CONFLICT, ASK_MOVED_ON);
    }
    let (decision, answers) = match (answering.answer, held.questions.is_empty()) {
        (Said::Decision(Decision::AllowOnce), true) => (HelmCallAnswer::AllowOnce, Vec::new()),
        (Said::Decision(Decision::Refuse), _) => (HelmCallAnswer::Refuse, Vec::new()),
        (Said::Chosen(chosen), false) => (
            HelmCallAnswer::AllowOnce,
            chosen
                .into_iter()
                .map(|one| QuestionAnswer { question: one.question, chosen: one.chosen })
                .collect(),
        ),
        _ => return refusal(StatusCode::BAD_REQUEST, ANSWER_DOES_NOT_FIT),
    };
    let sent = AnswerSessionAsk {
        session_id: record.id.clone(),
        call: held.call.clone(),
        answer: decision,
        note: None,
        answers,
    };
    let Ok(body) = ipc::encode(&sent) else {
        return refusal(StatusCode::INTERNAL_SERVER_ERROR, NOT_READABLE);
    };
    match post(&gateway, "/sessions/ask/answer", body).await {
        Ok(answer) if answer.status == 200 => StatusCode::NO_CONTENT.into_response(),
        Ok(answer) => passed(&answer),
        Err(refused) => refused,
    }
}

async fn manifests(gateway: &Gateway) -> Result<Vec<ManifestSummary>, Response> {
    required(gateway, "/manifests", "a manifest list").await
}

/// The labels the phone may choose from, each once.
pub async fn repository_labels(State(gateway): State<Gateway>) -> Response {
    match manifests(&gateway).await {
        Ok(list) => {
            let labels: BTreeSet<String> = list.into_iter().map(|m| m.repository).collect();
            json(&labels)
        }
        Err(refused) => refused,
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Dispatching {
    text: String,
    repository: String,
}

pub async fn dispatch(State(gateway): State<Gateway>, body: Bytes) -> Response {
    let Ok(dispatching) = ipc::decode::<Dispatching>("a dispatch", &body) else {
        return refusal(StatusCode::BAD_REQUEST, UNREADABLE_REQUEST);
    };
    let list = match manifests(&gateway).await {
        Ok(list) => list,
        Err(refused) => return refused,
    };
    let mut named = list.iter().filter(|m| m.repository == dispatching.repository);
    let manifest = match (named.next(), named.next()) {
        (Some(one), None) => one,
        (None, _) => return refusal(StatusCode::BAD_REQUEST, NO_SUCH_REPOSITORY),
        (Some(_), Some(_)) => {
            return (
                StatusCode::BAD_REQUEST,
                format!(
                    "Two repositories on your Mac are called {}. Rename one, then try again.",
                    dispatching.repository
                ),
            )
                .into_response()
        }
    };
    let request = JobRequest {
        request: dispatching.text,
        client_ref: None,
        attachments: Vec::new(),
        settings: None,
    };
    let Ok(sent) = ipc::encode(&request) else {
        return refusal(StatusCode::INTERNAL_SERVER_ERROR, NOT_READABLE);
    };
    let path = format!(
        "/jobs/from_request?manifest_id={}",
        ipc::door::encoded(manifest.id.as_str())
    );
    let answer = match post(&gateway, &path, sent).await {
        Ok(answer) => answer,
        Err(refused) => return refused,
    };
    if answer.status != 201 {
        return passed(&answer);
    }
    match ipc::decode::<ProposedPlan>("a proposed plan", &answer.body) {
        Ok(plan) => {
            let jobs = plan
                .jobs
                .iter()
                .map(|job| PhoneJob::of(job, Some(manifest.repository.as_str())))
                .collect();
            let mut made = json(&Dispatched { jobs });
            *made.status_mut() = StatusCode::CREATED;
            made
        }
        Err(_) => refusal(StatusCode::BAD_GATEWAY, NOT_READABLE),
    }
}

#[derive(Serialize)]
struct Dispatched {
    jobs: Vec<PhoneJob>,
}
