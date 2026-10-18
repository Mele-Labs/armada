//! `get_flight_controls`, `save_flight_controls` and `run_session_shell` over the router: the
//! routes exist, a body reaches the daemon whole, and one with a key nobody knows is a 400.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{FlightControlsRead, FlightControls};
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
async fn the_controls_are_read_and_a_save_reaches_the_daemon_whole() {
    let app = wired();
    let (status, body) = call(&app, "GET", "/kit/flight_controls", "").await;
    assert_eq!(status, StatusCode::OK);
    let read: FlightControlsRead = ipc::decode("flight controls", &body).expect("decodes");
    assert_eq!(read, FlightControlsRead::Read { controls: FlightControls::default() });

    let sent = r#"{"version":1,"job":[{"id":"ask","name":"Update","key":"1","hidden":true},
        {"id":"pull","kind":"shell","text":"git pull"}],"deck":[{"id":"dispatch"}]}"#;
    let (status, body) = call(&app, "POST", "/kit/flight_controls", sent).await;
    assert_eq!(status, StatusCode::OK);
    let FlightControlsRead::Read { controls } = ipc::decode("saved", &body).expect("decodes") else {
        panic!("the fake answers what it was sent");
    };
    assert_eq!(controls.job[0].name.as_deref(), Some("Update"));
    assert!(controls.job[0].hidden);
    assert_eq!(controls.job[1].kind, Some(ipc::FlightControlKind::Shell));
    assert_eq!(controls.deck[0].id, "dispatch");
}

#[tokio::test]
async fn a_key_nobody_knows_is_a_400() {
    let app = wired();
    let (status, _) = call(&app, "POST", "/kit/flight_controls", r#"{"version":1,"job":[{"id":"a","hiden":true}]}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_shell_action_is_routed_to_the_daemon() {
    let app = wired();
    let (status, body) = call(&app, "POST", "/sessions/shell", r#"{"session_id":"s1","command":"true"}"#).await;
    // The fake runs nothing and says so, which only a routed request can.
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{}", String::from_utf8_lossy(&body));
    let (status, _) = call(&app, "POST", "/sessions/shell", r#"{"command":"true"}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
