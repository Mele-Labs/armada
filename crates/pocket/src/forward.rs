//! The thread and reply routes: each a listed `/api/<path>` sent to Fleet's
//! `/<path>` with its query and body as the phone signed them, and Fleet's
//! status, type and body handed back as they came.
//!
//! **Nothing here reads the body.** Fleet decodes it and refuses what it does
//! not take, so a field Fleet adds reaches the phone without a change here.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{header, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};

use crate::reads::{port, refusal, NOT_ANSWERING};
use crate::{fleet_client, Gateway};

/// Fleet's path for a listed phone path: the same, without `/api`.
pub(crate) fn fleet_target(uri: &Uri) -> &str {
    let whole = uri.path_and_query().map_or(uri.path(), |p| p.as_str());
    whole.strip_prefix("/api").unwrap_or(whole)
}

pub async fn forward(State(gateway): State<Gateway>, method: Method, uri: Uri, body: Bytes) -> Response {
    send_to(&gateway, &method, fleet_target(&uri), &body).await
}

/// [`forward`] to a Fleet path the phone's own does not map to: its body only for a POST.
pub(crate) async fn send_to(gateway: &Gateway, method: &Method, target: &str, body: &[u8]) -> Response {
    let port = match port(gateway) {
        Ok(port) => port,
        Err(refused) => return refused,
    };
    let sent = (*method == Method::POST).then_some(body);
    let answer = match fleet_client::send(port, method.as_str(), target, sent).await {
        Ok(answer) => answer,
        Err(_) => return refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_ANSWERING),
    };
    let status = StatusCode::from_u16(answer.status).unwrap_or(StatusCode::BAD_GATEWAY);
    let kind = answer.content_type.unwrap_or_else(|| String::from("application/json"));
    (status, [(header::CONTENT_TYPE, kind)], answer.body).into_response()
}
