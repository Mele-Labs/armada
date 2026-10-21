//! Turning a Fleet notice off, or back on.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::Commands;
use crate::served::Served;

/// Mute the notice for a Job, a repository or an hour, or lift the mute; answers with the body.
pub(crate) async fn set_notice_mute<D: Commands>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let asked: ipc::SetNoticeMute = match ipc::decode("a notice to mute", &body) {
        Ok(asked) => asked,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().set_notice_mute(asked).await {
        Ok(kept) => answer(StatusCode::OK, &kept, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
