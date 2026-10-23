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
