//! Words read for which Job they are for, over the routes: the reading is Fleet's
//! (`fleet::tests::operator_shortlist`); this holds the route and the wire.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use ipc::ThoughtReading;
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served};

fn wired() -> (Arc<FakeDaemon>, Router) {
    let events = Broadcaster::new();
    let daemon = Arc::new(FakeDaemon::new(events.clone()));
    let app = router(Served::sharing(Arc::clone(&daemon), run_id(), events));
    (daemon, app)
}

async fn call(app: &Router, method: &str, uri: &str, body: &str) -> (StatusCode, Vec<u8>) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("a well-formed request"),
        )
        .await
        .expect("the router answers");
    let status = response.status();
    let body = http_body_util::BodyExt::collect(response.into_body())
        .await
        .expect("a body")
        .to_bytes()
        .to_vec();
    (status, body)
}

#[tokio::test]
async fn words_that_ask_for_a_route_come_back_as_a_thought_and_others_as_a_question() {
    let (_daemon, app) = wired();
    let (status, body) = call(&app, "POST", "/operator/read_thought", r#"{"text":"route this"}"#).await;
    assert_eq!(status, StatusCode::OK);
    let reading: ThoughtReading = ipc::decode("a reading", &body).expect("a reading");
    assert!(matches!(reading, ThoughtReading::Thought { target: Some(_) }), "{reading:?}");

    let (_, body) = call(&app, "POST", "/operator/read_thought", r#"{"text":"what is the time"}"#).await;
    let reading: ThoughtReading = ipc::decode("a reading", &body).expect("a reading");
    assert_eq!(reading, ThoughtReading::Question);
}
