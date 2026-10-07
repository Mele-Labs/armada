//! A command held while Fleet reconciles: let through when it finishes,
//! refused with `api.not_reconciled` when it never will.

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::WireError;
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Reconciliation, Served};

fn wired(reconciliation: &Reconciliation) -> Router {
    let events = Broadcaster::new();
    router(
        Served::by(FakeDaemon::new(events.clone()), run_id(), events).reconciling(reconciliation),
    )
}

async fn command(app: &Router) -> (StatusCode, Vec<u8>) {
    let request = Request::builder()
        .method("POST")
        .uri("/preferences/save")
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .expect("a well-formed request");
    let response = app.clone().oneshot(request).await.expect("an answer");
    let status = response.status();
    let body = response.into_body().collect().await.expect("a body");
    (status, body.to_bytes().to_vec())
}

#[tokio::test]
async fn a_command_held_is_refused_when_reconciliation_is_dropped_unfinished() {
    let reconciliation = Reconciliation::begun();
    let app = wired(&reconciliation);
    let held = tokio::spawn(async move { command(&app).await });

    // Still waiting: nothing has finished and nothing has been dropped.
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!held.is_finished(), "a command ran mid-reconciliation");

    // What a failed or aborted reconcile does: its holder goes away.
    drop(reconciliation);
    let (status, body) = tokio::time::timeout(Duration::from_secs(5), held)
        .await
        .expect("a held command is answered once nothing can finish")
        .expect("the request task");
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    let error: WireError = ipc::decode("an error", &body).expect("the usual error shape");
    assert_eq!(error.code, "api.not_reconciled");
}

#[tokio::test]
async fn a_command_held_runs_once_reconciliation_finishes() {
    let reconciliation = Reconciliation::begun();
    let app = wired(&reconciliation);
    let held = tokio::spawn(async move { command(&app).await });
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!held.is_finished(), "a command ran mid-reconciliation");

    reconciliation.finished();
    let (status, _) = tokio::time::timeout(Duration::from_secs(5), held)
        .await
        .expect("a held command is let through")
        .expect("the request task");
    assert_ne!(status, StatusCode::SERVICE_UNAVAILABLE);
}
