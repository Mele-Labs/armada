//! The routes that are the person's alone, called from inside a Job around the door: refused in
//! one sentence and nothing written, while the same call from outside any Job is answered.

use std::net::SocketAddr;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ipc::{SettingsList, WireError};
use tower::ServiceExt;

use crate::callers::{the_persons, A_JOBS_CALL, THE_PERSONS_OWN};
use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served};

/// The port a planted Job calls from, and one no Job holds, as Bridge's is.
const JOB: u16 = 53000;
const BRIDGE: u16 = 53001;
const GATEWAY: u16 = 53002;

const TURNING_THE_OPERATOR_ON: &str = r#"{"changes":{"limits.dronesAtOnce":3}}"#;

fn wired() -> Router {
    let events = Broadcaster::new();
    let daemon = FakeDaemon::new(events.clone());
    *daemon.session_on.lock().expect("not poisoned") = Some(JOB);
    router(Served::by(daemon, run_id(), events))
}

async fn call(app: &Router, method: &str, uri: &str, port: u16, body: &str) -> (StatusCode, Vec<u8>) {
    let peer: SocketAddr = format!("127.0.0.1:{port}").parse().expect("an address");
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        // What Job `8fd49e7c`'s call could have said, and what decides nothing.
        .header(crate::acting::CALLER_HEADER, crate::acting::BRIDGE)
        .extension(axum::extract::ConnectInfo(peer))
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    let response = app.clone().oneshot(request).await.expect("an answer");
    let status = response.status();
    let bytes = response.into_body().collect().await.expect("a body").to_bytes();
    (status, bytes.to_vec())
}

#[tokio::test]
async fn a_jobs_settings_save_is_refused_and_nothing_is_written() {
    let app = wired();
    let (status, body) = call(&app, "POST", "/settings/save", JOB, TURNING_THE_OPERATOR_ON).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let refused: WireError = ipc::decode("a refusal", &body).expect("decodes");
    assert_eq!(refused.code, A_JOBS_CALL);
    assert_eq!(
        refused.message,
        "`save_settings` is refused to a Job: only the person changes settings, from Bridge"
    );

    let (_, body) = call(&app, "GET", "/settings", BRIDGE, "").await;
    let read: SettingsList = ipc::decode("settings", &body).expect("decodes");
    assert_eq!(read.settings[0].saved, None, "the refused save wrote nothing");
}

#[tokio::test]
async fn the_same_save_from_outside_any_job_is_answered() {
    let app = wired();
    let (status, _) = call(&app, "POST", "/settings/save", BRIDGE, TURNING_THE_OPERATOR_ON).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn a_job_still_reads_the_settings() {
    let app = wired();
    let (status, _) = call(&app, "GET", "/settings", JOB, "").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn a_job_cannot_change_the_slot_pool() {
    let app = wired();
    let (status, body) = call(&app, "POST", "/worktrees/slots", JOB, r#"{"size":2}"#).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let refused: WireError = ipc::decode("a refusal", &body).expect("decodes");
    assert!(refused.message.contains("only the person changes the slot pool"), "{}", refused.message);
}

/// The routes another Mac's rows and claims come in on, each with a body the fake takes.
fn handing_in_another_macs_rows() -> Vec<(&'static str, &'static str, String)> {
    let event = ipc::OtherMacEvent {
        mac: "mac-work".into(),
        event: ipc::Event::ProjectsChanged(ipc::Projects { projects: Vec::new(), loose: Vec::new(), left_out: Vec::new() }),
    };
    vec![
        ("PUT", "/other_macs/sessions", r#"{"mac":"mac-work","sessions":[]}"#.to_string()),
        ("PUT", "/other_macs/bug_reports", r#"{"mac":"mac-work","reports":[]}"#.to_string()),
        ("POST", "/other_macs/event", ipc::encode(&event).expect("encodes")),
        ("POST", "/other_macs/forget", r#"{"mac":"mac-work"}"#.to_string()),
        ("PUT", "/other_macs/claims", r#"{"claims":[]}"#.to_string()),
    ]
}

#[tokio::test]
async fn a_job_cannot_hand_fleet_another_macs_rows_and_the_gateway_can() {
    let app = wired();
    for (method, uri, body) in handing_in_another_macs_rows() {
        let (status, refused) = call(&app, method, uri, JOB, &body).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {uri} from a Job");
        let refused: WireError = ipc::decode("a refusal", &refused).expect("decodes");
        assert_eq!(refused.code, A_JOBS_CALL);
        assert!(
            refused.message.ends_with("is refused to a Job: only the person pairs his other Macs, from Bridge"),
            "{}",
            refused.message
        );

        // The Gateway runs in Fleet's own process, which no Job holds, and says it is the phone.
        let peer: SocketAddr = format!("127.0.0.1:{GATEWAY}").parse().expect("an address");
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .header(crate::acting::CALLER_HEADER, crate::acting::PHONE)
            .extension(axum::extract::ConnectInfo(peer))
            .body(Body::from(body))
            .expect("a well-formed request");
        let answered = app.clone().oneshot(request).await.expect("an answer");
        assert_eq!(answered.status(), StatusCode::OK, "{method} {uri} from the Gateway");
    }
}

/// A name the table holds that no route serves would refuse nothing, silently.
#[test]
fn every_act_the_person_keeps_is_a_route_fleet_serves() {
    for (operation, _) in THE_PERSONS_OWN {
        let route = crate::SERVED
            .iter()
            .find(|route| route.operation == *operation)
            .unwrap_or_else(|| panic!("`{operation}` is not served"));
        assert_ne!(route.method, "GET", "`{operation}` is a write");
        assert_eq!(the_persons(route.method, route.path).map(|(name, _)| name), Some(*operation));
    }
}
