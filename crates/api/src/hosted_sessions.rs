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
    let start: ipc::StartSession = match ipc::decode_body("a job to start", &body) {
        Ok(start) => start,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().start_session_reading(start).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The person hands a group of a Session's plan to a linked Session. 200 with `SessionHeld`.
pub(crate) async fn hand_to_session<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let hand: ipc::HandToSession = match ipc::decode_body("a group to hand to a job", &body) {
        Ok(hand) => hand,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().hand_to_session(hand).await {
        Ok(held) => answer(StatusCode::OK, &held, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The proposer reads a brief that tags a Session for the wait it names. 200 with `WaitReading`.
pub(crate) async fn read_session_wait<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let read: ipc::ReadSessionWait = match ipc::decode_body("a brief to read for a wait", &body) {
        Ok(read) => read,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().read_session_wait(read).await {
        Ok(reading) => answer(StatusCode::OK, &reading, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The person starts a waiting Session at once. 200 with the `SessionRecord`.
pub(crate) async fn start_waiting_now<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let start: ipc::StartWaitingNow = match ipc::decode_body("a waiting job to start", &body) {
        Ok(start) => start,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().start_waiting_now(start).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The most a message may weigh. Its pictures and files ride inside it as base64, and one
/// Retina screenshot is past axum's 2 MB default on its own.
pub(crate) const MOST_A_MESSAGE_MAY_BE: usize = 64 * 1024 * 1024;

/// **202**: the message is taken and the reply is not here yet.
pub(crate) async fn send_session_message<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let sent: ipc::SendSessionMessageBody = match ipc::decode_body("a message to a job", &body) {
        Ok(sent) => sent,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().send_session_message_aiming(sent.message, sent.aim).await {
        Ok(record) => answer(StatusCode::ACCEPTED, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// **202**, as a message is: the note is taken, with the lines it was handed, and the reply is the
/// thread's.
pub(crate) async fn send_session_note<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let note: ipc::SendSessionNote = match ipc::decode_body("a note to a session", &body) {
        Ok(note) => note,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().send_session_note(note).await {
        Ok(sent) => answer(StatusCode::ACCEPTED, &sent, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// **202**, as one note is: the notes are taken as one turn, with each note's lines.
pub(crate) async fn send_session_notes<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let notes: ipc::SendSessionNotes = match ipc::decode_body("notes to a session", &body) {
        Ok(notes) => notes,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().send_session_notes(notes).await {
        Ok(sent) => answer(StatusCode::ACCEPTED, &sent, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn answer_session_ask<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let said: ipc::AnswerSessionAsk = match ipc::decode_body("an answer to a job's ask", &body) {
        Ok(said) => said,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().answer_session_ask(said).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The person settles one thing a session waits on. 200 with the row; 409 where nothing holds it.
pub(crate) async fn answer_waiting<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let said: ipc::AnswerWaiting = match ipc::decode_body("an answer to what a job waits on", &body) {
        Ok(said) => said,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().answer_waiting(said).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The person drops one thing a session waits on for good. 200 with the row; 409 where nothing holds it.
pub(crate) async fn dismiss_waiting<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let dismiss: ipc::DismissWaiting = match ipc::decode_body("a dismissal of what a job waits on", &body) {
        Ok(dismiss) => dismiss,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().dismiss_waiting(dismiss).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The person lets an item he closed wait again. 200 with the row; 409 where no closed item has the id;
/// 422 where it would not wait again.
pub(crate) async fn reopen_waiting<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let reopen: ipc::ReopenWaiting = match ipc::decode_body("a reopening of what a job waited on", &body) {
        Ok(reopen) => reopen,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().reopen_waiting(reopen).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The person says a question Fleet read from a reply is one. 200 with the row; 409 where nothing holds
/// the item, and where it was not read from a reply.
pub(crate) async fn confirm_waiting<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let confirm: ipc::ConfirmWaiting = match ipc::decode_body("a confirmation of what a job waits on", &body) {
        Ok(confirm) => confirm,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().confirm_waiting(confirm).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The person says a follow-up is one. 200 with the row; 409 where nothing open holds it.
pub(crate) async fn confirm_later<D: HostedSessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let confirm: ipc::ConfirmLater = match ipc::decode("a confirmation of a follow-up", &body) {
        Ok(confirm) => confirm,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().confirm_later(confirm).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The newest reading of the plan's windows.
pub(crate) async fn get_plan_usage<D: HostedSessions>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_plan_usage().await {
        Ok(usage) => answer(StatusCode::OK, &usage, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Where the default model's step down stands.
pub(crate) async fn get_model_downshift<D: HostedSessions>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_model_downshift().await {
        Ok(state) => answer(StatusCode::OK, &state, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The owner takes the lower default. 200 with the state; 409 where nothing is offered.
pub(crate) async fn accept_downshift<D: HostedSessions>(State(served): State<Served<D>>) -> Response {
    match served.daemon().accept_downshift().await {
        Ok(state) => answer(StatusCode::OK, &state, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The owner puts the default back. 200 with the state; 409 unless shifted or reset.
pub(crate) async fn restore_downshift<D: HostedSessions>(State(served): State<Served<D>>) -> Response {
    match served.daemon().restore_downshift().await {
        Ok(state) => answer(StatusCode::OK, &state, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The owner keeps the lower default after the reset. 200 with the state; 409 unless reset.
pub(crate) async fn keep_downshift<D: HostedSessions>(State(served): State<Served<D>>) -> Response {
    match served.daemon().keep_downshift().await {
        Ok(state) => answer(StatusCode::OK, &state, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The owner sets the default by hand. 200 with the state; 409 unless offered.
pub(crate) async fn settle_downshift<D: HostedSessions>(State(served): State<Served<D>>) -> Response {
    match served.daemon().settle_downshift().await {
        Ok(state) => answer(StatusCode::OK, &state, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Whether the agents' account is signed out, and whether Fleet is signing it in.
pub(crate) async fn get_agent_sign_in<D: HostedSessions>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_agent_sign_in().await {
        Ok(state) => answer(StatusCode::OK, &state, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Fleet runs the agents' sign-in. 200 with the state, `signing_in` set.
pub(crate) async fn start_agent_sign_in<D: HostedSessions>(State(served): State<Served<D>>) -> Response {
    match served.shared().start_agent_sign_in().await {
        Ok(state) => answer(StatusCode::OK, &state, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// Whether sleep mode is on and what the night holds.
pub(crate) async fn get_sleep<D: HostedSessions>(State(served): State<Served<D>>) -> Response {
    match served.daemon().get_sleep().await {
        Ok(state) => answer(StatusCode::OK, &state, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn set_sleep<D: HostedSessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let set: ipc::SetSleep = match ipc::decode_body("a sleep switch", &body) {
        Ok(set) => set,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().set_sleep(set).await {
        Ok(state) => answer(StatusCode::OK, &state, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The owner corrects one decision. 200 with the night; 409 where no decision holds the id.
pub(crate) async fn override_sleep<D: HostedSessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let over: ipc::OverrideSleep = match ipc::decode_body("a correction of a sleep decision", &body) {
        Ok(over) => over,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().override_sleep(over).await {
        Ok(state) => answer(StatusCode::OK, &state, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// The owner keeps one decision. 200 with the night; 409 where no decision holds the id.
pub(crate) async fn review_sleep<D: HostedSessions>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let review: ipc::ReviewSleep = match ipc::decode_body("a kept sleep decision", &body) {
        Ok(review) => review,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().review_sleep(review).await {
        Ok(state) => answer(StatusCode::OK, &state, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn tune_session<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let tuned: ipc::TuneSession = match ipc::decode_body("a job's tuning", &body) {
        Ok(tuned) => tuned,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().tune_session(tuned).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn run_session_command<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let run: ipc::RunSessionCommand = match ipc::decode_body("a job's command", &body) {
        Ok(run) => run,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().run_session_command(run).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn close_session<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let closed: ipc::CloseSession = match ipc::decode_body("a job to close", &body) {
        Ok(closed) => closed,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().close_session(closed).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// A Session moved to another repository. 200 with the new Session; 404 for an unknown Session or
/// repository, 409 for its own repository or a terminal Session still listening.
pub(crate) async fn move_session<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let moved: ipc::MoveSession = match ipc::decode_body("a job to move", &body) {
        Ok(moved) => moved,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().move_session(moved).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn stop_session<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let stopped: ipc::StopSession = match ipc::decode_body("a job to stop", &body) {
        Ok(stopped) => stopped,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().stop_session(stopped).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn retry_handoff<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: ipc::RetryHandoff = match ipc::decode_body("a hand off to retry", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().retry_handoff(asked).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn pause_session<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: ipc::PauseSession = match ipc::decode_body("a job to pause", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().pause_session(asked).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn set_session_auto_land<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: ipc::SetSessionAutoLand = match ipc::decode_body("an Auto land setting", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().set_session_auto_land(asked).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn get_session_mcp<D: HostedSessions>(
    State(served): State<Served<D>>,
    Query(which): Query<Which>,
) -> Response {
    match served
        .daemon()
        .get_session_mcp(ipc::SessionId::carried(which.session_id))
        .await
    {
        Ok(servers) => answer(StatusCode::OK, &servers, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn act_on_session_mcp<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: ipc::ActOnSessionMcp = match ipc::decode_body("an act on a job's MCP server", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().act_on_session_mcp(asked).await {
        Ok(acted) => answer(StatusCode::OK, &acted, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn mend_session_tools<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: ipc::MendSessionTools = match ipc::decode_body("a job's tools to mend", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().mend_session_tools(asked).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn start_again<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: ipc::StartAgain = match ipc::decode_body("a job to start again", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().start_again(asked).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn resume_session<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: ipc::ResumeSession = match ipc::decode_body("a job to resume", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().resume_session(asked).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn set_session_workflow<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let set: ipc::SetSessionWorkflow = match ipc::decode_body("a workflow for a job", &body) {
        Ok(set) => set,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().set_session_workflow(set).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn skip_session_step<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let skip: ipc::SkipSessionStep = match ipc::decode_body("a step to skip", &body) {
        Ok(skip) => skip,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().skip_session_step(skip).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

pub(crate) async fn begin_session<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let begin: ipc::BeginSession = match ipc::decode_body("a job to begin", &body) {
        Ok(begin) => begin,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().begin_session(begin).await {
        Ok(record) => answer(StatusCode::OK, &record, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// What a Session held, given back or handed over. 200 with what moved and what stayed; 422 for a
/// Session Fleet has no row for, 409 for a target that cannot take them.
pub(crate) async fn release_session_holds<D: HostedSessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let release: ipc::ReleaseSessionHolds = match ipc::decode_body("a job's holds to release", &body) {
        Ok(release) => release,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().release_session_holds(release).await {
        Ok(released) => answer(StatusCode::OK, &released, served.run_id()),
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
    let ask: ipc::TakeHeld = match ipc::decode_body("a terminal job's ask for held messages", &body)
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
    let ask: ipc::TerminalAsk = match ipc::decode_body("a terminal job's question", &body) {
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
    let gate: ipc::SessionGate = match ipc::decode_body("a job's tool gate", &body) {
        Ok(gate) => gate,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().gate_session_call(gate).await {
        Ok(decided) => answer(StatusCode::OK, &decided, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
