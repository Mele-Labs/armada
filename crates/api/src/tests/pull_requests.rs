//! A pull request by repository and number, over the routes. What each read and act does is
//! Fleet's (`fleet::tests::pull_requesting`, `fleet::tests::pull_refreshing`); this holds the
//! route and the wire.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::PullRequestState;
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served};

fn wired() -> Router {
    let events = Broadcaster::new();
    let daemon = Arc::new(FakeDaemon::new(events.clone()));
    router(Served::sharing(daemon, run_id(), events))
}

async fn call(app: &Router, method: &str, uri: &str) -> (StatusCode, Vec<u8>) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .body(Body::empty())
                .expect("a well-formed request"),
        )
        .await
        .expect("the router answers");
    let status = response.status();
    let body = response.into_body().collect().await.expect("a body").to_bytes().to_vec();
    (status, body)
}

#[tokio::test]
async fn a_refresh_is_a_post_on_the_pull_request_and_answers_its_state() {
    let app = wired();
    let (status, body) = call(&app, "POST", "/pull_requests/armada/7/refresh").await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let state: PullRequestState = ipc::decode("a pull request", &body).expect("a pull request");
    assert_eq!((state.manifest_id.as_str(), state.number), ("armada", 7));

    let (status, _) = call(&app, "GET", "/pull_requests/armada/7/refresh").await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "a command is never a GET");
}
