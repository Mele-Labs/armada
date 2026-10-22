//! The allowlist. A route is here or it is 404.

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware::{from_fn, from_fn_with_state, map_response};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::Router;

use crate::{admin, cors, fleet_client, forward, machine, pair_routes, push, reads, session_routes, signing, stat};

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
        .route("/admin/pair/start", post(pair_routes::start))
        .route("/admin/pair/pending", get(pair_routes::pending))
        .route("/admin/pair/confirm", post(pair_routes::confirm))
        .route("/admin/devices", get(pair_routes::devices))
        .route("/admin/devices/:id", delete(pair_routes::unpair))
        .layer(from_fn(admin::loopback_only));

    // Issues 1995, 1996, 2001, 2004 and 2006 fill these in.
    Router::new()
        // GET is the app's own Pair page, which Bridge's QR opens; POST is the claim.
        .route("/pair", get(stat::serve).post(pair_routes::claim))
        .route("/api/needs", get(reads::needs))
        .route("/api/live", get(reads::live))
        .route("/api/push/subscribe", post(push::subscribe))
        .route("/api/push/key", get(push::key))
        .route("/api/push/vapid_private", get(push::vapid_private))
        .route("/api/sessions", get(session_routes::sessions).post(session_routes::dispatch))
        .route("/api/repositories", get(session_routes::repository_labels))
        .route("/api/sessions/answer", post(session_routes::answer))
        .route("/api/machine", get(machine::machine))
        .route("/api/sessions/one", get(forward::forward))
        .route("/api/sessions/shown", get(forward::forward))
        .route("/api/sessions/ask/answer", post(forward::forward))
        .route("/api/sessions/waiting/answer", post(forward::forward))
        .route("/api/sessions/plan/approve", post(forward::forward))
        .route("/api/sessions/plan/changes", post(forward::forward))
        .route("/api/sessions/message", post(forward::forward))
        .layer(from_fn_with_state(gateway.pairing.clone(), signing::signed))
        .merge(admin)
        .fallback(stat::serve)
        .layer(map_response(unlisted))
        .layer(from_fn(cors::cors))
        .with_state(gateway)
}
