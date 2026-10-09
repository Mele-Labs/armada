//! The phone's act routes: each one call to one Fleet route.
//!
//! **Fleet's body is built here field by field** from the few fields the phone
//! may send; the phone's JSON is never forwarded. An answer to the phone is
//! 204 on success, since the phone re-reads the Job and `/api/live` tells it
//! the change. A refusal passes through with Fleet's status and its sentence.

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{post, MethodRouter};
use ipc::{ChangesRequested, Redirection, WireError};
use serde::Deserialize;

use crate::{fleet_client, Gateway};

const NOT_RUNNING: &str = "Armada is not running on your Mac. Open it there, then try again.";
const NOT_ANSWERING: &str = "Armada is not answering on your Mac. Open it there, then try again.";
const NOT_READABLE: &str =
    "Armada answered in a way this phone cannot read. Update Armada on your Mac.";
const NO_SUCH_JOB: &str = "That Job is not here.";
const NOT_UNDERSTOOD: &str = "Armada could not read that from this phone. Update the app, then try again.";

#[derive(Clone, Copy)]
pub enum Act {
    Approve,
    Redirect,
    RestartStep,
    Kill,
    Redispatch,
    ApproveReview,
    RequestChanges,
}

impl Act {
    fn fleet_route(self) -> &'static str {
        match self {
            Act::Approve => "approve_dispatch",
            Act::Redirect => "redirect",
            Act::RestartStep => "restart_step",
            Act::Kill => "kill_job",
            Act::Redispatch => "redispatch",
            Act::ApproveReview => "approve_review",
            Act::RequestChanges => "request_changes",
        }
    }
}

/// What the phone sends, and nothing more is read.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Nothing {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Text {
    text: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reason {
    reason: String,
}

fn refusal(status: StatusCode, sentence: &'static str) -> Response {
    (status, sentence).into_response()
}

/// The body Fleet is sent, or the phone's refusal. `None` is no body at all,
/// which Fleet reads as the plain act.
fn fleet_body(act: Act, phone: &[u8]) -> Result<Option<String>, Response> {
    let unreadable = || refusal(StatusCode::BAD_REQUEST, NOT_UNDERSTOOD);
    let wrong = |_| unreadable();
    match act {
        Act::Redirect => {
            let said: Text = ipc::decode("a redirect", phone).map_err(wrong)?;
            let body = ipc::encode(&Redirection { instruction: said.text });
            body.map(Some).map_err(|_| refusal(StatusCode::INTERNAL_SERVER_ERROR, NOT_READABLE))
        }
        Act::RequestChanges => {
            let said: Reason = ipc::decode("a change request", phone).map_err(wrong)?;
            let body = ipc::encode(&ChangesRequested { note: said.reason, with_walk_notes: false });
            body.map(Some).map_err(|_| refusal(StatusCode::INTERNAL_SERVER_ERROR, NOT_READABLE))
        }
        _ => {
            if !phone.is_empty() {
                ipc::decode::<Nothing>("an empty act", phone).map_err(wrong)?;
            }
            Ok(None)
        }
    }
}

async fn act(gateway: Gateway, id: String, phone: Bytes, act: Act) -> Response {
    // An id is put into a path to Fleet, so only an id's own characters pass.
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return refusal(StatusCode::NOT_FOUND, NO_SUCH_JOB);
    }
    let body = match fleet_body(act, &phone) {
        Ok(body) => body,
        Err(refused) => return refused,
    };
    let port = match (gateway.fleet)() {
        Ok(port) => port,
        Err(_) => return refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_RUNNING),
    };
    let path = format!("/jobs/{id}/{}", act.fleet_route());
    let answer = match fleet_client::send(port, "POST", &path, body.as_deref().map(str::as_bytes)).await {
        Ok(answer) => answer,
        Err(_) => return refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_ANSWERING),
    };
    if (200..300).contains(&answer.status) {
        return StatusCode::NO_CONTENT.into_response();
    }
    let status = StatusCode::from_u16(answer.status).unwrap_or(StatusCode::BAD_GATEWAY);
    match ipc::decode::<WireError>("a refusal", &answer.body) {
        Ok(why) => (status, why.message).into_response(),
        Err(_) => refusal(StatusCode::BAD_GATEWAY, NOT_READABLE),
    }
}

pub fn route(which: Act) -> MethodRouter<Gateway> {
    post(move |State(gateway): State<Gateway>, Path(id): Path<String>, body: Bytes| {
        act(gateway, id, body, which)
    })
}
