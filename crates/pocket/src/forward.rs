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
use axum::routing::{get, post};
use axum::Router;

use crate::reads::{port, refusal, NOT_ANSWERING};
use crate::{fleet_client, Gateway};

/// Every phone path handed to Fleet as it is, with its method. **Fleet's phone filter rules on each by its path
/// without `/api`**, and a Fleet test asks each through a real Gateway, so a path added here with no ruling fails
/// there and not on his phone, as Refresh preview did: an empty 404 the phone read as a Mac not answering.
pub const FORWARDED: &[(Method, &str)] = &[
    // What waits for a Mac asleep, and taking it back: Fleet's own lists and refusals, as they are.
    (Method::GET, "/api/queued"),
    (Method::POST, "/api/queued/take_back"),
    (Method::GET, "/api/sessions/shown"),
    (Method::GET, "/api/sessions/kept_styles"),
    (Method::POST, "/api/sessions/ask/answer"),
    (Method::POST, "/api/sessions/waiting/answer"),
    (Method::POST, "/api/sessions/plan/approve"),
    (Method::POST, "/api/sessions/plan/changes"),
    (Method::POST, "/api/sessions/sketch/answer"),
    (Method::POST, "/api/sessions/document/answer"),
    (Method::POST, "/api/sessions/message"),
    (Method::POST, "/api/sessions/begin"),
    (Method::POST, "/api/sessions/stop"),
    (Method::POST, "/api/sessions/pause"),
    (Method::POST, "/api/sessions/resume"),
    (Method::POST, "/api/sessions/close"),
    (Method::POST, "/api/sessions/tune"),
    (Method::POST, "/api/sessions/run/start"),
    (Method::POST, "/api/sessions/waiting/dismiss"),
    (Method::POST, "/api/sessions/change_waiting"),
    (Method::POST, "/api/sessions/start_waiting"),
    // A Job's artifacts, drawn and answered on the phone. The page is sealed `text/html`, and `forward` hands its type back.
    (Method::GET, "/api/sessions/artifacts"),
    (Method::GET, "/api/sessions/artifacts/page"),
    (Method::POST, "/api/sessions/artifacts/part/answer"),
    (Method::POST, "/api/sessions/artifacts/press"),
    (Method::POST, "/api/sessions/artifacts/vote"),
    (Method::POST, "/api/sessions/artifacts/seen"),
    (Method::GET, "/api/sessions/thread"),
    (Method::GET, "/api/sessions/repository_file"),
    (Method::POST, "/api/sessions/ready_to_try/answer"),
    // To review: every Job's asks in one list, and the two answers no other phone route carries.
    (Method::GET, "/api/sessions/to_review"),
    (Method::POST, "/api/sessions/items/answer"),
    (Method::POST, "/api/sessions/waiting/done"),
    // The Judge on demand: a press, a note sent or dismissed, a reading opened, and the judge types a press offers.
    (Method::POST, "/api/sessions/judge"),
    (Method::POST, "/api/sessions/judge/send"),
    (Method::POST, "/api/sessions/judge/dismiss"),
    (Method::POST, "/api/sessions/judge/read"),
    (Method::GET, "/api/judge_types/:repository"),
    (Method::GET, "/api/plan_usage"),
    // Refresh preview: Fleet's own refusals and its both-Macs refresh, as they are.
    (Method::POST, "/api/fleet/build/change"),
    // Bridge's model list reads Fleet's `/models` too, so the path maps as it is.
    (Method::GET, "/api/models"),
];

/// [`FORWARDED`], each to [`forward`].
pub(crate) fn routes() -> Router<Gateway> {
    FORWARDED.iter().fold(Router::new(), |router, (method, path)| {
        let handler = if *method == Method::GET { get(forward) } else { post(forward) };
        router.route(path, handler)
    })
}

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
