//! A Job's shell: the person's open from Bridge, and the agent's typing and reading through the
//! door. Its output, keys, size and stop are the run routes, on its `run_id`.

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Sessions;
use crate::served::Served;

/// 200 with the shell's run id, open already or opened now. 422 for a Job that has ended or that
/// Fleet does not know, or a size with no cells.
pub(crate) async fn open_terminal<D: Sessions>(
    State(served): State<Served<D>>,
    body: Bytes,
) -> Response {
    let open: ipc::OpenTerminal = match ipc::decode("a job shell to open", &body) {
        Ok(open) => open,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().open_terminal(open).await {
        Ok(underway) => answer(StatusCode::OK, &underway, served.run_id()),
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
    let typed: ipc::TypeInTerminal = match ipc::decode("keys for a job shell", &body) {
        Ok(typed) => typed,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().type_in_terminal(crate::acting::asking(), typed).await {
        Ok(heard) => answer(StatusCode::OK, &heard, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

#[derive(Deserialize)]
pub(crate) struct ForTerminal {
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    lines: Option<usize>,
}

/// 200 with the shell's last lines. 422 for a call that places no Job, or a Job with no shell open.
/// **The Job is placed by the connection** as `type_in_terminal`'s is.
pub(crate) async fn read_terminal<D: Sessions>(
    State(served): State<Served<D>>,
    Query(asked): Query<ForTerminal>,
) -> Response {
    let named = asked
        .session_id
        .filter(|id| !id.trim().is_empty())
        .map(|id| ipc::SessionId::carried(id.trim()));
    match served.daemon().read_terminal(crate::acting::asking(), named, asked.lines).await {
        Ok(screen) => answer(StatusCode::OK, &screen, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
