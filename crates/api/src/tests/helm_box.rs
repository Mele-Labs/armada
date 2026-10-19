//! Helm's message box over the routes: one issue read by number. What it is, is Fleet's
//! (`fleet::tests::helm_box`); this holds the route and the wire. The box's model, effort and files
//! are a Session's (`tune_session`, `get_session_file`).

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{IssueRead, IssueState};
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

async fn call(app: &Router, method: &str, uri: &str, body: &str) -> (StatusCode, Option<String>, Vec<u8>) {
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
    let kind = response
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|value| value.to_str().expect("text").to_string());
    let body = response
        .into_body()
        .collect()
        .await
        .expect("a body")
        .to_bytes()
        .to_vec();
    (status, kind, body)
}

#[tokio::test]
async fn one_issue_is_read_by_reference_open_or_closed_and_a_missing_one_is_a_404() {
    let (_daemon, app) = wired();
    let (status, _, body) = call(&app, "GET", "/issues/armada/214", "").await;
    assert_eq!(status, StatusCode::OK);
    let read: IssueRead = ipc::decode("an issue", &body).expect("an issue");
    assert_eq!((read.reference.as_str(), read.state), ("#214", IssueState::Open));

    let (_, _, body) = call(&app, "GET", "/issues/armada/215", "").await;
    let read: IssueRead = ipc::decode("an issue", &body).expect("an issue");
    assert_eq!(read.state, IssueState::Closed);
    assert_eq!((read.body.as_str(), read.author.as_str()), ("The whole body", "nick"));
    assert_eq!(read.opened_at.as_str(), "2026-10-01T09:00:00Z");
    assert_eq!(read.comments.len(), 1);
    assert_eq!(read.linked_pull_requests[0].number, 230);
    assert!(String::from_utf8_lossy(&body).contains(r#""state":"merged""#));

    let (status, _, body) = call(&app, "GET", "/issues/armada/99", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(String::from_utf8_lossy(&body).contains("fleet.no_such_issue"));
}
