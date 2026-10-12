//! `get_settings` and `save_settings` over the router: a save round-trips, a
//! `null` puts the default back, a key the daemon refuses is a 422 by name, and
//! a body that is not a set of changes is a 400 the daemon never sees.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{SettingValue, SettingsList, WireError};
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
async fn a_save_round_trips_and_null_puts_the_default_back() {
    let app = wired();
    let (status, body) = call(&app, "GET", "/settings", "").await;
    assert_eq!(status, StatusCode::OK);
    let read: SettingsList = ipc::decode("settings", &body).expect("decodes");
    assert_eq!(read.settings[0].saved, None);

    let (status, body) = call(&app, "POST", "/settings/save", r#"{"changes":{"limits.dronesAtOnce":3}}"#).await;
    assert_eq!(status, StatusCode::OK);
    let saved: SettingsList = ipc::decode("settings", &body).expect("decodes");
    assert_eq!(saved.settings[0].saved, Some(SettingValue::Integer(3)));
    assert_eq!(saved.settings[0].value, SettingValue::Integer(3));

    let (_, body) = call(&app, "POST", "/settings/save", r#"{"changes":{"limits.dronesAtOnce":null}}"#).await;
    let reset: SettingsList = ipc::decode("settings", &body).expect("decodes");
    assert_eq!(reset.settings[0].saved, None);
    assert_eq!(reset.settings[0].value, SettingValue::Integer(2));
}

#[tokio::test]
async fn a_key_the_daemon_refuses_is_a_422_and_a_body_that_is_not_changes_a_400() {
    let app = wired();
    let (status, body) = call(&app, "POST", "/settings/save", r#"{"changes":{"limits.nothing":1}}"#).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let refused: WireError = ipc::decode("a refusal", &body).expect("decodes");
    assert_eq!(refused.code, "fleet.unacceptable_settings");

    let (status, _) = call(&app, "POST", "/settings/save", r#"{"limits.dronesAtOnce":3}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
