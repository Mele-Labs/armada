//! His other Macs over the router: what this Mac's Phone Gateway reads to send the other Mac, and what it hands
//! in from it. **Mounted whole by `crate::routes`**, which sits at the line count the gate reports.

use axum::body::Bytes;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::extract::{DefaultBodyLimit, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::Router;
use ipc::{Delivered, Missed, StreamMessage};
use serde::Deserialize;

use crate::answers::{answer, refused, undecodable};
use crate::daemon::{Commands, HostedSessions, OtherMacs, PullRequests, Queries, Sessions};
use crate::served::Served;
use crate::sockets::{resync, send};
use crate::stream::Next;

/// The most a list handed in from another Mac may weigh: that Mac's whole list of Jobs, projects or reports, or one
/// event, which carries a Job's row whole. **Its every attachment rides in a Job's row**, so on 19 Nov one Mac's
/// Job list was 5 MB, past axum's 2 MB default, and nothing after it crossed.
pub(crate) const MOST_A_MAC_LIST_MAY_BE: usize = 64 * 1024 * 1024;

pub(crate) fn mounted<D: Commands + OtherMacs + Queries + HostedSessions + Sessions + PullRequests + crate::daemon::Story>() -> Router<Served<D>> {
    Router::new()
        .route("/other_mac/sessions", get(share_sessions::<D>))
        .route("/other_mac/bug_reports", get(share_bug_reports::<D>))
        .route("/other_mac/preview_offers", get(share_preview_offers::<D>))
        .route("/other_mac/projects", get(share_projects::<D>))
        .route("/other_mac/project_act", post(project_act_from_mac::<D>))
        .route("/other_mac/act", post(crate::mac_acts::act_from_mac::<D>))
        .route("/other_mac/refresh_claim", post(hold_refresh_claim::<D>))
        .route("/other_mac/kit", get(share_kit::<D>))
        .route("/other_mac/kit_settle", post(kit_settle_from_mac::<D>))
        .route("/other_macs/kit", put(hold_other_mac_kit::<D>).layer(DefaultBodyLimit::max(MOST_A_MAC_LIST_MAY_BE)))
        .route("/other_mac/conversation", get(crate::mac_acts::share_job_conversation::<D>))
        .route("/sessions/move_to_mac", post(crate::mac_acts::move_job_to_mac::<D>))
        .route("/queued", get(crate::mac_acts::list_queued::<D>))
        .route("/queued/take_back", post(crate::mac_acts::take_back_queued::<D>))
        .route("/other_mac/events", get(share_events::<D>))
        .route("/other_mac/thread", get(share_job_thread::<D>))
        .route("/other_mac/session", get(share_job_session::<D>))
        .route("/other_mac/artifacts", get(share_job_artifacts::<D>))
        .route("/other_mac/artifact_page", get(share_job_artifact_page::<D>))
        .route("/other_mac/page", get(share_job_page::<D>))
        .route("/sessions/other_mac_page", post(open_other_mac_page::<D>))
        .route("/other_macs/sessions", put(hold_other_mac_sessions::<D>).layer(DefaultBodyLimit::max(MOST_A_MAC_LIST_MAY_BE)))
        .route("/other_macs/bug_reports", put(hold_other_mac_bug_reports::<D>).layer(DefaultBodyLimit::max(MOST_A_MAC_LIST_MAY_BE)))
        .route("/other_macs/preview_offers", put(hold_other_mac_preview_offers::<D>).layer(DefaultBodyLimit::max(MOST_A_MAC_LIST_MAY_BE)))
        .route("/other_macs/projects", put(hold_other_mac_projects::<D>).layer(DefaultBodyLimit::max(MOST_A_MAC_LIST_MAY_BE)))
        .route("/other_macs/event", post(hold_other_mac_event::<D>).layer(DefaultBodyLimit::max(MOST_A_MAC_LIST_MAY_BE)))
        .route("/other_macs/forget", post(forget_other_mac::<D>))
        .route("/other_macs/claims", put(hold_mac_claims::<D>))
        .merge(crate::mac_terminals::mounted::<D>())
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

async fn share_preview_offers<D: OtherMacs>(State(served): State<Served<D>>) -> Response {
    match served.daemon().share_preview_offers().await {
        Ok(offers) => answer(StatusCode::OK, &offers, served.run_id()),
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

/// `POST /other_mac/refresh_claim`: the Refresh the Mac named in `x-armada-mac` has under way, or word it ended.
async fn hold_refresh_claim<D: OtherMacs>(State(served): State<Served<D>>, headers: axum::http::HeaderMap, body: Bytes) -> Response {
    let sent: ipc::RefreshClaimSent = match ipc::decode_body("a Refresh another Mac has under way", &body) {
        Ok(sent) => sent,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    let named = headers.get(crate::acting::MAC_HEADER).and_then(|one| one.to_str().ok()).map(str::trim).filter(|one| !one.is_empty());
    let Some(mac) = named else {
        return undecodable("a Refresh another Mac has under way names the Mac in x-armada-mac", served.run_id());
    };
    match served.daemon().hold_refresh_claim(mac.to_string(), sent).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(refusal) => refused(refusal),
    }
}

async fn share_kit<D: OtherMacs>(State(served): State<Served<D>>, headers: axum::http::HeaderMap) -> Response {
    let asking = headers.get(crate::acting::MAC_HEADER).and_then(|one| one.to_str().ok()).map(str::trim).filter(|one| !one.is_empty());
    match served.daemon().share_kit(asking.map(str::to_string)).await {
        Ok(offer) => answer(StatusCode::OK, &offer, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

async fn hold_other_mac_kit<D: OtherMacs>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let held: ipc::OtherMacKit = match ipc::decode_body("another Mac's Kit", &body) {
        Ok(held) => held,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().hold_other_mac_kit(held).await {
        Ok(named) => answer(StatusCode::OK, &named, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `POST /other_mac/kit_settle`: a Kit clash the Mac named in `x-armada-mac` settled.
async fn kit_settle_from_mac<D: OtherMacs>(State(served): State<Served<D>>, headers: axum::http::HeaderMap, body: Bytes) -> Response {
    let settled: ipc::KitSettled = match ipc::decode_body("a Kit clash another Mac settled", &body) {
        Ok(settled) => settled,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    let named = headers.get(crate::acting::MAC_HEADER).and_then(|one| one.to_str().ok()).map(str::trim).filter(|one| !one.is_empty());
    let Some(mac) = named else {
        return undecodable("a Kit clash another Mac settled names the Mac in x-armada-mac", served.run_id());
    };
    match served.daemon().kit_settle_from_mac(mac.to_string(), settled).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
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

/// `?session_id=`, `?last=` and `?before=` on `share_job_session`, as on `get_session`.
#[derive(Deserialize)]
struct WhichSession {
    session_id: String,
    last: Option<usize>,
    before: Option<String>,
}

async fn share_job_session<D: OtherMacs>(State(served): State<Served<D>>, Query(which): Query<WhichSession>) -> Response {
    let window = which.last.map(|last| ipc::SessionWindow { last, before: which.before });
    match served.shared().share_job_session(ipc::SessionId::carried(which.session_id), window).await {
        Ok(thread) => answer(StatusCode::OK, &thread, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?session_id=` on `share_job_artifacts`.
#[derive(Deserialize)]
struct WhichArtifacts {
    session_id: String,
}

async fn share_job_artifacts<D: OtherMacs>(State(served): State<Served<D>>, Query(which): Query<WhichArtifacts>) -> Response {
    match served.daemon().share_job_artifacts(ipc::SessionId::carried(which.session_id)).await {
        Ok(artifacts) => answer(StatusCode::OK, &artifacts, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}

/// `?session_id=`, `?artifact=` and `?revision=` on `share_job_artifact_page`, as on `get_artifact_page`.
#[derive(Deserialize)]
struct WhichArtifactPage {
    session_id: String,
    artifact: String,
    revision: Option<u32>,
}

async fn share_job_artifact_page<D: OtherMacs>(State(served): State<Served<D>>, Query(which): Query<WhichArtifactPage>) -> Response {
    let asked = served.daemon().share_job_artifact_page(ipc::SessionId::carried(which.session_id), which.artifact, which.revision).await;
    match asked {
        Ok(page) => answer(StatusCode::OK, &page, served.run_id()),
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

async fn hold_other_mac_preview_offers<D: OtherMacs>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let held: ipc::OtherMacPreviewOffers = match ipc::decode_body("another Mac's preview offers", &body) {
        Ok(held) => held,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.shared().hold_other_mac_preview_offers(held).await {
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

async fn hold_mac_claims<D: OtherMacs>(State(served): State<Served<D>>, body: Bytes) -> Response {
    let held: ipc::MacClaimsWaiting = match ipc::decode_body("the Macs waiting to pair", &body) {
        Ok(held) => held,
        Err(why) => return undecodable(&why.to_string(), served.run_id()),
    };
    match served.daemon().hold_mac_claims(held).await {
        Ok(held) => answer(StatusCode::OK, &held, served.run_id()),
        Err(refusal) => refused(refusal),
    }
}
