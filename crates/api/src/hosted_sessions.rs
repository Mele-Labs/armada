//! The routes of a session Bridge hosts. Since 23.49.
//! `docs/concepts/session.md`.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::HostedSessions;
use crate::served::Served;

pub(crate) async fn start_session<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let start: ipc::StartSession = match ipc::decode("a session to start", &body) {
        Ok(start) => start,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().start_session(start).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// **202**: the message is taken and the reply is not here yet.
pub(crate) async fn send_session_message<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let sent: ipc::SendSessionMessage = match ipc::decode("a message to a session", &body) {
        Ok(sent) => sent,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().send_session_message(sent).await {
        Ok(record) => answer(StatusCode::ACCEPTED, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn answer_session_ask<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let said: ipc::AnswerSessionAsk = match ipc::decode("an answer to a session's ask", &body) {
        Ok(said) => said,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().answer_session_ask(said).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn tune_session<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let tuned: ipc::TuneSession = match ipc::decode("a session's tuning", &body) {
        Ok(tuned) => tuned,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().tune_session(tuned).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn close_session<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let closed: ipc::CloseSession = match ipc::decode("a session to close", &body) {
        Ok(closed) => closed,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().close_session(closed).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?session_id=` on `get_session`.
#[derive(Deserialize)]
pub(crate) struct Which {
    session_id: String,
}

pub(crate) async fn get_session<D: HostedSessions>(
    State(served): State<Served<D>>,
    Query(which): Query<Which>,
) -> Response {
    match served
        .shared()
        .get_session(ipc::SessionId::carried(which.session_id))
        .await
    {
        Ok(thread) => answer(StatusCode::OK, &thread, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?session_id=` and `?subagent_id=` on `get_session_subagent`.
#[derive(Deserialize)]
pub(crate) struct WhichSubagent {
    session_id: String,
    subagent_id: String,
}

pub(crate) async fn get_session_subagent<D: HostedSessions>(
    State(served): State<Served<D>>,
    Query(which): Query<WhichSubagent>,
) -> Response {
    match served
        .daemon()
        .get_session_subagent(ipc::SessionId::carried(which.session_id), which.subagent_id)
        .await
    {
        Ok(thread) => answer(StatusCode::OK, &thread, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?session_id=` and `?file=` on `get_session_file`.
#[derive(Deserialize)]
pub(crate) struct WhichFile {
    session_id: String,
    file: String,
}

/// The bytes, with the type they were sent as.
pub(crate) async fn get_session_file<D: HostedSessions>(
    State(served): State<Served<D>>,
    Query(which): Query<WhichFile>,
) -> Response {
    match served
        .daemon()
        .get_session_file(ipc::SessionId::carried(which.session_id), which.file)
        .await
    {
        Ok(stored) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, stored.media_type)],
            stored.bytes,
        )
            .into_response(),
        Err(refusal) => refused(refusal),
    }
}

/// The terminal session's mod, asking what a person sent it.
pub(crate) async fn take_held_messages<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let ask: ipc::TakeHeld = match ipc::decode("a terminal session's ask for held messages", &body)
    {
        Ok(ask) => ask,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().take_held_messages(ask).await {
        Ok(held) => answer(StatusCode::OK, &held, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The terminal session's mod, putting a question to Bridge. Held open until
/// it is answered, so it ends when the person does.
pub(crate) async fn ask_from_terminal<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let ask: ipc::TerminalAsk = match ipc::decode("a terminal session's question", &body) {
        Ok(ask) => ask,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().ask_from_terminal(ask).await {
        Ok(asked) => answer(StatusCode::OK, &asked, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The hook a hosted session runs before a tool. **Always 200**: a refusal is
/// the answer's own `deny`, in the shape the harness reads.
pub(crate) async fn gate_session_call<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let gate: ipc::SessionGate = match ipc::decode("a session's tool gate", &body) {
        Ok(gate) => gate,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().gate_session_call(gate).await {
        Ok(decided) => answer(StatusCode::OK, &decided, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
