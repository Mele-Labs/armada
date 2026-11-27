//! Margin threads over the router: his comments and acts on a page's threads, and the agent's answers and
//! suggested edits. **The agent's Job is the connection's** where Fleet hosts it; a body's `session_id` is read only
//! where the connection places none. **His comments and acts on a Job another Mac holds run there**, as acts
//! (`crate::mac_acts`), and its threads are read from there.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Margins, OtherMacs};
use crate::hosted_sessions::held_artifact_mac;
use crate::mac_acts::across;
use crate::served::Served;

pub(crate) async fn comment_margin<D: Margins + OtherMacs>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let comment: ipc::CommentMargin = match ipc::decode_body("a margin comment", &body) {
        Ok(comment) => comment,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    if let Some(mac) = held_artifact_mac(&served, Some(&comment.session_id)).await {
        return across(&served, mac, ipc::JobAct::MarginComment(comment)).await;
    }
    match served.shared().comment_margin(comment).await {
        Ok(threads) => answer(StatusCode::OK, &threads, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn send_held_margin<D: Margins + OtherMacs>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let send: ipc::SendHeldMargin = match ipc::decode_body("held margin comments to send", &body) {
        Ok(send) => send,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    if let Some(mac) = held_artifact_mac(&served, Some(&send.session_id)).await {
        return across(&served, mac, ipc::JobAct::MarginSendHeld(send)).await;
    }
    match served.shared().send_held_margin(send).await {
        Ok(threads) => answer(StatusCode::OK, &threads, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn act_on_margin_thread<D: Margins + OtherMacs>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let act: ipc::ActOnMargin = match ipc::decode_body("an act on a margin thread", &body) {
        Ok(act) => act,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    if let Some(mac) = held_artifact_mac(&served, Some(&act.session_id)).await {
        return across(&served, mac, ipc::JobAct::MarginAct(act)).await;
    }
    match served.shared().act_on_margin_thread(act).await {
        Ok(threads) => answer(StatusCode::OK, &threads, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?session_id=&document=`: the Job, and the page by its file name or path.
#[derive(Deserialize)]
pub(crate) struct ForPage {
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    document: String,
}

pub(crate) async fn get_margin_threads<D: Margins>(State(served): State<Served<D>>, Query(asked): Query<ForPage>) -> Response {
    if asked.session_id.trim().is_empty() || asked.document.trim().is_empty() {
        return undecodable("a page's margin threads are read by `session_id` and `document`", served.run_id());
    }
    let get = ipc::GetMarginThreads { session_id: ipc::SessionId::carried(asked.session_id.trim()), document: asked.document.trim().to_string() };
    match served.daemon().get_margin_threads(get).await {
        Ok(threads) => answer(StatusCode::OK, &threads, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn answer_thread<D: Margins>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let said: ipc::AnswerThread = match ipc::decode_body("an answer in a margin thread", &body) {
        Ok(said) => said,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().answer_thread(crate::acting::asking(), said).await {
        Ok(thread) => answer(StatusCode::OK, &thread, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn suggest_edit<D: Margins>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let said: ipc::SuggestEdit = match ipc::decode_body("an edit suggested in a margin thread", &body) {
        Ok(said) => said,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().suggest_edit(crate::acting::asking(), said).await {
        Ok(thread) => answer(StatusCode::OK, &thread, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
