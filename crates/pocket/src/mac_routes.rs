//! The routes only a linked Mac may call, each a read of this Fleet's `/other_mac/*`, which hands
//! out only what the owner shares with his other Macs. `signing::signed` keeps a phone off them.
//!
//! **A Mac whose build speaks another wire is answered 503** on all but hello and update
//! (`macs::frozen`): nothing runs half matched, and a 503 is what keeps its queue waiting.

use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::Extension;
use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use futures_util::StreamExt;

use crate::mac_client::hello_now;
use crate::signing::MacId;
use crate::reads::{json, port, refusal, NOT_ANSWERING};
use crate::{fleet_client, reads, Gateway};

/// A line with nothing on it: the other Mac reads it as the stream still being alive.
const BEATING: &str = "\n";

/// `/api/mac/<rest>` asked of Fleet as `/other_mac/<rest>`, query and all.
fn fleet_target(uri: &Uri) -> String {
    let whole = uri.path_and_query().map_or(uri.path(), |p| p.as_str());
    format!("/other_mac/{}", whole.strip_prefix("/api/mac/").unwrap_or(whole))
}

/// 503 with why, where the link to Mac `mac` is frozen.
pub(crate) fn refused_frozen(gateway: &Gateway, mac: &str) -> Option<Response> {
    let link = crate::mac_link::frozen_with(gateway, mac)?;
    let name = gateway.pairing.store.lock().unwrap().paired_mac(mac).ok().flatten().map_or_else(|| mac.to_string(), |row| row.name);
    Some((StatusCode::SERVICE_UNAVAILABLE, crate::macs::frozen_why(&link, &name)).into_response())
}

/// `GET /api/mac/sessions`, `/api/mac/bug_reports`, `/api/mac/projects` and `/api/mac/thread`: Fleet's answer as it came.
pub async fn read(State(gateway): State<Gateway>, Extension(MacId(mac)): Extension<MacId>, uri: Uri) -> Response {
    if let Some(refused) = refused_frozen(&gateway, &mac) {
        return refused;
    }
    let port = match port(&gateway) {
        Ok(port) => port,
        Err(refused) => return refused,
    };
    let answer = match fleet_client::send(port, "GET", &fleet_target(&uri), None).await {
        Ok(answer) => answer,
        Err(_) => return refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_ANSWERING),
    };
    let status = StatusCode::from_u16(answer.status).unwrap_or(StatusCode::BAD_GATEWAY);
    let kind = answer.content_type.unwrap_or_else(|| String::from("application/json"));
    (status, [(header::CONTENT_TYPE, kind)], answer.body).into_response()
}

/// `POST /api/mac/project_act`: the other Mac's edit of a task on its copy of one of this Mac's shared projects,
/// handed to Fleet's `/other_mac/project_act` as it came. Fleet answers it, and refuses it where the project is
/// not shared; its status and body are passed back as they were said.
pub async fn project_act(State(gateway): State<Gateway>, Extension(MacId(mac)): Extension<MacId>, body: Bytes) -> Response {
    if let Some(refused) = refused_frozen(&gateway, &mac) {
        return refused;
    }
    let port = match port(&gateway) {
        Ok(port) => port,
        Err(refused) => return refused,
    };
    let answer = match fleet_client::send(port, "POST", "/other_mac/project_act", Some(&body)).await {
        Ok(answer) => answer,
        Err(_) => return refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_ANSWERING),
    };
    let status = StatusCode::from_u16(answer.status).unwrap_or(StatusCode::BAD_GATEWAY);
    let kind = answer.content_type.unwrap_or_else(|| String::from("application/json"));
    (status, [(header::CONTENT_TYPE, kind)], answer.body).into_response()
}

/// `POST /api/mac/act`: the other Mac's act on a Job of this Mac, or a Job it starts here, handed to Fleet's
/// `/other_mac/act` as the Mac the signature names. Fleet answers it, and refuses it where the Job is not
/// shared; its status and body are passed back as they were said.
pub async fn act(State(gateway): State<Gateway>, Extension(MacId(mac)): Extension<MacId>, body: Bytes) -> Response {
    if let Some(refused) = refused_frozen(&gateway, &mac) {
        return refused;
    }
    let port = match port(&gateway) {
        Ok(port) => port,
        Err(refused) => return refused,
    };
    let answer = match fleet_client::send_as_mac(port, "POST", "/other_mac/act", Some(&body), &mac).await {
        Ok(answer) => answer,
        Err(_) => return refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_ANSWERING),
    };
    let status = StatusCode::from_u16(answer.status).unwrap_or(StatusCode::BAD_GATEWAY);
    let kind = answer.content_type.unwrap_or_else(|| String::from("application/json"));
    (status, [(header::CONTENT_TYPE, kind)], answer.body).into_response()
}

/// `POST /api/mac/update`: the other Mac asks this one to update, handed to Fleet's `/macs/update` as the Mac the
/// signature names. **Taken while the link is frozen**, since it is how the two builds come to match.
pub async fn update(State(gateway): State<Gateway>, Extension(MacId(mac)): Extension<MacId>, body: Bytes) -> Response {
    let port = match port(&gateway) {
        Ok(port) => port,
        Err(refused) => return refused,
    };
    let answer = match fleet_client::send_as_mac(port, "POST", "/macs/update", Some(&body), &mac).await {
        Ok(answer) => answer,
        Err(_) => return refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_ANSWERING),
    };
    let status = StatusCode::from_u16(answer.status).unwrap_or(StatusCode::BAD_GATEWAY);
    let kind = answer.content_type.unwrap_or_else(|| String::from("application/json"));
    (status, [(header::CONTENT_TYPE, kind)], answer.body).into_response()
}

/// `GET /api/mac/hello`: who this Mac is now.
pub async fn hello(State(gateway): State<Gateway>) -> Response {
    match hello_now(&gateway).await {
        Ok(me) => json(&me),
        Err(why) => (StatusCode::INTERNAL_SERVER_ERROR, why).into_response(),
    }
}

/// `GET /api/mac/live`: Fleet's `/other_mac/events`, one message a line as Fleet wrote it, and an
/// empty line whenever it is quiet for a beat. It ends when the Gateway stops, so the other Mac
/// sees the drop at once.
pub async fn live(State(gateway): State<Gateway>, Extension(MacId(mac)): Extension<MacId>) -> Response {
    if let Some(refused) = refused_frozen(&gateway, &mac) {
        return refused;
    }
    let port = match port(&gateway) {
        Ok(port) => port,
        Err(refused) => return refused,
    };
    let upgraded = match fleet_client::upgrade(port, "/other_mac/events").await {
        Ok(upgraded) => upgraded,
        Err(_) => return refusal(StatusCode::SERVICE_UNAVAILABLE, NOT_ANSWERING),
    };
    // Fleet writes JSON on one line, so a message is never split across two.
    let lines = reads::relay(upgraded, gateway.macs.timing.beat, BEATING, |text| Some(format!("{}\n", text.trim())));
    let lines = lines.take_until(gateway.macs.closed());
    (
        [(header::CONTENT_TYPE, "application/x-ndjson"), (header::CACHE_CONTROL, "no-cache")],
        Body::from_stream(lines),
    )
        .into_response()
}
