//! Requests over the router: a request sent, and a repository's collections and environments read
//! and saved.
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
    let send: ipc::SendRequest = match ipc::decode_body("a request to send", &body) {
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

/// `?repository=&session_id=`, each optional; the session is read only where the connection
/// places no Job.
#[derive(Deserialize)]
pub(crate) struct ForRequests {
    #[serde(default)]
    repository: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
}

pub(crate) async fn get_requests<D: Requests>(
    State(served): State<Served<D>>,
    Query(asked): Query<ForRequests>,
) -> Response {
    let named = asked
        .session_id
        .filter(|id| !id.trim().is_empty())
        .map(|id| ipc::SessionId::carried(id.trim()));
    let repository = asked.repository.filter(|root| !root.trim().is_empty());
    match served.daemon().get_requests(crate::acting::asking(), repository, named).await {
        Ok(read) => answer(StatusCode::OK, &read, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn save_request_collection<D: Requests>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let save: ipc::SaveRequestCollection = match ipc::decode_body("a collection to save", &body) {
        Ok(save) => save,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().save_request_collection(crate::acting::asking(), save).await {
        Ok(read) => answer(StatusCode::OK, &read, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn save_request_environment<D: Requests>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    // The decode error names a field and never quotes the body, so a secret's value stays out of it.
    let save: ipc::SaveRequestEnvironment = match ipc::decode_body("an environment to save", &body) {
        Ok(save) => save,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().save_request_environment(crate::acting::asking(), save).await {
        Ok(read) => answer(StatusCode::OK, &read, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn delete_request_environment<D: Requests>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let delete: ipc::DeleteRequestEnvironment = match ipc::decode_body("an environment to delete", &body) {
        Ok(delete) => delete,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().delete_request_environment(crate::acting::asking(), delete).await {
        Ok(read) => answer(StatusCode::OK, &read, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn save_request_to_collection<D: Requests>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let save: ipc::SaveRequestToCollection = match ipc::decode_body("a request to save", &body) {
        Ok(save) => save,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().save_request_to_collection(crate::acting::asking(), save).await {
        Ok(saved) => answer(StatusCode::OK, &saved, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
