//! The allowlist. A route is here or it is 404.

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware::{from_fn, from_fn_with_state, map_response};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::Router;

use tower_http::compression::CompressionLayer;

use crate::{
    admin, cors, fleet_client, forward, mac_ask_routes, mac_pair_routes, mac_routes, machine, pair_routes, push, reads, session_routes, setup,
    shown_address, signing, stat, thread_window,
};

/// Where Fleet is now: its port out of the runtime file, read afresh each time
/// so a Fleet that restarted is found, or the sentence saying why it is not.
pub type Fleet = Arc<dyn Fn() -> Result<u16, String> + Send + Sync>;

#[derive(Clone)]
pub struct Gateway {
    pub fleet: Fleet,
    /// The built PWA. `None`, or a directory that is not there, serves nothing.
    pub assets: Option<PathBuf>,
    pub pairing: crate::Pairing,
    pub push: push::Push,
    pub way_in: setup::WayIn,
    /// The owner's other Macs: pairing codes, keys, and the link's pace.
    pub macs: crate::Macs,
}


/// A listed path asked with a method not on the list is not listed.
async fn unlisted(answer: Response) -> Response {
    match answer.status() {
        StatusCode::METHOD_NOT_ALLOWED => StatusCode::NOT_FOUND.into_response(),
        _ => answer,
    }
}

/// Whether Fleet answers, for Bridge's Settings to say.
async fn status(State(gateway): State<Gateway>) -> Response {
    let port = match (gateway.fleet)() {
        Ok(port) => port,
        Err(why) => return (StatusCode::SERVICE_UNAVAILABLE, why).into_response(),
    };
    match fleet_client::send(port, "GET", "/alerts", None).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(why) => (StatusCode::BAD_GATEWAY, why.to_string()).into_response(),
    }
}

pub fn router(gateway: Gateway) -> Router {
    let admin = Router::new()
        .route("/admin/status", get(status))
        .route("/admin/setup", get(setup::setup))
        .route("/admin/setup/funnel", post(setup::funnel))
        .route("/admin/setup/cloudflare", post(setup::cloudflare))
        .route("/admin/macs", get(machine::this_and_paired_macs))
        .route("/admin/macs/pair/start", post(mac_pair_routes::start))
        .route("/admin/macs/pair/claim", post(mac_pair_routes::claim))
        .route("/admin/macs/pair/confirm", post(mac_pair_routes::confirm))
        .route("/admin/macs/forget", post(mac_pair_routes::forget))
        .route("/admin/macs/found", get(mac_ask_routes::found))
        .route("/admin/macs/pair/ask", post(mac_ask_routes::ask))
        .route("/admin/pair/start", post(pair_routes::start))
        .route("/admin/pair/pending", get(pair_routes::pending))
        .route("/admin/pair/confirm", post(pair_routes::confirm))
        .route("/admin/pair/decline", post(pair_routes::decline))
        .route("/admin/devices", get(pair_routes::devices))
        .route("/admin/devices/:id", delete(pair_routes::unpair))
        .layer(from_fn(admin::loopback_only));

    // The stream and the push routes stay uncompressed, so nothing holds a frame back to fill a block.
    let plain = Router::new()
        .route("/api/live", get(reads::live))
        .route("/api/mac/live", get(mac_routes::live))
        .route("/api/push/subscribe", post(push::subscribe))
        .route("/api/push/key", get(push::key))
        .route("/api/push/vapid_private", get(push::vapid_private));

    // Issues 1995, 1996, 2001, 2004 and 2006 fill these in.
    Router::new()
        // GET is the app's own Pair page, which Bridge's QR opens; POST is the claim.
        .route("/pair", get(stat::serve).post(pair_routes::claim))
        // Another Mac pairing, unsigned: the claim carries the key its calls are signed with after.
        .route("/pair/mac", post(mac_pair_routes::claimed_here))
        .route("/pair/mac/back", post(mac_pair_routes::back))
        // Another Mac on his account: who this is, its ask to pair, and word that the ask was declined.
        .route("/pair/mac/hello", get(mac_ask_routes::hello))
        .route("/pair/mac/ask", post(mac_ask_routes::asked_here))
        .route("/pair/mac/declined", post(mac_pair_routes::declined))
        // Only a linked Mac, and never a phone: `signing::signed`.
        .route("/api/mac/hello", get(mac_routes::hello))
        .route("/api/mac/sessions", get(mac_routes::read))
        .route("/api/mac/bug_reports", get(mac_routes::read))
        .route("/api/mac/projects", get(mac_routes::read))
        .route("/api/mac/project_act", post(mac_routes::project_act))
        .route("/api/mac/act", post(mac_routes::act))
        .route("/api/mac/thread", get(mac_routes::read))
        .route("/api/mac/forget", post(mac_pair_routes::forgotten))
        .route("/api/needs", get(reads::needs))
        .route("/api/sessions", get(session_routes::sessions).post(session_routes::dispatch))
        .route("/api/sessions/dispatch", post(session_routes::operator_dispatch))
        .route("/api/repositories", get(session_routes::repository_labels))
        // What waits for a Mac asleep, and taking it back: Fleet's own lists and refusals, as they are.
        .route("/api/queued", get(forward::forward))
        .route("/api/queued/take_back", post(forward::forward))
        .route("/api/sessions/answer", post(session_routes::answer))
        .route("/api/machine", get(machine::machine))
        .route("/api/sessions/one", get(thread_window::one))
        .route("/api/sessions/shown", get(forward::forward))
        .route("/api/sessions/shown/address", get(shown_address::address))
        .route("/api/sessions/kept_styles", get(forward::forward))
        .route("/api/sessions/ask/answer", post(forward::forward))
        .route("/api/sessions/waiting/answer", post(forward::forward))
        .route("/api/sessions/plan/approve", post(forward::forward))
        .route("/api/sessions/plan/changes", post(forward::forward))
        .route("/api/sessions/sketch/answer", post(forward::forward))
        .route("/api/sessions/document/answer", post(forward::forward))
        .route("/api/sessions/message", post(forward::forward))
        .route("/api/sessions/fork", post(session_routes::fork))
        .route("/api/sessions/begin", post(forward::forward))
        .route("/api/sessions/stop", post(forward::forward))
        .route("/api/sessions/pause", post(forward::forward))
        .route("/api/sessions/resume", post(forward::forward))
        .route("/api/sessions/close", post(forward::forward))
        .route("/api/sessions/tune", post(forward::forward))
        .route("/api/sessions/run/start", post(forward::forward))
        .route("/api/sessions/waiting/dismiss", post(forward::forward))
        .route("/api/sessions/change_waiting", post(forward::forward))
        .route("/api/sessions/start_waiting", post(forward::forward))
        .route("/api/plan_usage", get(forward::forward))
        // Bridge's model list reads Fleet's `/models` too, so the path maps as it is.
        .route("/api/models", get(forward::forward))
        // A long Job's thread is mostly repeated JSON: gzip takes its newest rows from 768 KB to 96 KB.
        .layer(CompressionLayer::new().gzip(true))
        .merge(plain)
        .layer(from_fn_with_state(gateway.pairing.clone(), signing::signed))
        // After the signed layer, so it is unsigned: a video element cannot sign. See `shown_address`.
        .route("/shown/:ticket", get(shown_address::serve))
        .merge(admin)
        .fallback(stat::serve)
        .layer(map_response(unlisted))
        .layer(from_fn(cors::cors))
        .with_state(gateway)
}
