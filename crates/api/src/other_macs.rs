//! His other Macs over the router: what this Mac's Phone Gateway reads to send the other Mac, and what it hands
//! in from it. **Mounted whole by `crate::routes`**, which sits at the line count the gate reports.

use axum::body::Bytes;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::Router;
use ipc::{Delivered, Missed, StreamMessage};
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{HostedSessions, OtherMacs, PullRequests, Queries, Sessions};
use crate::served::Served;
use crate::sockets::{resync, send};
use crate::stream::Next;

pub(crate) fn mounted<D: OtherMacs + Queries + HostedSessions + Sessions + PullRequests>() -> Router<Served<D>> {
    Router::new()
        .route("/other_mac/sessions", get(share_sessions::<D>))
        .route("/other_mac/bug_reports", get(share_bug_reports::<D>))
        .route("/other_mac/projects", get(share_projects::<D>))
        .route("/other_mac/project_act", post(project_act_from_mac::<D>))
        .route("/other_mac/act", post(crate::mac_acts::act_from_mac::<D>))
        .route("/queued", get(crate::mac_acts::list_queued::<D>))
        .route("/queued/take_back", post(crate::mac_acts::take_back_queued::<D>))
        .route("/other_mac/events", get(share_events::<D>))
        .route("/other_mac/thread", get(share_job_thread::<D>))
        .route("/other_mac/page", get(share_job_page::<D>))
        .route("/sessions/other_mac_page", post(open_other_mac_page::<D>))
        .route("/other_macs/sessions", put(hold_other_mac_sessions::<D>))
        .route("/other_macs/bug_reports", put(hold_other_mac_bug_reports::<D>))
        .route("/other_macs/projects", put(hold_other_mac_projects::<D>))
        .route("/other_macs/event", post(hold_other_mac_event::<D>))
        .route("/other_macs/forget", post(forget_other_mac::<D>))
}

async fn share_sessions<D: OtherMacs>(State(served): State<Served<D>>) -> Response {
    match served.daemon().share_sessions().await {
        Ok(list) => answer(StatusCode::OK, &list, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

async fn share_bug_reports<D: OtherMacs>(State(served): State<Served<D>>) -> Response {
    match served.daemon().share_bug_reports().await {
        Ok(reports) => answer(StatusCode::OK, &reports, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

async fn share_projects<D: OtherMacs>(State(served): State<Served<D>>) -> Response {
    match served.daemon().share_projects().await {
        Ok(projects) => answer(StatusCode::OK, &projects, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

async fn project_act_from_mac<D: OtherMacs>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let sent: ipc::ProjectActFromMac = match ipc::decode_body("a task act from another Mac", &body) {
        Ok(sent) => sent,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().project_act_from_mac(sent).await {
        Ok(projects) => answer(StatusCode::OK, &projects, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `/events`, each event passed through the share filter: framed alike, so the Gateway reads both one way.
async fn share_events<D: OtherMacs + Queries>(State(served): State<Served<D>>, upgrade: WebSocketUpgrade) -> Response {
    upgrade.on_upgrade(move |socket| watch(socket, served))
}

async fn watch<D: OtherMacs + Queries>(mut socket: WebSocket, served: Served<D>) {
    // `/events`' order: subscribe, then resync.
    let mut subscription = served.events().subscribe();
    if !resync(&mut socket, &served).await {
        return;
    }
    while let Some(next) = subscription.next().await {
        let delivered = match next {
            Next::Send(Delivered { cursor, event }) => match served.daemon().share_event(event).await {
                Some(event) => send(&mut socket, &StreamMessage::Event(Delivered { cursor, event })).await,
                None => true,
            },
            Next::Missed(dropped) => {
                send(&mut socket, &StreamMessage::Missed(Missed { dropped })).await && resync(&mut socket, &served).await
            }
        };
        if !delivered {
            return;
        }
    }
}

/// `?session_id=`, `?before=`, `?limit=` and `?upto=` on `share_job_thread`, as on `get_job_thread`.
#[derive(Deserialize)]
struct WhichThread {
    session_id: String,
    before: Option<String>,
    limit: Option<u32>,
    upto: Option<String>,
}

async fn share_job_thread<D: OtherMacs>(State(served): State<Served<D>>, Query(which): Query<WhichThread>) -> Response {
    let read = ipc::JobThreadRead {
        session_id: ipc::SessionId::carried(which.session_id),
        before: which.before,
        limit: which.limit,
        upto: which.upto,
    };
    match served.shared().share_job_thread(read).await {
        Ok(thread) => answer(StatusCode::OK, &thread, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?session_id=` and `?port=` on `share_job_page`.
#[derive(Deserialize)]
struct WhichPage {
    session_id: String,
    port: u16,
}

async fn share_job_page<D: OtherMacs>(State(served): State<Served<D>>, Query(which): Query<WhichPage>) -> Response {
    match served.daemon().share_job_page(ipc::SessionId::carried(which.session_id), which.port).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(refusal) => refused(refusal),
    }
}

async fn open_other_mac_page<D: OtherMacs>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let open: ipc::OpenOtherMacPage = match ipc::decode_body("a page of another Mac's Job to open", &body) {
        Ok(open) => open,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().open_other_mac_page(open).await {
        Ok(opened) => answer(StatusCode::OK, &opened, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

async fn hold_other_mac_sessions<D: OtherMacs>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let held: ipc::OtherMacSessions = match ipc::decode_body("another Mac's Job rows", &body) {
        Ok(held) => held,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().hold_other_mac_sessions(held).await {
        Ok(mac) => answer(StatusCode::OK, &mac, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

async fn hold_other_mac_projects<D: OtherMacs>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let held: ipc::OtherMacProjects = match ipc::decode_body("another Mac's projects", &body) {
        Ok(held) => held,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().hold_other_mac_projects(held).await {
        Ok(mac) => answer(StatusCode::OK, &mac, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

async fn hold_other_mac_bug_reports<D: OtherMacs>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let held: ipc::OtherMacBugReports = match ipc::decode_body("another Mac's bug reports", &body) {
        Ok(held) => held,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().hold_other_mac_bug_reports(held).await {
        Ok(mac) => answer(StatusCode::OK, &mac, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

async fn hold_other_mac_event<D: OtherMacs>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let event: ipc::OtherMacEvent = match ipc::decode_body("an event from another Mac", &body) {
        Ok(event) => event,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().hold_other_mac_event(event).await {
        Ok(mac) => answer(StatusCode::OK, &mac, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

async fn forget_other_mac<D: OtherMacs>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let named: ipc::MacNamed = match ipc::decode_body("the Mac to forget", &body) {
        Ok(named) => named,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().forget_other_mac(named).await {
        Ok(mac) => answer(StatusCode::OK, &mac, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
