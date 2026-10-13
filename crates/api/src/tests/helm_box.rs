//! Helm's message box over the routes: what a tune answers and offers, a file read back under the
//! type it was sent as, an `opened` that carries the box's facts, and one issue read by number.
//! What each is, is Fleet's (`fleet::tests::helm_box`); this holds the routes and the wire.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use futures_util::StreamExt;
use http_body_util::BodyExt;
use ipc::{HelmBoxFacts, HelmMessage, IssueRead, IssueState};
use tower::ServiceExt;

use crate::tests::connected;
use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, HelmSeen, Served};

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
async fn a_tune_answers_the_facts_and_offers_them_to_whoever_watches() {
    let (daemon, app) = wired();
    let mut watching = daemon.helm.watch();
    let (status, _, body) = call(&app, "POST", "/helm/tune", r#"{"model":"opus","effort":"high"}"#).await;
    assert_eq!(status, StatusCode::OK);
    let facts: HelmBoxFacts = ipc::decode("facts", &body).expect("facts");
    assert_eq!(facts.model.as_deref(), Some("opus"));
    assert_eq!(facts.effort.as_deref(), Some("high"));
    let Some(HelmSeen::Message(HelmMessage::Facts(offered))) = watching.next().await else {
        panic!("the watcher is sent facts");
    };
    assert_eq!(offered, facts);

    let (status, _, _) = call(&app, "POST", "/helm/tune?manifest_id=elsewhere", "{}").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _, _) = call(&app, "POST", "/helm/tune", "not json").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_file_is_served_as_the_type_it_was_sent_and_an_unknown_one_is_refused() {
    let (_daemon, app) = wired();
    let (status, kind, body) = call(&app, "GET", "/helm/file?file=f1", "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(kind.as_deref(), Some("image/png"));
    assert_eq!(body, [0x89, b'P', b'N', b'G']);

    let (status, _, _) = call(&app, "GET", "/helm/file?file=f9", "").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _, _) = call(&app, "GET", "/helm/file", "").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "a file is named");
}

#[tokio::test]
async fn one_issue_is_read_by_number_open_or_closed_and_a_missing_one_is_a_404() {
    let (_daemon, app) = wired();
    let (status, _, body) = call(&app, "GET", "/issues/armada/214", "").await;
    assert_eq!(status, StatusCode::OK);
    let read: IssueRead = ipc::decode("an issue", &body).expect("an issue");
    assert_eq!((read.number, read.state), (214, IssueState::Open));

    let (_, _, body) = call(&app, "GET", "/issues/armada/215", "").await;
    let read: IssueRead = ipc::decode("an issue", &body).expect("an issue");
    assert_eq!(read.state, IssueState::Closed);

    let (status, _, body) = call(&app, "GET", "/issues/armada/99", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(String::from_utf8_lossy(&body).contains("fleet.no_such_issue"));
}

#[tokio::test]
async fn opened_carries_the_facts_the_box_runs_on() {
    let (daemon, app) = wired();
    *daemon.helm_facts.lock().expect("not poisoned") = HelmBoxFacts {
        model: Some("opus".to_string()),
        effort: None,
        commands: vec!["/help".to_string()],
    };
    let mut socket = connected(app, "/helm/observe", 8192).await;
    let frame = tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .expect("the socket answers")
        .expect("the socket is open")
        .expect("a frame");
    let tokio_tungstenite::tungstenite::Message::Text(json) = frame else {
        panic!("the socket is text");
    };
    assert!(json.contains(r#""facts":{"model":"opus","commands":["/help"]}"#), "{json}");
    let HelmMessage::Opened(opened) = ipc::decode("a Helm message", json.as_bytes()).expect("decodes")
    else {
        panic!("the first message is opened");
    };
    assert_eq!(opened.facts.commands, ["/help"]);
}
