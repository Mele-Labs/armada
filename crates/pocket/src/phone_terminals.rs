//! A Job's terminals on the phone. The phone is the owner on this Mac, so these are Fleet's own terminal
//! operations (`list_terminals`, `write_session_run`, `observe_session_run`), not the shared-Job ones of
//! [`crate::mac_terminals`]. The phone follows terminals the Mac opened; it opens, resizes and stops none.
//!
//! **Nothing is queued.** A keystroke the Gateway cannot hand to Fleet is refused at once and never typed
//! later, and a stream that Fleet ends or drops ends.

use axum::body::{Body, Bytes};
use axum::extract::{Query, State};
use axum::http::{header, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use futures_util::StreamExt;
use ipc::door::encoded;
use serde::Deserialize;

use crate::reads::{port, refusal, NOT_ANSWERING};
use crate::{fleet_client, forward, reads, Gateway};

/// A line with nothing on it: the phone reads it as the stream still being alive.
const BEATING: &str = "\n";

#[derive(Deserialize)]
pub struct Job {
    session_id: String,
}

#[derive(Deserialize)]
pub struct Run {
    run_id: String,
}

/// `GET /api/sessions/terminals?session_id=`: Fleet's `list_terminals` for that Job, as it answers.
pub async fn list(State(gateway): State<Gateway>, Query(job): Query<Job>) -> Response {
    let target = format!("/sessions/terminal/list?session_id={}", encoded(&job.session_id));
    forward::send_to(&gateway, &Method::GET, &target, &[]).await
}

/// `POST /api/sessions/terminal/write`, `{run_id, data}`: Fleet's `write_session_run`, as it answers. Fleet
/// reads the body, so a run that has ended is its 422.
pub async fn write(State(gateway): State<Gateway>, body: Bytes) -> Response {
    forward::send_to(&gateway, &Method::POST, "/sessions/run/write", &body).await
}

/// `GET /api/sessions/terminal/observe?run_id=`: Fleet's `observe_session_run`, one `SessionRunMessage` a
/// line as Fleet wrote it, and an empty line whenever it is quiet for a beat. Fleet's refusal of a run it
/// does not hold is answered before anything streams, and the stream ends where Fleet's socket closes,
/// which it does after `exited`.
pub async fn observe(State(gateway): State<Gateway>, Query(run): Query<Run>) -> Response {
    let port = match port(&gateway) {
        Ok(port) => port,
        Err(refused) => return refused,
    };
    let path = format!("/sessions/run/{}/observe", encoded(&run.run_id));
    let upgraded = match fleet_client::upgrade_or_refused(port, &path).await {
        Ok(Ok(upgraded)) => upgraded,
        Ok(Err(answer)) => {
            let status = StatusCode::from_u16(answer.status).unwrap_or(StatusCode::BAD_GATEWAY);
            let kind = answer.content_type.unwrap_or_else(|| String::from("application/json"));
            return (status, [(header::CONTENT_TYPE, kind)], answer.body).into_response();
        }
        Err(_) => return refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_ANSWERING),
    };
    let lines = reads::relay(upgraded, gateway.macs.timing.beat, BEATING, |text| Some(format!("{}\n", text.trim())));
    let lines = lines.take_until(gateway.macs.closed());
    (
        [(header::CONTENT_TYPE, "application/x-ndjson"), (header::CACHE_CONTROL, "no-cache")],
        Body::from_stream(lines),
    )
        .into_response()
}
