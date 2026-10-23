//! A Job's shell: the person's open from Bridge, and the agent's typing and reading through the
//! door. Its output, keys, size and stop are the run routes, on its `run_id`.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

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
