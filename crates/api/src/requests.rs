//! Requests over the router: a request sent, and a repository's collections and environments read
//! and saved.
//!
//! **Who sent it is the door it came through.** Bridge and the phone name themselves and are the
//! owner; the agent's door and a request naming nobody are an agent, so a script that
//! leaves the header out is answered as an agent is and never as the owner.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use ipc::{RequestBy, RequestRunStatus, Via, WireError};
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Refusal, Requests, RunsAsked};
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

pub(crate) async fn send_requests<D: Requests>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let send: ipc::SendRequests = match ipc::decode_body("requests to send", &body) {
        Ok(send) => send,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().send_requests(crate::acting::asking(), send).await {
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

/// A run read that names nothing, or gives a value that is none of the spellings. A 422.
const RUN_REFUSED: &str = "api.request_run_refused";

fn run_refused(why: String, run_id: &ipc::RunId) -> Response {
    refused(Refusal::Unacceptable(WireError::raised(RUN_REFUSED, why, run_id.clone())))
}

/// `?repository=&session_id=&collection=&request=&by=&status=&before=&limit=`, each a string so a
/// value that is none of its spellings is answered by name and not as bytes that would not decode.
#[derive(Deserialize)]
pub(crate) struct ForRuns {
    #[serde(default)]
    repository: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    collection: Option<String>,
    #[serde(default)]
    request: Option<String>,
    #[serde(default)]
    by: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    before: Option<String>,
    #[serde(default)]
    limit: Option<String>,
}

/// `text` trimmed, and `None` where it is blank.
fn given(text: Option<String>) -> Option<String> {
    text.map(|text| text.trim().to_string()).filter(|text| !text.is_empty())
}

pub(crate) async fn list_request_runs<D: Requests>(
    State(served): State<Served<D>>,
    Query(asked): Query<ForRuns>,
) -> Response {
    let run_id = served.run_id();
    let (Some(collection), Some(request)) = (given(asked.collection), given(asked.request)) else {
        return run_refused("name the request: `collection` and `request` are its ids".to_string(), run_id);
    };
    let by = match given(asked.by).as_deref() {
        None => None,
        Some("owner") => Some(RequestBy::Owner),
        Some("agent") => Some(RequestBy::Agent),
        Some(other) => return run_refused(format!("`by` is `owner` or `agent`, not `{other}`"), run_id),
    };
    let status = match given(asked.status) {
        None => None,
        Some(spelled) => match RequestRunStatus::from_wire(&spelled) {
            Some(status) => Some(status),
            None => {
                return run_refused(format!("`status` is `2xx`, `3xx`, `4xx`, `5xx` or `failed`, not `{spelled}`"), run_id)
            }
        },
    };
    let limit = match given(asked.limit) {
        None => None,
        Some(spelled) => match spelled.parse::<usize>() {
            Ok(limit) => Some(limit),
            Err(_) => return run_refused(format!("`limit` is a count, not `{spelled}`"), run_id),
        },
    };
    let asked = RunsAsked {
        repository: given(asked.repository),
        session_id: given(asked.session_id).map(ipc::SessionId::carried),
        collection,
        request,
        by,
        status,
        before: given(asked.before),
        limit,
    };
    match served.daemon().list_request_runs(crate::acting::asking(), asked).await {
        Ok(runs) => answer(StatusCode::OK, &runs, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?repository=&session_id=&id=`.
#[derive(Deserialize)]
pub(crate) struct ForRun {
    #[serde(default)]
    repository: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    id: Option<String>,
}

pub(crate) async fn get_request_run<D: Requests>(
    State(served): State<Served<D>>,
    Query(asked): Query<ForRun>,
) -> Response {
    let Some(id) = given(asked.id) else {
        return run_refused("name the run: `id` is one `list_request_runs` answered".to_string(), served.run_id());
    };
    let session_id = given(asked.session_id).map(ipc::SessionId::carried);
    match served
        .daemon()
        .get_request_run(crate::acting::asking(), sent_by(), given(asked.repository), session_id, id)
        .await
    {
        Ok(run) => answer(StatusCode::OK, &run, served.run_id()),
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
