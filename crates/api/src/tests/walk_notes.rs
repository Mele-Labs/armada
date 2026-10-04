//! A Job's walk notes over the router: both routes are served, the body
//! reaches the daemon whole, and the answer is every note on the Job. What
//! Fleet keeps, and what it refuses, is `fleet`'s to test.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::tests::fake::{self, FakeDaemon};
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served};

fn wired() -> Router {
    let daemon = FakeDaemon::new(Broadcaster::new());
    fake::at(&daemon, "01WALKED", "awaiting_review");
    router(Served::sharing(
        Arc::new(daemon),
        run_id(),
        Broadcaster::new(),
    ))
}

async fn call(app: &Router, uri: &str, body: &str) -> (StatusCode, Vec<u8>) {
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    let response = app.clone().oneshot(request).await.expect("an answer");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("a body")
        .to_bytes();
    (status, bytes.to_vec())
}

#[tokio::test]
async fn a_walk_note_is_captured_and_answered_with_every_note() {
    let app = wired();
    let body = r#"{"said":"this button is too small","capture":{"selector":"button.save",
        "element":{"tag":"button","text":"Save"},"location":"/settings",
        "bounds":{"x":1,"y":2,"width":30,"height":12},"window":{"width":1440,"height":900},
        "markup":"<button>Save</button>","served":{"run":"01RUN","name":"mock",
        "address":"http://127.0.0.1:5173"}},
        "frame":{"staged_path":"/tmp/f.png","width":60,"height":24}}"#;
    let (status, answer) = call(&app, "/jobs/01WALKED/walk_notes", body).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&answer)
    );
    let notes: ipc::WalkNotes = ipc::decode("walk notes", &answer).expect("decodes");
    let note = &notes.notes[0];
    assert_eq!(note.said, "this button is too small");
    assert_eq!(note.selector, "button.save");
    assert_eq!(note.location, "/settings");
    assert_eq!(note.served.as_ref().map(|s| s.name.as_str()), Some("mock"));
}

#[tokio::test]
async fn a_walk_note_is_removed_by_id_and_a_job_that_is_not_there_is_a_404() {
    let app = wired();
    let (status, answer) = call(
        &app,
        "/jobs/01WALKED/walk_notes/remove",
        r#"{"id":"01WALKNOTE"}"#,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&answer)
    );
    let (status, _) = call(&app, "/jobs/01NOSUCH/walk_notes/remove", r#"{"id":"x"}"#).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = call(&app, "/jobs/01WALKED/walk_notes", r#"{"said":"x"}"#).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "a body with no capture does not decode"
    );
}
