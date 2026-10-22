//! Pocket v2's claim: a paired phone reads a Job's whole thread and replies to it,
//! each call passed to Fleet as it was sent and Fleet's answer passed back as it came.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use axum::extract::RawQuery;
use axum::http::{HeaderMap, Method, StatusCode};
use axum::routing::{get, post};
use axum::Router;

use crate::pairing_tests::{call, paired, rig_with, signed, Rig};

/// A path, the caller header Fleet was sent, and the body or query.
type Seen = Arc<Mutex<Vec<(String, String, String)>>>;

/// A thread as Fleet answers it, with a field the phone list trims away, so a
/// pass-through that trimmed would show.
const THREAD: &str = r#"{"session":{"id":"h1","harness":"term","origin":"bridge","cwd":"/Users/owner/armada","title":"Fix it","state":"live","started_at":"2026-10-08T01:00:00.000Z","last_seen_at":"2026-10-08T01:00:00.000Z","usage":{},"attachments":[]},"rows":[{"kind":"ask","id":"c1","question":"Which branch?"}]}"#;
const ANSWER: &str = r#"{"session_id":"h1","call":"c1","answer":"allow_once","answers":[{"question":"Which branch?","chosen":["main"]}]}"#;

fn caller(headers: &HeaderMap) -> String {
    headers.get("x-armada-caller").and_then(|v| v.to_str().ok()).unwrap_or("").to_string()
}

pub(crate) async fn stub_fleet(seen: Seen) -> u16 {
    let read = seen.clone();
    let wrote = seen;
    let app = Router::new()
        .route(
            "/sessions/one",
            get(move |headers: HeaderMap, RawQuery(query): RawQuery| async move {
                let query = query.unwrap_or_default();
                read.lock().unwrap().push(("/sessions/one".into(), caller(&headers), query.clone()));
                match query.as_str() {
                    "session_id=h1" => (StatusCode::OK, THREAD.to_string()),
                    _ => (StatusCode::NOT_FOUND, r#"{"code":"fleet.no_session","message":"No such job."}"#.to_string()),
                }
            }),
        )
        .route(
            "/sessions/ask/answer",
            post(move |headers: HeaderMap, body: String| async move {
                wrote.lock().unwrap().push(("/sessions/ask/answer".into(), caller(&headers), body));
                (StatusCode::OK, THREAD.to_string())
            }),
        );
    let listener = crate::bind(0).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, app).await });
    port
}

pub(crate) struct Phone {
    pub rig: Rig,
    pub device: String,
}

pub(crate) async fn phone_on(port: u16) -> Phone {
    let rig = rig_with(Arc::new(move || Ok(port)));
    let device = paired(&rig).await;
    Phone { rig, device }
}

pub(crate) async fn send(phone: &Phone, method: Method, path: &str, body: &str) -> (StatusCode, String) {
    let time = phone.rig.now.fetch_add(1, Ordering::Relaxed) + 1;
    let headers = signed(&phone.device, method.as_str(), path, time, body);
    call(&phone.rig.app, method, path, &headers, body).await
}

#[tokio::test]
async fn a_paired_phone_reads_a_jobs_thread_and_answers_its_question() {
    let seen = Seen::default();
    let phone = phone_on(stub_fleet(seen.clone()).await).await;

    let (status, body) = send(&phone, Method::GET, "/api/sessions/one?session_id=h1", "").await;
    assert_eq!((status, body.as_str()), (StatusCode::OK, THREAD), "the thread arrives as Fleet sent it");

    let (status, body) = send(&phone, Method::POST, "/api/sessions/ask/answer", ANSWER).await;
    assert_eq!((status, body.as_str()), (StatusCode::OK, THREAD), "Fleet's answer comes back as it is");

    let (status, body) = send(&phone, Method::GET, "/api/sessions/one?session_id=nope", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");

    let seen = seen.lock().unwrap();
    assert_eq!(seen[0], ("/sessions/one".into(), "phone".into(), "session_id=h1".into()));
    assert_eq!(seen[1], ("/sessions/ask/answer".into(), "phone".into(), ANSWER.into()));
}

#[tokio::test]
async fn the_phone_reads_this_macs_name_off_its_tailnet_address() {
    let phone = phone_on(stub_fleet(Seen::default()).await).await;
    assert_eq!(send(&phone, Method::GET, "/api/machine", "").await, (StatusCode::OK, r#"{"name":"mac"}"#.to_string()));
    assert_eq!(crate::machine::short_name("https://mac-b.tail1234.ts.net").as_deref(), Some("mac-b"));
    assert_eq!(crate::machine::short_name("no tailscale"), None);
}
