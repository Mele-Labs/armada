//! A Job's terminals: the person's open and rename from Bridge, and the agent's typing, reading and
//! listing through the door. A shell's output, keys, size and stop are the run routes, on its `run_id`.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Sessions;
use crate::served::Served;

/// 200 with the shell's run id, open already or opened now. 422 for a Job that has ended or that
/// Fleet does not know, a size with no cells, a `terminal` the Job never had, and `new` with `terminal`.
pub(crate) async fn open_terminal<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let open: ipc::OpenTerminal = match ipc::decode_body("a job shell to open", &body) {
        Ok(open) => open,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().open_terminal(open).await {
        Ok(underway) => answer(StatusCode::OK, &underway, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 200 with the Job's id and its `t1`'s run id. 422 for both or neither of `manifest_id` and `dir`, a
/// `dir` that is not an absolute directory, a size with no cells, and as `start_session`'s refusals.
pub(crate) async fn start_terminal_job<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let start: ipc::StartTerminalJob = match ipc::decode_body("a quick terminal to start", &body) {
        Ok(start) => start,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().start_terminal_job(start).await {
        Ok(started) => answer(StatusCode::OK, &started, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 200 with the run's id. 422 for a blank command, a directory that does not exist or a size with
/// no cells. No Job is named or told: the run's output, keys, size and stop are the run routes.
pub(crate) async fn start_terminal_run<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let start: ipc::StartTerminalRun = match ipc::decode("a terminal run to start", &body) {
        Ok(start) => start,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().start_terminal_run(start).await {
        Ok(underway) => answer(StatusCode::OK, &underway, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 200 with what the shell printed once it went quiet. 422 for a call that places no Job, or one
/// `open_terminal` refuses. **The Job is placed by the connection** where the call arrived
/// through the agent's door, and by the body's `session_id` only where it places none.
pub(crate) async fn type_in_terminal<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let typed: ipc::TypeInTerminal = match ipc::decode_body("keys for a job shell", &body) {
        Ok(typed) => typed,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().type_in_terminal(crate::acting::asking(), typed).await {
        Ok(heard) => answer(StatusCode::OK, &heard, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 200 with the shell's last lines, once `until` was printed or its wait ran out. 422 for a call
/// that places no Job, a Job with no shell open, or an `until` that is no regular expression.
/// **The Job is placed by the connection** as `type_in_terminal`'s is.
pub(crate) async fn read_terminal<D: Sessions>(
    State(served): State<Served<D>>,
    Query(mut asked): Query<ipc::ReadTerminal>,
) -> Response {
    asked.session_id = asked
        .session_id
        .filter(|id| !id.as_str().trim().is_empty())
        .map(|id| ipc::SessionId::carried(id.as_str().trim()));
    match served.daemon().read_terminal(crate::acting::asking(), asked).await {
        Ok(screen) => answer(StatusCode::OK, &screen, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 200 with the Job's open terminals. 422 for a call that places no Job.
/// **The Job is placed by the connection** as `type_in_terminal`'s is.
pub(crate) async fn list_terminals<D: Sessions>(
    State(served): State<Served<D>>,
    Query(mut asked): Query<ipc::ListTerminals>,
) -> Response {
    asked.session_id = asked
        .session_id
        .filter(|id| !id.as_str().trim().is_empty())
        .map(|id| ipc::SessionId::carried(id.as_str().trim()));
    match served.daemon().list_terminals(crate::acting::asking(), asked).await {
        Ok(listed) => answer(StatusCode::OK, &listed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 200 with the terminal shown, once `terminal.shown` is published. 422 for a call that places no
/// Job, or a terminal that is not open. **The Job is placed by the connection** as
/// `type_in_terminal`'s is.
pub(crate) async fn show_terminal<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let asked: ipc::ShowTerminal = match ipc::decode_body("a terminal to show", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().show_terminal(crate::acting::asking(), asked).await {
        Ok(shown) => answer(StatusCode::OK, &shown, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// 200 with the terminal as it now stands. 422 for a terminal the Job has not open.
pub(crate) async fn rename_terminal<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let rename: ipc::RenameTerminal = match ipc::decode_body("a terminal to rename", &body) {
        Ok(rename) => rename,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().rename_terminal(rename).await {
        Ok(renamed) => answer(StatusCode::OK, &renamed, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
