//! Requests over the router: a request sent, the cookies' hosts, and the environments.
//!
//! **Who sent it is the door it came through.** Bridge and the phone name themselves and are the
//! owner; the agent's door, Helm's, and a request naming nobody are an agent, so a script that
//! leaves the header out is answered as an agent is and never as the owner.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use ipc::{RequestBy, Via};
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Requests;
use crate::served::Served;

fn sent_by() -> RequestBy {
    match crate::acting::via().as_ref().map(Via::as_wire) {
        Some("bridge" | "phone") => RequestBy::Owner,
        _ => RequestBy::Agent,
    }
}

pub(crate) async fn send_request<D: Requests>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let send: ipc::SendRequest = match ipc::decode("a request to send", &body) {
        Ok(send) => send,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served
        .shared()
        .send_request(crate::acting::asking(), sent_by(), send)
        .await
    {
        Ok(sent) => answer(StatusCode::OK, &sent, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn list_request_hosts<D: Requests>(State(served): State<Served<D>>) -> Response {
    match served.daemon().list_request_hosts().await {
        Ok(hosts) => answer(StatusCode::OK, &hosts, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn set_request_cookie<D: Requests>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    // The decode error names a field and never quotes the body, so the value stays out of it.
    let set: ipc::SetRequestCookie = match ipc::decode("a cookie for a host", &body) {
        Ok(set) => set,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().set_request_cookie(set).await {
        Ok(hosts) => answer(StatusCode::OK, &hosts, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn forget_request_cookie<D: Requests>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let forget: ipc::ForgetRequestCookie = match ipc::decode("a host to forget", &body) {
        Ok(forget) => forget,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().forget_request_cookie(forget).await {
        Ok(hosts) => answer(StatusCode::OK, &hosts, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?session_id=`, read only where the connection places no Job.
#[derive(Deserialize)]
pub(crate) struct ForSession {
    #[serde(default)]
    session_id: Option<String>,
}

pub(crate) async fn list_request_environments<D: Requests>(
    State(served): State<Served<D>>,
    Query(asked): Query<ForSession>,
) -> Response {
    let named = asked
        .session_id
        .filter(|id| !id.trim().is_empty())
        .map(|id| ipc::SessionId::carried(id.trim()));
    match served
        .daemon()
        .list_request_environments(crate::acting::asking(), named)
        .await
    {
        Ok(environments) => answer(StatusCode::OK, &environments, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn save_request_environments<D: Requests>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let file: ipc::RequestEnvironmentsFile =
        match ipc::decode("the environments to save", &body) {
            Ok(file) => file,
            Err(why) => return undecodable(&why.to_string(), served.run_id()),
        };
    match served.daemon().save_request_environments(file).await {
        Ok(read) => answer(StatusCode::OK, &read, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
