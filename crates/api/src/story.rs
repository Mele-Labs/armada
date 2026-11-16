//! What the owner said in a Job, sorted by kind, the asks that came of it, and a Job's story, over the router.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Story;
use crate::served::Served;

/// `?session_id=` on the reads of one Job.
#[derive(Deserialize)]
pub(crate) struct OfJob {
    session_id: String,
}

/// `?project=` and optionally `&feature=` on `list_feature_asks`.
#[derive(Deserialize)]
pub(crate) struct OfProject {
    project: String,
    #[serde(default)]
    feature: Option<String>,
}

/// How each message of his in the Job was sorted. 200; 404 for no such Job.
pub(crate) async fn list_message_kinds<D: Story>(State(served): State<Served<D>>, Query(of): Query<OfJob>) -> Response {
    match served.daemon().list_message_kinds(ipc::SessionId::carried(of.session_id)).await {
        Ok(list) => answer(StatusCode::OK, &list, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// He changes a message's kind. 200 with the sorting as kept; 404 for a Job, a message or an ask not held; 409 where an
/// ask the message opened holds more words; 422 for a revision or answer with no ask.
pub(crate) async fn set_message_kind<D: Story>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let set: ipc::SetMessageKind = match ipc::decode_body("a message's kind", &body) {
        Ok(set) => set,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().set_message_kind(set).await {
        Ok(sorting) => answer(StatusCode::OK, &sorting, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The asks said in, built by or handed through the Job. 200; 404 for no such Job.
pub(crate) async fn list_job_asks<D: Story>(State(served): State<Served<D>>, Query(of): Query<OfJob>) -> Response {
    match served.daemon().list_job_asks(ipc::SessionId::carried(of.session_id)).await {
        Ok(list) => answer(StatusCode::OK, &list, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The Job's story. 200; 404 for no such Job.
pub(crate) async fn get_job_story<D: Story>(State(served): State<Served<D>>, Query(of): Query<OfJob>) -> Response {
    match served.daemon().get_job_story(ipc::SessionId::carried(of.session_id)).await {
        Ok(story) => answer(StatusCode::OK, &story, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// He moves an ask. 200 with the ask as kept; 404 for no such ask; 422 for a state Fleet moves itself or a blank title.
pub(crate) async fn change_ask<D: Story>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let change: ipc::ChangeAsk = match ipc::decode_body("a change to an ask", &body) {
        Ok(change) => change,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().change_ask(change).await {
        Ok(ask) => answer(StatusCode::OK, &ask, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A project's asks, narrowed to a feature slug. 200; an unknown project or slug answers an empty list.
pub(crate) async fn list_feature_asks<D: Story>(State(served): State<Served<D>>, Query(of): Query<OfProject>) -> Response {
    match served.daemon().list_feature_asks(of.project, of.feature).await {
        Ok(list) => answer(StatusCode::OK, &list, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
