//! A request turned away for its size answers in Fleet's own words.
//!
//! The extractor's refusal is a bare text 413 that Bridge can only show as a status number. This
//! crate has no log of its own, so the route, the size and the limit ride in the WireError.

use axum::extract::Request;
use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use ipc::{RunId, WireError, WireValue};

use crate::answers::problem;
use crate::door::DOOR_PATH;
use crate::hosted_sessions::MOST_A_MESSAGE_MAY_BE;

pub const REQUEST_TOO_LARGE: &str = "request.too_large";

/// axum's own limit, for every route that sets none.
const THE_DEFAULT: usize = 2 * 1024 * 1024;

/// The cap on a route, from the layers `routes.rs` and `door.rs` put on it. **A copy of those
/// numbers**, kept here because axum keeps the limit private; a route added with its own cap
/// belongs in this table too.
fn limit_of(path: &str) -> usize {
    match path {
        "/sessions/dispatch" | "/sessions/message" | "/sessions/note" | "/sessions/notes" | "/other_mac/act" => {
            MOST_A_MESSAGE_MAY_BE
        }
        "/other_macs/sessions" | "/other_macs/bug_reports" | "/other_macs/projects" | "/other_macs/event" => {
            crate::other_macs::MOST_A_MAC_LIST_MAY_BE
        }
        DOOR_PATH => 1024 * 1024,
        _ => THE_DEFAULT,
    }
}

fn megabytes(bytes: usize) -> String {
    let tenths = (bytes as f64 / (1024.0 * 1024.0) * 10.0).round() as u64;
    if tenths % 10 == 0 {
        format!("{} MB", tenths / 10)
    } else {
        format!("{}.{} MB", tenths / 10, tenths % 10)
    }
}

/// Over the whole listener: a 413 that is not already one of Fleet's own refusals becomes one.
pub(crate) async fn name_a_413(
    State(run_id): State<RunId>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    let sent: Option<usize> = request
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok());
    let response = next.run(request).await;
    let ours = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|kind| kind.starts_with("application/json"));
    if response.status() != StatusCode::PAYLOAD_TOO_LARGE || ours {
        return response;
    }
    let limit = limit_of(&path);
    let was = match sent {
        Some(bytes) => format!("this request was {}", megabytes(bytes)),
        None => "this request was larger".to_string(),
    };
    let mut error = WireError::raised(
        REQUEST_TOO_LARGE,
        format!("Fleet takes at most {} on {path}, and {was}.", megabytes(limit)),
        run_id,
    )
    .with_field("route", WireValue::Str(path))
    .with_field("limit_bytes", WireValue::Int(limit as i64));
    if let Some(bytes) = sent {
        error = error.with_field("content_length_bytes", WireValue::Int(bytes as i64));
    }
    problem(StatusCode::PAYLOAD_TOO_LARGE, &error)
}
