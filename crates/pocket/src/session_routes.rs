//! The phone's Session routes and one-line dispatch: each a Fleet call whose
//! body is built here field by field from a body that denies unknown fields.

use std::collections::BTreeSet;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ipc::{
    AnswerSessionAsk, ForkFrom, CallAnswer, DispatchJob, DispatchKind, DispatchMessage, DispatchOn, DispatchThen, JobDispatched,
    ManifestId, ManifestSummary, QuestionAnswer, Queued, SendSessionMessage, SessionList, SessionOrigin, SessionRecord,
    SessionRow, SessionState, SessionThread, SessionVoice, StartSession, WireError,
};
use serde::{Deserialize, Serialize};

use crate::phone_sessions::{held_ask, PhoneSession};
use crate::reads::{json, port, read, refusal, repositories, required, NOT_ANSWERING, NOT_READABLE};
use crate::{fleet_client, Gateway};

const NO_SUCH_SESSION: &str = "That Job is not here.";
const NO_LONGER_HELD: &str = "Armada is no longer holding that question. Answer it in the terminal.";
const NOT_ASKING: &str = "That Job is not waiting on an answer.";
const ASK_MOVED_ON: &str = "That question has been answered or has changed. Open it again.";
const ANSWER_DOES_NOT_FIT: &str = "That answer does not fit the question. Open it again.";
const UNREADABLE_REQUEST: &str = "Armada could not read that from this phone. Update the app.";
const NO_SUCH_REPOSITORY: &str = "Armada does not know that repository. Pick one from the list.";
const NOTHING_TO_FORK_YET: &str = "That Job has not answered yet, so there is nothing to copy. Try again once it has.";
const NO_SUCH_MAC: &str = "Armada does not know that Mac. Pair it, then try again.";
const NOT_SHARED: &str = "That repository is not in a project shared between your Macs, so a Job cannot start there from here.";
const NO_REPOSITORY_TO_FORK: &str = "That Job has no repository, so Armada cannot copy it. Fork it in Armada on your Mac.";
const NOT_SHARED_HERE: &str = "That repository is in no project shared from this Mac, so a Job cannot start there from your phone.";

/// The project a Job the phone starts on this Mac goes into: the first of this Mac's shared projects holding
/// `manifest`. Fleet starts the phone's Jobs only there, so the phone sees what it started.
async fn shared_project(gateway: &Gateway, manifest: &ManifestId) -> Result<String, Response> {
    let projects = required::<ipc::Projects>(gateway, "/projects", "a project list").await?.projects;
    projects
        .into_iter()
        .find(|one| one.mac.is_none() && one.members.iter().any(|member| member.manifest_id.as_str() == manifest.as_str()))
        .map(|one| one.id)
        .ok_or_else(|| refusal(StatusCode::CONFLICT, NOT_SHARED_HERE))
}

