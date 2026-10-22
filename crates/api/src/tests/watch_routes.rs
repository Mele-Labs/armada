//! Watches over the router: each route reaches the daemon with its body whole, a body in neither
//! shape is the caller's fault, and a watch nothing holds is a 404.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{WatchList, WatchState, WatchSummary};
use tower::ServiceExt;

use crate::tests::fake::{FakeDaemon, THE_WATCH};
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served};

fn wired() -> Router {
    let events = Broadcaster::new();
    router(Served::by(FakeDaemon::new(events.clone()), run_id(), events))
}

async fn call(app: &Router, method: &str, uri: &str, body: &str) -> (StatusCode, Vec<u8>) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    let response = app.clone().oneshot(request).await.expect("an answer");
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("a body").to_bytes();
    (status, bytes.to_vec())
}

#[tokio::test]
async fn a_start_reaches_the_daemon_with_its_command_whole() {
    let app = wired();
    let body = r#"{"session_id":"job-7","name":"QA deploy","command":["scripts/deploy-state","abc; rm -rf /"],"every":"60s"}"#;
    let (status, bytes) = call(&app, "POST", "/watches/start", body).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&bytes));
    let started: WatchSummary = ipc::decode("a watch", &bytes).expect("a summary");
    assert_eq!(started.session_id.as_str(), "job-7");
    assert_eq!(started.command, ["scripts/deploy-state", "abc; rm -rf /"]);
}

#[tokio::test]
async fn a_start_in_neither_shape_is_a_400() {
    let app = wired();
    for body in [r#"{"name":"x"}"#, r#"{"name":"x","recipe":"r","every":"60s"}"#, "not json"] {
        let (status, _) = call(&app, "POST", "/watches/start", body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    }
}

#[tokio::test]
async fn weigh_and_stop_name_a_watch_and_one_nothing_holds_is_a_404() {
    let app = wired();
    let weigh = format!(r#"{{"watch":"{THE_WATCH}","verdict":"not_ours","sentence":"Another service.","parts":["unit"]}}"#);
    let (status, _) = call(&app, "POST", "/watches/weigh", &weigh).await;
    assert_eq!(status, StatusCode::OK);

    let (status, bytes) = call(&app, "POST", "/watches/stop", &format!(r#"{{"watch":"{THE_WATCH}"}}"#)).await;
    assert_eq!(status, StatusCode::OK);
    let stopped: WatchSummary = ipc::decode("a watch", &bytes).expect("a summary");
    assert_eq!((stopped.state, stopped.live), (WatchState::Stopped, false));

    let (status, bytes) = call(&app, "POST", "/watches/stop", r#"{"watch":"w-none"}"#).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(String::from_utf8_lossy(&bytes).contains("fleet.no_such_watch"));
}

#[tokio::test]
async fn a_list_is_asked_on_the_query_string() {
    let (status, bytes) = call(&wired(), "GET", "/watches?session_id=job-7", "").await;
    assert_eq!(status, StatusCode::OK);
    let list: WatchList = ipc::decode("a list", &bytes).expect("a list");
    assert!(list.watches.is_empty());
}
