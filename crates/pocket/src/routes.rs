//! The allowlist. A route is here or it is 404.

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware::{from_fn, from_fn_with_state, map_response};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::Router;

use crate::{admin, fleet_client, pair_routes, signing, stat};

/// Where Fleet is now: its port out of the runtime file, read afresh each time
/// so a Fleet that restarted is found, or the sentence saying why it is not.
pub type Fleet = Arc<dyn Fn() -> Result<u16, String> + Send + Sync>;

#[derive(Clone)]
pub struct Gateway {
    pub fleet: Fleet,
    /// The built PWA. `None`, or a directory that is not there, serves nothing.
    pub assets: Option<PathBuf>,
    pub pairing: crate::Pairing,
}

/// A route whose issue has not landed.
async fn later() -> Response {
    (StatusCode::NOT_IMPLEMENTED, "This route is not built yet.").into_response()
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
        .route("/admin/pair/confirm", post(pair_routes::confirm))
        .route("/admin/devices", get(pair_routes::devices))
        .route("/admin/devices/:id", delete(pair_routes::unpair))
        .layer(from_fn(admin::loopback_only));

    // Issues 1995, 1996, 2001, 2004 and 2006 fill these in.
    Router::new()
        .route("/pair", post(pair_routes::claim))
        .route("/api/needs", get(later))
        .route("/api/jobs", get(later).post(later))
        .route("/api/jobs/:id", get(later))
        .route("/api/live", get(later))
        .route("/api/push/subscribe", post(later))
        .route("/api/jobs/:id/approve", post(later))
        .route("/api/jobs/:id/redirect", post(later))
        .route("/api/jobs/:id/restart_step", post(later))
        .route("/api/jobs/:id/kill", post(later))
        .route("/api/jobs/:id/redispatch", post(later))
        .route("/api/jobs/:id/approve_review", post(later))
        .route("/api/jobs/:id/request_changes", post(later))
        .route("/api/sessions", get(later))
        .route("/api/sessions/answer", post(later))
        .layer(from_fn_with_state(gateway.pairing.clone(), signing::signed))
        .merge(admin)
        .fallback(stat::serve)
        .layer(map_response(unlisted))
        .with_state(gateway)
}
