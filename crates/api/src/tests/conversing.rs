//! Helm's routes: a message answered with the Helm Session it went to, the Session found by its
//! repository, a fresh start ending it, a message with nothing in it refused as the daemon refuses
//! it, a repository nobody serves refused, and the record served whole. What the Session's thread
//! is made of is Fleet's (`fleet::tests::helm_session`); this holds the routes and the wire.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use ipc::{HelmSession, SessionRecord};
use tower::ServiceExt;

use crate::tests::fake::{FakeDaemon, SERVED_MANIFEST};
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
async fn a_message_is_answered_with_the_helm_session_and_the_session_is_then_found() {
    let (daemon, app) = wired();
    let (status, body) = call(&app, "GET", "/helm/session", "").await;
    assert_eq!(status, StatusCode::OK);
    let before: HelmSession = ipc::decode("a Helm session", &body).expect("an answer");
    assert_eq!(before.manifest_id.as_str(), SERVED_MANIFEST);
    assert!(before.session.is_none(), "none before the first message");
    assert!(!String::from_utf8_lossy(&body).contains("session\""), "left out, never null: {body:?}");

    let (status, body) = call(&app, "POST", "/helm/ask", r#"{"text":"why did it stall?"}"#).await;
    assert_eq!(status, StatusCode::OK);
    let record: SessionRecord = ipc::decode("a record", &body).expect("the Session comes back");
    assert!(record.helm, "it is marked as Helm");
    assert_eq!(daemon.helm_asked.lock().expect("not poisoned").len(), 1);

    let (_, body) = call(&app, "GET", "/helm/session?manifest_id=armada", "").await;
    let after: HelmSession = ipc::decode("a Helm session", &body).expect("an answer");
    assert_eq!(after.session.expect("one now").session.id, record.id);
}

/// A blank text decodes, so refusing a message with nothing in it is the daemon's: a 422, and a
/// message that carries a file is taken.
#[tokio::test]
async fn a_message_with_nothing_in_it_is_the_daemons_to_refuse() {
    let (_daemon, app) = wired();
    let (status, body) = call(&app, "POST", "/helm/ask", r#"{"text":"   "}"#).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(String::from_utf8_lossy(&body).contains("fleet.session_message_empty"));

    let (status, _) = call(
        &app,
        "POST",
        "/helm/ask",
        r#"{"text":"","attachments":[{"name":"a.txt","media_type":"text/plain","data":"aGk="}]}"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn starting_fresh_answers_with_no_session() {
    let (_daemon, app) = wired();
    call(&app, "POST", "/helm/ask", r#"{"text":"hello"}"#).await;

    let (status, body) = call(&app, "POST", "/helm/start_fresh", "").await;
    assert_eq!(status, StatusCode::OK);
    let fresh: HelmSession = ipc::decode("a Helm session", &body).expect("an answer");
    assert!(fresh.session.is_none());

    let (_, body) = call(&app, "GET", "/helm/session", "").await;
    let after: HelmSession = ipc::decode("a Helm session", &body).expect("an answer");
    assert!(after.session.is_none(), "the next message starts a new one");
}

#[tokio::test]
async fn a_repository_nobody_serves_is_refused_by_every_route() {
    let (_daemon, app) = wired();
    for (method, uri) in [
        ("GET", "/helm/session?manifest_id=elsewhere"),
        ("POST", "/helm/start_fresh?manifest_id=elsewhere"),
        ("GET", "/helm/debug?manifest_id=elsewhere"),
    ] {
        let (status, _) = call(&app, method, uri, "").await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{method} {uri}");
    }
    let (status, _) = call(&app, "POST", "/helm/ask?manifest_id=elsewhere", r#"{"text":"hi"}"#).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn the_record_is_served_whole() {
    let (_daemon, app) = wired();
    let (status, body) = call(&app, "GET", "/helm/debug?manifest_id=armada", "").await;
    assert_eq!(status, StatusCode::OK);
    let record: ipc::HelmDebugInfo = ipc::decode("a record", &body).expect("the record comes back");
    assert_eq!(record.manifest_id.as_str(), SERVED_MANIFEST);
    assert_eq!(record.protocol_id, ipc::ProtocolId::current());
    assert!(!record.note.is_empty(), "the note as it is appended");
}
