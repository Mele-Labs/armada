//! `send_session_message` over the router: a message carrying a screenshot is
//! past axum's 2 MB default, and reaches the daemon rather than a 413.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served};

#[tokio::test]
async fn a_message_past_two_megabytes_reaches_the_daemon() {
    let events = Broadcaster::new();
    let app = router(Served::by(FakeDaemon::new(events.clone()), run_id(), events));
    let body = format!(
        r#"{{"session_id":"c279e883-d316-4202-8134-76ea8b531835","text":"{}"}}"#,
        "a".repeat(5 * 1024 * 1024)
    );
    let request = Request::builder()
        .method("POST")
        .uri("/sessions/message")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .expect("a well-formed request");
    let response = app.oneshot(request).await.expect("an answer");
    assert_ne!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let said = response.into_body().collect().await.expect("a body").to_bytes();
    assert!(
        String::from_utf8_lossy(&said).contains("fake.no_hosted_sessions"),
        "the fake daemon is what answered"
    );
}
