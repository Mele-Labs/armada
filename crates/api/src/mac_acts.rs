//! Acts across his Macs over the router: an act on a Job another Mac holds goes there, and what another Mac sends
//! here runs through the routes the person's own press uses. `docs/practices/protocol/his-other-macs-fleets-half.md`,
//! *Acting across Macs*.
//!
//! **The holder runs the very handler the person's press reaches**, so every status, refusal and body is the one
//! it gives at home; the door it runs under is `mac`, named by the caller header before any handler sees it.

use axum::body::{Body, Bytes};
use axum::extract::{Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use ipc::{JobAct, MacAct, WireError};
use serde::{Deserialize, Serialize};

use crate::acting::MAC_HEADER;
use crate::answers::{answer, problem, refused, undecodable};
use crate::daemon::{ActBegun, HostedSessions, OtherMacs, PullRequests, Refusal, Sessions};
use crate::served::Served;

/// The most an act's answer is read to, which is the most a message may weigh and a record beside it.
const MOST_AN_ANSWER_MAY_BE: usize = 96 * 1024 * 1024;

/// A response that would not read back to be kept.
const ANSWER_UNREADABLE: &str = "api.act_answer_unreadable";

/// Another Mac's answer, handed back as it was said.
pub(crate) fn said(status: u16, body: Vec<u8>) -> Response {
    let status = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY);
    (status, [(header::CONTENT_TYPE, "application/json")], body).into_response()
}

/// `act` sent to Mac `mac`, which holds its Job, and what it answered.
pub(crate) async fn across<D: OtherMacs>(served: &Served<D>, mac: String, act: JobAct) -> Response {
    match served.shared().act_on_other_mac(mac, act).await {
        Ok(answered) => said(answered.status, answered.body),
        Err(refusal) => refused(refusal),
    }
}

/// `body` as the route's own body, for the handler that reads it.
fn carried<T: Serialize>(value: &T, run_id: &ipc::RunId) -> Result<Bytes, Response> {
    ipc::encode(value).map(Bytes::from).map_err(|why| {
        problem(StatusCode::INTERNAL_SERVER_ERROR, &WireError::raised(ANSWER_UNREADABLE, why.to_string(), run_id.clone()))
    })
}

/// The act run by the handler its own route has.
async fn run<D: HostedSessions + Sessions + PullRequests + OtherMacs>(served: Served<D>, act: JobAct) -> Response {
    use crate::{hosted_sessions as hosted, pull_requests, sessions};
    let run_id = served.run_id().clone();
    macro_rules! through {
        ($handler:path, $body:expr) => {
            match carried($body, &run_id) {
                Ok(body) => $handler(State(served), body).await,
                Err(problem) => problem,
            }
        };
    }
    match act {
        JobAct::Message(body) => through!(hosted::send_session_message, &body),
        JobAct::AskAnswer(body) => through!(hosted::answer_session_ask, &body),
        JobAct::WaitingAnswer(body) => through!(hosted::answer_waiting, &body),
        JobAct::WaitingAnswerParts(body) => through!(hosted::answer_waiting_parts, &body),
        JobAct::PlanApprove(body) => through!(sessions::approve_plan, &body),
        JobAct::PlanChanges(body) => through!(sessions::send_plan_changes, &body),
        JobAct::SketchAnswer(body) => through!(sessions::answer_sketch, &body),
        JobAct::DocumentAnswer(body) => through!(sessions::answer_document, &body),
        JobAct::Stop(body) => through!(hosted::stop_session, &body),
        JobAct::Merge { repository, number, .. } => {
            pull_requests::merge_pull_request_by_number(State(served), axum::extract::Path((repository, number))).await
        }
        // The words a Job acts as the person on are Bridge's to give, never another Mac's.
        JobAct::Start(mut start) => {
            start.on_your_words = None;
            match carried(&start, &run_id) {
                Ok(body) => hosted::start_session(State(served), None, body).await,
                Err(problem) => problem,
            }
        }
        JobAct::Dispatch(mut dispatch) => {
            dispatch.start.on_your_words = None;
            through!(hosted::dispatch_job, &dispatch)
        }
    }
}

/// `POST /other_mac/act`: what the Gateway hands in from the Mac it names in `x-armada-mac`.
pub(crate) async fn act_from_mac<D: HostedSessions + Sessions + PullRequests + OtherMacs>(
    State(served): State<Served<D>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let sent: MacAct = match ipc::decode_body("an act from another Mac", &body) {
        Ok(sent) => sent,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    let named = headers.get(MAC_HEADER).and_then(|one| one.to_str().ok()).map(str::trim).filter(|one| !one.is_empty());
    let Some(mac) = named else {
        return undecodable("an act from another Mac names the Mac in x-armada-mac", served.run_id());
    };
    let turn = match served.daemon().begin_act_from_mac(mac.to_string(), &sent).await {
        Ok(ActBegun::Answered(kept)) => return said(kept.status, kept.body),
        Ok(ActBegun::Run(turn)) => turn,
        Err(refusal) => return refused(refusal),
    };
    let (parts, body) = run(served.clone(), sent.act).await.into_parts();
    let Ok(bytes) = axum::body::to_bytes(body, MOST_AN_ANSWER_MAY_BE).await else {
        return refused(Refusal::Fault(WireError::raised(ANSWER_UNREADABLE, "the answer to the act could not be read", served.run_id().clone())));
    };
    served.daemon().finish_act_from_mac(turn, parts.status.as_u16(), &bytes).await;
    Response::from_parts(parts, Body::from(bytes))
}

/// `?mac=` on `list_queued`.
#[derive(Deserialize)]
pub(crate) struct WhichMac {
    mac: Option<String>,
}

pub(crate) async fn list_queued<D: OtherMacs>(State(served): State<Served<D>>, Query(which): Query<WhichMac>) -> Response {
    match served.daemon().list_queued(which.mac).await {
        Ok(queued) => answer(StatusCode::OK, &queued, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn take_back_queued<D: OtherMacs>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let take: ipc::TakeBack = match ipc::decode_body("an act to take back", &body) {
        Ok(take) => take,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().take_back_queued(take).await {
        Ok(queued) => answer(StatusCode::OK, &queued, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
