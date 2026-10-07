//! What a listener does with a command while Fleet is still reconciling.
//!
//! **A read and the health answer at once; a command waits.** Reconciliation
//! moves Jobs, and a command landing between two of its moves would race them.
//! Waiting is what every request did before the listener served first, so a
//! command and a Drone's tool call see no new refusal.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use ipc::{RunId, WireError};
use tokio::sync::watch;

use crate::answers::problem;

/// Raised when reconciliation ended without finishing, so a command was
/// never let through. The process is on its way down.
pub(crate) const NOT_RECONCILED: &str = "api.not_reconciled";

/// Held by whoever reconciles, and told when it is over.
#[derive(Clone)]
pub struct Reconciliation(Arc<watch::Sender<bool>>);

impl Reconciliation {
    pub fn begun() -> Reconciliation {
        Reconciliation(Arc::new(watch::channel(false).0))
    }

    /// Commands waiting are let through, and every later one passes at once.
    pub fn finished(&self) {
        self.0.send_replace(true);
    }

    pub(crate) fn watched(&self) -> watch::Receiver<bool> {
        self.0.subscribe()
    }
}

/// The wait, as a layer. Only a request that cannot change anything passes.
pub(crate) async fn hold_commands(
    State((mut over, run_id)): State<(watch::Receiver<bool>, RunId)>,
    request: Request,
    next: Next,
) -> Response {
    if matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    ) {
        return next.run(request).await;
    }
    // Mapped out of the guard before anything else is awaited: it is not `Send`.
    let finished = over.wait_for(|over| *over).await.is_ok();
    match finished {
        true => next.run(request).await,
        false => problem(
            StatusCode::SERVICE_UNAVAILABLE,
            &WireError::raised(
                NOT_RECONCILED,
                "Fleet stopped before it finished reconciling, so this command was not run",
                run_id,
            ),
        ),
    }
}
