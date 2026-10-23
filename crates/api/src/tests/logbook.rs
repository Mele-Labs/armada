//! The Logbook's five routes over the router: each is served at its path, and a section, a state
//! or a body the wire does not hold is a 400 the daemon never sees. `docs/concepts/logbook.md`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{Logbook, LogbookEntry, ForgottenLogbookEntry};
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
async fn each_route_is_served_and_answers_its_wire_type() {
    let app = wired();

    let (status, body) = call(&app, "GET", "/logbook?section=habits&state=faded", "").await;
    assert_eq!(status, StatusCode::OK);
    assert!(ipc::decode::<Logbook>("the logbook", &body).expect("decodes").entries.is_empty());

    let teach = r#"{"section":"habits","kind":"habit","text":"Walk before landing.","paths":["docs"]}"#;
    let (status, body) = call(&app, "POST", "/logbook/entries", teach).await;
    assert_eq!(status, StatusCode::OK);
    let taught: LogbookEntry = ipc::decode("an entry", &body).expect("decodes");
    assert_eq!((taught.text.as_str(), taught.paths.as_slice()), ("Walk before landing.", ["docs".to_string()].as_slice()));

    let (status, body) = call(&app, "POST", "/logbook/entries/7/edit", r#"{"text":"Walk, then land."}"#).await;
    assert_eq!(status, StatusCode::OK);
    let edited: LogbookEntry = ipc::decode("an entry", &body).expect("decodes");
    assert_eq!((edited.id.as_str(), edited.text.as_str(), edited.edited), ("7", "Walk, then land.", true));

    let (status, body) = call(&app, "POST", "/logbook/entries/7/pin", r#"{"pinned":true}"#).await;
    assert_eq!(status, StatusCode::OK);
    assert!(ipc::decode::<LogbookEntry>("an entry", &body).expect("decodes").pinned);

    let (status, body) = call(&app, "POST", "/logbook/entries/7/forget", "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ipc::decode::<ForgottenLogbookEntry>("a forgotten entry", &body).expect("decodes").id, "7");
}

#[tokio::test]
async fn a_bad_section_state_or_body_is_a_400() {
    let app = wired();
    let bad = r#"{"section":"everywhere","kind":"habit","text":"Text."}"#;
    assert_eq!(call(&app, "POST", "/logbook/entries", bad).await.0, StatusCode::BAD_REQUEST);
    let kind = r#"{"section":"habits","kind":"rumour","text":"Text."}"#;
    assert_eq!(call(&app, "POST", "/logbook/entries", kind).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(call(&app, "GET", "/logbook?section=everywhere", "").await.0, StatusCode::BAD_REQUEST);
    assert_eq!(call(&app, "GET", "/logbook?state=gone", "").await.0, StatusCode::BAD_REQUEST);
    assert_eq!(call(&app, "POST", "/logbook/entries/7/edit", "{}").await.0, StatusCode::BAD_REQUEST);
    assert_eq!(call(&app, "POST", "/logbook/entries/7/pin", "").await.0, StatusCode::BAD_REQUEST);
}
