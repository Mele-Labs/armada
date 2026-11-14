//! The routes of the two things a Job may send the Operator, over the router: `raise_conflict` and `ask_held`.
//! The routes are hand-written, so the path and the method are what is held here.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served};

fn app() -> Router {
    let events = Broadcaster::new();
    router(Served::by(FakeDaemon::new(events.clone()), run_id(), events))
}

async fn call(app: &Router, method: &str, uri: &str, body: &str) -> (StatusCode, String) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    let response = app.clone().oneshot(request).await.expect("an answer");
    let status = response.status();
    let said = response.into_body().collect().await.expect("a body").to_bytes();
    (status, String::from_utf8_lossy(&said).into_owned())
}

#[tokio::test]
async fn a_conflict_is_raised_over_the_router_and_its_id_comes_back() {
    let sent = r#"{"with":"s-0a1b2c3d","files":["a.rs"],"wire":[],"mine":"m","theirs":"t","breaks":"b"}"#;
    let (status, said) = call(&app(), "POST", "/sessions/raise_conflict", sent).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert_eq!(said, r#"{"id":"conflict:s-a:s-b","folded":false}"#);
}

#[tokio::test]
async fn a_conflict_without_what_breaks_is_undecodable_and_names_its_shape() {
    let (status, said) = call(&app(), "POST", "/sessions/raise_conflict", r#"{"with":"s-0a1b2c3d","mine":"m","theirs":"t"}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{said}");
    assert!(said.contains("breaks"), "{said}");
}

#[tokio::test]
async fn a_landing_question_is_asked_over_the_router_and_its_id_comes_back() {
    let (status, said) = call(&app(), "POST", "/sessions/ask_held", r#"{"pull":12}"#).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert_eq!(said, r#"{"id":"held:repo:1"}"#);
}

#[tokio::test]
async fn a_landing_question_with_a_pull_that_is_no_number_is_undecodable() {
    let (status, said) = call(&app(), "POST", "/sessions/ask_held", r#"{"pull":"twelve"}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{said}");
}
