//! The phone's read routes, each one Fleet's answer narrowed to [`PhoneSession`].

use std::collections::BTreeMap;

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use futures_util::stream;
use hyper::body::Bytes;
use ipc::{ManifestSummary, SessionList};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::fleet_events::Messages;
use crate::phone_sessions::PhoneSession;
use crate::{fleet_client, live, Gateway};

/// What the owner is told when Fleet cannot be reached. Each says what to do.
const NOT_RUNNING: &str = "Armada is not running on your Mac. Open it there, then try again.";
pub(crate) const NOT_ANSWERING: &str = "Armada is not answering on your Mac. Open it there, then try again.";
pub(crate) const NOT_READABLE: &str =
    "Armada answered in a way this phone cannot read. Update Armada on your Mac.";

pub(crate) fn refusal(status: StatusCode, sentence: &'static str) -> Response {
    (status, sentence).into_response()
}

pub(crate) fn port(gateway: &Gateway) -> Result<u16, Response> {
    (gateway.fleet)().map_err(|_| refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_RUNNING))
}

/// A read of Fleet, decoded as the type Fleet sends. `Ok(None)` is Fleet
/// saying the thing is not there.
async fn read<T: DeserializeOwned>(
    gateway: &Gateway,
    path: &str,
    what: &'static str,
) -> Result<Option<T>, Response> {
    let port = port(gateway)?;
    let answer = fleet_client::send(port, "GET", path, None)
        .await
        .map_err(|_| refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_ANSWERING))?;
    match answer.status {
        200 => ipc::decode(what, &answer.body)
            .map(Some)
            .map_err(|_| refusal(StatusCode::BAD_GATEWAY, NOT_READABLE)),
        404 => Ok(None),
        _ => Err(refusal(StatusCode::BAD_GATEWAY, NOT_ANSWERING)),
    }
}

pub(crate) async fn required<T: DeserializeOwned>(
    gateway: &Gateway,
    path: &str,
    what: &'static str,
) -> Result<T, Response> {
    read(gateway, path, what)
        .await?
        .ok_or_else(|| refusal(StatusCode::BAD_GATEWAY, NOT_READABLE))
}

pub(crate) fn json<T: Serialize>(value: &T) -> Response {
    match ipc::encode(value) {
        Ok(body) => ([(header::CONTENT_TYPE, "application/json")], body).into_response(),
        Err(_) => refusal(StatusCode::INTERNAL_SERVER_ERROR, NOT_READABLE),
    }
}

/// Repository labels by Manifest id. A Fleet that will not list them costs the
/// labels, not the page.
pub(crate) async fn repositories(gateway: &Gateway) -> BTreeMap<String, String> {
    read::<Vec<ManifestSummary>>(gateway, "/manifests", "a manifest list")
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
        .into_iter()
        .map(|manifest| (manifest.id.as_str().to_string(), manifest.repository))
        .collect()
}

#[derive(Serialize)]
struct Needs {
    /// Sessions waiting on the owner: a hosted one with its ask, a Terminal
    /// one as waiting and nothing more.
    sessions: Vec<PhoneSession>,
}

/// Needs you: the live Sessions waiting on the owner, oldest wait first.
pub async fn needs(State(gateway): State<Gateway>) -> Response {
    let list = match required::<SessionList>(&gateway, "/sessions?state=live", "a job list").await {
        Ok(list) => list.sessions,
        Err(refused) => return refused,
    };
    let labels = repositories(&gateway).await;
    let mut sessions: Vec<PhoneSession> = list
        .iter()
        .map(|record| {
            let repository = record
                .manifest_id
                .as_ref()
                .and_then(|id| labels.get(id.as_str()))
                .map(String::as_str);
            PhoneSession::of(record, repository, gateway.pairing.now())
        })
        .filter(|session| session.waiting)
        .collect();
    sessions.sort_by(|a, b| (&a.waiting_since, &a.id).cmp(&(&b.waiting_since, &b.id)));
    json(&Needs { sessions })
}

/// Server-Sent Events: each Session change Fleet publishes, narrowed.
pub async fn live(State(gateway): State<Gateway>) -> Response {
    let port = match port(&gateway) {
        Ok(port) => port,
        Err(refused) => return refused,
    };
    let upgraded = match fleet_client::upgrade(port, "/events").await {
        Ok(upgraded) => upgraded,
        Err(_) => return refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_ANSWERING),
    };
    let frames = stream::unfold(Messages::new(upgraded), |mut messages| async move {
        loop {
            let text = messages.next().await?;
            if let Some(frame) = live::change(&text).and_then(|change| live::frame(&change)) {
                return Some((Ok::<_, std::convert::Infallible>(Bytes::from(frame)), messages));
            }
        }
    });
    (
        [
            (header::CONTENT_TYPE, "text/event-stream"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        Body::from_stream(frames),
    )
        .into_response()
}
