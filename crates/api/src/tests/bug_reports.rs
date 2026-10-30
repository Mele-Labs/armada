//! The bug report routes over the router: the agent's `report_bug` and Bridge's list, keep and drop.
//! The routes are hand-written, so the path, the method and the id in the path are what is held here.

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
async fn a_bug_is_reported_over_the_router_and_its_id_comes_back() {
    let (status, said) = call(&app(), "POST", "/sessions/report_bug", r#"{"title":"T","what":"W","steps":["a"],"errors":[],"logs":[]}"#).await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert_eq!(said, r#"{"report_id":"report"}"#);
}

#[tokio::test]
async fn a_bug_report_without_a_title_is_undecodable_and_names_its_shape() {
    let (status, said) = call(&app(), "POST", "/sessions/report_bug", r#"{"what":"W"}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{said}");
    assert!(said.contains("title"), "{said}");
}

#[tokio::test]
async fn the_bug_reports_are_listed_over_the_router() {
    let (status, said) = call(&app(), "GET", "/bug_reports", "").await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert_eq!(said, r#"{"reports":[]}"#);
}

#[tokio::test]
async fn a_bug_report_is_kept_over_the_router_by_the_id_in_its_path() {
    let (status, said) = call(&app(), "POST", "/bug_reports/r-7/keep", "").await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert!(said.contains(r#""id":"r-7""#), "{said}");
}

#[tokio::test]
async fn a_bug_report_is_dropped_over_the_router_by_the_id_in_its_path() {
    let (status, said) = call(&app(), "POST", "/bug_reports/r-7/drop", "").await;
    assert_eq!(status, StatusCode::OK, "{said}");
    assert_eq!(said, r#"{"id":"r-7"}"#);
}
