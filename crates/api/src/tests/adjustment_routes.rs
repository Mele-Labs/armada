//! The adjustments read over the router and at the agent's door: the window reaches the daemon as named, and
//! a Job may call it with the one field it takes.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::UsageAdjustments;
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{offered, router, Broadcaster, Served, DOOR_PATH};

fn wired() -> Router {
    let events = Broadcaster::new();
    router(Served::by(FakeDaemon::new(events.clone()), run_id(), events))
}

async fn send(app: &Router, request: Request<Body>) -> (StatusCode, Vec<u8>) {
    let response = app.clone().oneshot(request).await.expect("an answer");
    let status = response.status();
    (status, response.into_body().collect().await.expect("a body").to_bytes().to_vec())
}

#[tokio::test]
async fn the_adjustments_route_reaches_the_daemon_with_the_window_it_names() {
    let app = wired();
    let get = |uri: &str| Request::builder().method("GET").uri(uri).body(Body::empty()).expect("a request");

    let (status, bytes) = send(&app, get("/usage/adjustments?window=fortnight")).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&bytes));
    let read: UsageAdjustments = ipc::decode("a window's adjustments", &bytes).expect("reads");
    assert_eq!(read.settings[0].key, "fortnight");

    let (_, bytes) = send(&app, get("/usage/adjustments")).await;
    let read: UsageAdjustments = ipc::decode("a window's adjustments", &bytes).expect("reads");
    assert_eq!(read.settings[0].key, "", "the window is optional");
}

#[tokio::test]
async fn a_job_may_ask_what_would_cut_the_burn_with_the_one_field_it_takes() {
    let shape = offered().into_iter().find(|shape| shape.operation == "get_usage_adjustments").expect("offered");
    assert_eq!((shape.method, shape.path, shape.query), ("GET", "/usage/adjustments", &["window"][..]));

    let request = Request::builder()
        .method("POST")
        .uri(DOOR_PATH)
        .header("content-type", "application/json")
        .body(Body::from(ipc::job_cli::list_message()))
        .expect("a request");
    let (status, bytes) = send(&wired(), request).await;
    assert_eq!(status, StatusCode::OK);
    let acts = ipc::job_cli::acts(&bytes).expect("the list reads");
    let read = acts.iter().find(|act| act.name == "get_usage_adjustments").expect("listed");
    assert_eq!(read.fields.iter().map(|field| field.name.as_str()).collect::<Vec<_>>(), ["window"]);
    assert!(read.description.starts_with("The settings that change the token burn"), "{}", read.description);
}
