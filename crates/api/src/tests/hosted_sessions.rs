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

/// The route is hand-written, so a typo in it is a runtime 500: the query must reach the daemon
/// whole, a path with a space and a `&` in it among the rest.
#[tokio::test]
async fn a_clash_file_is_asked_over_the_router_and_answers_both_patches() {
    let events = Broadcaster::new();
    let app = router(Served::by(FakeDaemon::new(events.clone()), run_id(), events));
    let ask = |query: &str| {
        Request::builder().method("GET").uri(format!("/clashes/file?{query}")).body(Body::empty()).expect("a well-formed request")
    };
    let response = app
        .clone()
        .oneshot(ask("session_id=s-1&other_kind=main&other_id=main&path=src%2Fa%20%26%20b.rs"))
        .await
        .expect("an answer");
    assert_eq!(response.status(), StatusCode::OK);
    let said = response.into_body().collect().await.expect("a body").to_bytes();
    let said = String::from_utf8_lossy(&said);
    assert!(said.contains("\"ours\"") && said.contains("\"theirs\""), "{said}");

    let unknown_kind = app.oneshot(ask("session_id=s-1&other_kind=job&other_id=j&path=a")).await.expect("an answer");
    assert_eq!(unknown_kind.status(), StatusCode::BAD_REQUEST, "a kind that is not a ClashSide");
}

/// The route is hand-written: the query, `seen` optional, must reach the daemon.
#[tokio::test]
async fn a_jobs_file_change_is_asked_over_the_router() {
    let events = Broadcaster::new();
    let app = router(Served::by(FakeDaemon::new(events.clone()), run_id(), events));
    let ask = |query: &str| {
        Request::builder().method("GET").uri(format!("/sessions/file_change?{query}")).body(Body::empty()).expect("a well-formed request")
    };
    for query in ["session_id=s-1&path=src%2Fa%20%26%20b.rs", "session_id=s-1&path=a.rs&seen=0123abcd"] {
        let response = app.clone().oneshot(ask(query)).await.expect("an answer");
        assert_eq!(response.status(), StatusCode::OK, "{query}");
        let said = response.into_body().collect().await.expect("a body").to_bytes();
        let said = String::from_utf8_lossy(&said);
        assert!(said.contains("\"stamp\"") && said.contains("\"diff\""), "{said}");
    }
    let no_path = app.oneshot(ask("session_id=s-1")).await.expect("an answer");
    assert_eq!(no_path.status(), StatusCode::BAD_REQUEST, "path is asked for");
}

/// `SendSessionMessageBody` is flat on the wire: `aim` sits beside the message's own fields, and the
/// route hands both to the daemon. Held here, at the route, because the decode and the hand-over are
/// what Fleet's own tests do not reach.
async fn post_message(body: &str) -> (StatusCode, Vec<(ipc::SendSessionMessage, Option<ipc::ModAim>)>) {
    let events = Broadcaster::new();
    let daemon = std::sync::Arc::new(FakeDaemon::new(events.clone()));
    let app = router(Served::sharing(daemon.clone(), run_id(), events));
    let request = Request::builder()
        .method("POST")
        .uri("/sessions/message")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    let response = app.oneshot(request).await.expect("an answer");
    let seen = daemon.messages_aimed.lock().expect("messages aimed").clone();
    (response.status(), seen)
}

const SESSION: &str = "c279e883-d316-4202-8134-76ea8b531835";

#[tokio::test]
async fn a_flat_message_with_an_aim_reaches_the_daemon_with_both() {
    let (status, seen) = post_message(&format!(r#"{{"session_id":"{SESSION}","text":"a darker theme","aim":"theme"}}"#)).await;
    assert_ne!(status, StatusCode::BAD_REQUEST, "a flat body with an aim decodes");
    assert_eq!(seen.len(), 1, "the daemon was handed one message");
    assert_eq!(seen[0].0.text, "a darker theme");
    assert_eq!(seen[0].0.session_id.as_str(), SESSION);
    assert_eq!(seen[0].1, Some(ipc::ModAim::Theme), "the aim travels beside the message");
}

#[tokio::test]
async fn a_flat_message_without_an_aim_reaches_the_daemon_with_none() {
    let (status, seen) = post_message(&format!(r#"{{"session_id":"{SESSION}","text":"hello"}}"#)).await;
    assert_ne!(status, StatusCode::BAD_REQUEST, "aim is optional");
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].0.text, "hello");
    assert_eq!(seen[0].1, None);
}

#[tokio::test]
async fn an_aim_that_is_not_a_mod_kind_is_refused_before_the_daemon() {
    let (status, seen) = post_message(&format!(r#"{{"session_id":"{SESSION}","text":"hello","aim":"sparkle"}}"#)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "an unknown aim is undecodable");
    assert!(seen.is_empty(), "nothing reached the daemon");
}
