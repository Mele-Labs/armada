//! A Job's shell over the router: the person's open reaches the daemon with its body whole.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
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
async fn an_open_reaches_the_daemon_and_answers_the_shells_run_id() {
    let app = wired();
    let (status, bytes) = call(&app, "POST", "/sessions/terminal/open", r#"{"session_id":"job-7","cols":120,"rows":40}"#).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&bytes));
    let open: ipc::SessionRunUnderway = ipc::decode("a shell", &bytes).expect("a run id");
    assert_eq!(open.run_id, "shell-of-job-7");

    let (status, _) = call(&app, "POST", "/sessions/terminal/open", r#"{"cols":120}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "a Job is named");
}
