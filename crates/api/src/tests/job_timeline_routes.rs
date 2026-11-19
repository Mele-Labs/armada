//! The Job's timeline read over the router and at the agent's door: the Job reaches the daemon as named, and a
//! Job may call it with the one field it takes.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::JobTimeline;
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
async fn the_timeline_route_reaches_the_daemon_with_the_job_it_names() {
    let request = Request::builder().method("GET").uri("/usage/job_timeline?session_id=s7").body(Body::empty()).expect("a request");
    let (status, bytes) = send(&wired(), request).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&bytes));
    let timeline: JobTimeline = ipc::decode("a job's timeline", &bytes).expect("reads");
    assert_eq!(timeline.session_id, "s7");
    assert_eq!(timeline.ended, None, "an absent end is not on the wire");
    assert!(!String::from_utf8_lossy(&bytes).contains("\"ended\""), "{}", String::from_utf8_lossy(&bytes));
}

#[tokio::test]
async fn a_job_may_ask_for_a_timeline_with_the_one_field_it_takes() {
    let shape = offered().into_iter().find(|shape| shape.operation == "get_job_timeline").expect("offered");
    assert_eq!((shape.method, shape.path, shape.query), ("GET", "/usage/job_timeline", &["session_id"][..]));

    let request = Request::builder()
        .method("POST")
        .uri(DOOR_PATH)
        .header("content-type", "application/json")
        .body(Body::from(ipc::job_cli::list_message()))
        .expect("a request");
    let (status, bytes) = send(&wired(), request).await;
    assert_eq!(status, StatusCode::OK);
    let acts = ipc::job_cli::acts(&bytes).expect("the list reads");
    let timeline = acts.iter().find(|act| act.name == "get_job_timeline").expect("listed");
    assert_eq!(timeline.fields.iter().map(|field| field.name.as_str()).collect::<Vec<_>>(), ["session_id"]);
    assert!(timeline.description.starts_with("One Job's burn from start to finish"), "{}", timeline.description);
}