/// The live Sessions. The phone has no list of ended ones.
pub async fn sessions(State(gateway): State<Gateway>) -> Response {
    let list = match required::<SessionList>(&gateway, "/sessions?state=live", "a job list").await {
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
    said(answer).into_response()
}

fn said(answer: &fleet_client::Answer) -> (StatusCode, String) {
    let status = StatusCode::from_u16(answer.status).unwrap_or(StatusCode::BAD_GATEWAY);
    if !status.is_client_error() {
        return (StatusCode::BAD_GATEWAY, NOT_ANSWERING.to_string());
    }
    match ipc::decode::<WireError>("a refusal", &answer.body) {
        Ok(error) => (status, error.message),
        Err(_) => (status, NOT_ANSWERING.to_string()),
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
    let list = match required::<SessionList>(&gateway, "/sessions?state=live", "a job list").await {
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
        (Said::Decision(Decision::AllowOnce), true) => (CallAnswer::AllowOnce, Vec::new()),
        (Said::Decision(Decision::Refuse), _) => (CallAnswer::Refuse, Vec::new()),
        (Said::Chosen(chosen), false) => (
            CallAnswer::AllowOnce,
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
        folder: None,
        kind: None,
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

/// The one Manifest whose repository is called `repository`, or the refusal for none or two.
fn one_manifest<'a>(list: &'a [ManifestSummary], repository: &str) -> Result<&'a ManifestSummary, Response> {
    let mut named = list.iter().filter(|m| m.repository == repository);
    match (named.next(), named.next()) {
        (Some(one), None) => Ok(one),
        (None, _) => Err(refusal(StatusCode::BAD_REQUEST, NO_SUCH_REPOSITORY)),
        (Some(_), Some(_)) => Err((
            StatusCode::BAD_REQUEST,
            format!("Two repositories on your Mac are called {repository}. Rename one, then try again."),
        )
            .into_response()),
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
    let manifest = match one_manifest(&list, &dispatching.repository) {
        Ok(one) => one,
        Err(refused) => return refused,
    };
    let project = match shared_project(&gateway, &manifest.id).await {
        Ok(project) => project,
        Err(refused) => return refused,
    };
    // `request` only names the Job and picks its workflow; the words reach the agent as its
    // first message, which Fleet holds while the proposal is read. Bridge starts the same way.
    let start = StartSession {
        request: Some(dispatching.text.clone()),
        project: Some(project),
        ..starting(manifest.id.clone())
    };
    let record = match started(&gateway, &start).await {
        Ok(record) => record,
        Err(refused) => return refused,
    };
    let message = SendSessionMessage {
        session_id: record.id.clone(),
        text: dispatching.text,
        attachments: Vec::new(),
        mentions: Vec::new(),
        sketches: Vec::new(),
    };
    let Ok(sent) = ipc::encode(&message) else {
        return refusal(StatusCode::INTERNAL_SERVER_ERROR, NOT_READABLE);
    };
    let (status, why) = match post(&gateway, "/sessions/message", sent).await {
        Ok(answer) if (200..300).contains(&answer.status) => {
            return made(&record, Some(manifest.repository.as_str()), &gateway);
        }
        Ok(answer) => said(&answer),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, NOT_ANSWERING.to_string()),
    };
    (status, format!("The Job started, but your message did not reach it. {why}")).into_response()
}

/// What the phone sends the Operator: the words, a repository, a hint, and the Mac to dispatch on.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Operating {
    text: String,
    repository: String,
    #[serde(default)]
    kind: Option<DispatchKind>,
    /// By id, or by the name the phone knows the Mac by. Absent is this Mac.
    #[serde(default)]
    mac: Option<String>,
}

/// What a dispatch comes to for the phone: `JobDispatched` with its Job trimmed as the lists trim it.
#[derive(Serialize)]
struct Handled {
    #[serde(skip_serializing_if = "Option::is_none")]
    session: Option<PhoneSession>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parked: Option<String>,
}

/// `POST /api/sessions/dispatch`: the words go to Fleet's `/sessions/dispatch` as Bridge's composer sends them,
/// on the Mac `mac` names where it names another. 200 with what the dispatch came to; 202 with the `Queued`
/// act where that Mac is asleep and the send waits for it. A Job needs a shared project, this Mac's own or the copy
/// held from another, which the Gateway finds, since the phone knows no project.
pub async fn operator_dispatch(State(gateway): State<Gateway>, body: Bytes) -> Response {
    let Ok(operating) = ipc::decode::<Operating>("a dispatch", &body) else {
        return refusal(StatusCode::BAD_REQUEST, UNREADABLE_REQUEST);
    };
    let list = match manifests(&gateway).await {
        Ok(list) => list,
        Err(refused) => return refused,
    };
    let manifest = match one_manifest(&list, &operating.repository) {
        Ok(one) => one,
        Err(refused) => return refused,
    };
    let mac = match operating.mac {
        None => None,
        Some(named) => {
            let held = gateway.clone();
            let Ok(Ok(all)) = tokio::task::spawn_blocking(move || crate::machine::macs(&held)).await else {
                return refusal(StatusCode::INTERNAL_SERVER_ERROR, NOT_READABLE);
            };
            let found = all.into_iter().find(|one| {
                let short = one.address.as_deref().and_then(crate::machine::short_name);
                one.id == named || one.name == named || short.as_deref() == Some(named.as_str())
            });
            match found {
                Some(one) if one.this_mac => None,
                Some(one) => Some(one.id),
                None => return refusal(StatusCode::NOT_FOUND, NO_SUCH_MAC),
            }
        }
    };
    let project = match &mac {
        None => match shared_project(&gateway, &manifest.id).await {
            Ok(project) => Some(project),
            Err(refused) => return refused,
        },
        Some(id) => {
            let projects = match required::<ipc::Projects>(&gateway, "/projects", "a project list").await {
                Ok(projects) => projects.projects,
                Err(refused) => return refused,
            };
            let held = projects.into_iter().find(|one| {
                one.mac.as_deref() == Some(id.as_str()) && one.members.iter().any(|member| member.manifest_id.as_str() == manifest.id.as_str())
            });
            match held {
                Some(one) => Some(one.id),
                None => return refusal(StatusCode::CONFLICT, NOT_SHARED),
            }
        }
    };
    let start = StartSession { request: Some(operating.text.clone()), project, ..starting(manifest.id.clone()) };
    let then = vec![DispatchThen::Message(DispatchMessage {
        text: operating.text,
        attachments: Vec::new(),
        mentions: Vec::new(),
        sketches: Vec::new(),
    })];
    let sent = DispatchOn { mac, job: DispatchJob { start, kind: operating.kind, then } };
    let Ok(sent) = ipc::encode(&sent) else {
        return refusal(StatusCode::INTERNAL_SERVER_ERROR, NOT_READABLE);
    };
    let answer = match post(&gateway, "/sessions/dispatch", sent).await {
        Ok(answer) => answer,
        Err(refused) => return refused,
    };
    match answer.status {
        200 => match ipc::decode::<JobDispatched>("a dispatch answer", &answer.body) {
            Ok(done) => json(&Handled {
                session: done.session.map(|record| PhoneSession::of(&record, Some(manifest.repository.as_str()), gateway.pairing.now())),
                operator: done.operator,
                parked: done.parked,
            }),
            Err(_) => refusal(StatusCode::BAD_GATEWAY, NOT_READABLE),
        },
        202 => match ipc::decode::<Queued>("a queued act", &answer.body) {
            Ok(queued) => {
                let mut kept = json(&queued);
                *kept.status_mut() = StatusCode::ACCEPTED;
                kept
            }
            Err(_) => refusal(StatusCode::BAD_GATEWAY, NOT_READABLE),
        },
        _ => passed(&answer),
    }
}

/// A `StartSession` for a Manifest with nothing else chosen.
fn starting(manifest_id: ManifestId) -> StartSession {
    StartSession {
        project: None,
        manifest_id,
        title: None,
        model: None,
        effort: None,
        mode: None,
        fork: None,
        root: None,
        request: None,
        after: None,
        held: false,
        project_rows: Vec::new(),
        on_your_words: None,
    }
}

/// Fleet's record of the Job it started, or its refusal for the phone.
async fn started(gateway: &Gateway, start: &StartSession) -> Result<SessionRecord, Response> {
    let Ok(sent) = ipc::encode(start) else {
        return Err(refusal(StatusCode::INTERNAL_SERVER_ERROR, NOT_READABLE));
    };
    let answer = post(gateway, "/sessions/start", sent).await?;
    if answer.status != 200 {
        return Err(passed(&answer));
    }
    ipc::decode::<SessionRecord>("a job", &answer.body).map_err(|_| refusal(StatusCode::BAD_GATEWAY, NOT_READABLE))
}

/// 201 with the new Job as the phone sees it.
fn made(record: &SessionRecord, repository: Option<&str>, gateway: &Gateway) -> Response {
    let session = PhoneSession::of(record, repository, gateway.pairing.now());
    let mut made = json(&Dispatched { session });
    *made.status_mut() = StatusCode::CREATED;
    made
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Forking {
    session_id: String,
}

/// A new Job copying an old one, in the old Job's own repository, as Bridge's `fork` does:
/// the whole of an ended one, a live one up to its agent's last message.
pub async fn fork(State(gateway): State<Gateway>, body: Bytes) -> Response {
    let Ok(forking) = ipc::decode::<Forking>("a fork", &body) else {
        return refusal(StatusCode::BAD_REQUEST, UNREADABLE_REQUEST);
    };
    // The id goes into a query, so nothing but an id's own characters is sent on.
    let plain = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    if forking.session_id.is_empty() || !forking.session_id.chars().all(plain) {
        return refusal(StatusCode::NOT_FOUND, NO_SUCH_SESSION);
    }
    let path = format!("/sessions/one?session_id={}", forking.session_id);
    let SessionThread { session: old, rows } = match read::<SessionThread>(&gateway, &path, "a job").await {
        Ok(Some(thread)) => thread,
        Ok(None) => return refusal(StatusCode::NOT_FOUND, NO_SUCH_SESSION),
        Err(refused) => return refused,
    };
    let Some(manifest_id) = old.manifest_id.clone() else {
        return refusal(StatusCode::CONFLICT, NO_REPOSITORY_TO_FORK);
    };
    // A live Job is copied up to a row, or Fleet refuses it. Fleet cuts only at a message, so
    // this is its agent's last one, the point Fleet itself picks when a fork names none.
    let at = if old.state == SessionState::Live {
        let last = rows.iter().rev().find_map(|row| match row {
            SessionRow::Message { id, from: SessionVoice::Agent, .. } => Some(id.clone()),
            _ => None,
        });
        match last {
            Some(id) => Some(id),
            None => return refusal(StatusCode::CONFLICT, NOTHING_TO_FORK_YET),
        }
    } else {
        None
    };
    let project = match shared_project(&gateway, &manifest_id).await {
        Ok(project) => project,
        Err(refused) => return refused,
    };
    let start = StartSession {
        fork: Some(ForkFrom { session_id: old.id.clone(), at }),
        project: Some(project),
        ..starting(manifest_id)
    };
    let record = match started(&gateway, &start).await {
        Ok(record) => record,
        Err(refused) => return refused,
    };
    let labels = repositories(&gateway).await;
    let repository = record.manifest_id.as_ref().and_then(|id| labels.get(id.as_str())).map(String::as_str);
    made(&record, repository, &gateway)
}

#[derive(Serialize)]
struct Dispatched {
    session: PhoneSession,
}
