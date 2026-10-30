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
const THREAD: &str = r#"{"session":{"id":"h1","harness":"term","origin":"bridge","cwd":"/Users/user/armada","title":"Fix it","state":"live","started_at":"2026-10-08T01:00:00.000Z","last_seen_at":"2026-10-08T01:00:00.000Z","usage":{},"attachments":[]},"rows":[{"kind":"ask","id":"c1","question":"Which branch?"}]}"#;
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
            "/sessions/shown",
            get(|RawQuery(query): RawQuery| async move {
                match query.as_deref() {
                    Some("session_id=h1&path=%2Fr%2Fshot.png") => (StatusCode::OK, [("content-type", "image/png")], vec![0x89, b'P']),
                    _ => (StatusCode::NOT_FOUND, [("content-type", "application/json")], br#"{"code":"fleet.shown_file_not_shown"}"#.to_vec()),
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

#[tokio::test]
async fn a_picture_the_job_showed_reaches_the_phone_as_its_bytes() {
    let phone = phone_on(stub_fleet(Seen::default()).await).await;
    let time = phone.rig.now.fetch_add(1, Ordering::Relaxed) + 1;
    let path = "/api/sessions/shown?session_id=h1&path=%2Fr%2Fshot.png";
    let request = signed(&phone.device, "GET", path, time, "")
        .into_iter()
        .fold(axum::http::Request::get(path), |request, (name, value)| request.header(name, value));
    let answer = tower::ServiceExt::oneshot(phone.rig.app.clone(), request.body(axum::body::Body::empty()).unwrap()).await.unwrap();
    assert_eq!(answer.status(), StatusCode::OK);
    assert_eq!(answer.headers()["content-type"], "image/png");
    assert_eq!(axum::body::to_bytes(answer.into_body(), 64).await.unwrap().as_ref(), [0x89, b'P']);
    let (status, _) = send(&phone, Method::GET, "/api/sessions/shown?session_id=h1&path=%2Fetc%2Fhosts", "").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Every method, path-and-query and body the Fleet below was sent.
type Heard = Arc<Mutex<Vec<(String, String, String)>>>;

async fn recording_fleet(heard: Heard) -> u16 {
    let app = Router::new().fallback(move |method: Method, uri: axum::http::Uri, body: String| async move {
        let target = uri.path_and_query().map_or(uri.path().to_string(), |p| p.as_str().to_string());
        heard.lock().unwrap().push((method.to_string(), target, body));
        (StatusCode::OK, r#"{"from":"fleet"}"#)
    });
    let listener = crate::bind(0).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, app).await });
    port
}

#[tokio::test]
async fn the_routes_the_job_screens_use_are_signed_and_reach_fleet_without_api() {
    let heard = Heard::default();
    let phone = phone_on(recording_fleet(heard.clone()).await).await;
    let body = r#"{"session_id":"h1"}"#;
    let posts = [
        "/sessions/begin",
        "/sessions/stop",
        "/sessions/pause",
        "/sessions/resume",
        "/sessions/close",
        "/sessions/tune",
        "/sessions/run/start",
        "/sessions/waiting/dismiss",
    ];
    for path in posts {
        let (status, said) = send(&phone, Method::POST, &format!("/api{path}"), body).await;
        assert_eq!((status, said.as_str()), (StatusCode::OK, r#"{"from":"fleet"}"#), "{path}");
        // A signature on another path, or none, opens nothing.
        assert_eq!(call(&phone.rig.app, Method::POST, &format!("/api{path}"), &[], body).await.0, StatusCode::BAD_REQUEST, "{path}");
    }
    for path in ["/plan_usage", "/models"] {
        let (status, said) = send(&phone, Method::GET, &format!("/api{path}"), "").await;
        assert_eq!((status, said.as_str()), (StatusCode::OK, r#"{"from":"fleet"}"#), "{path}");
        assert_eq!(call(&phone.rig.app, Method::GET, &format!("/api{path}"), &[], "").await.0, StatusCode::BAD_REQUEST, "{path}");
    }
    let mut want: Vec<(String, String, String)> =
        posts.iter().map(|path| ("POST".to_string(), path.to_string(), body.to_string())).collect();
    want.extend(["/plan_usage", "/models"].map(|path| ("GET".to_string(), path.to_string(), String::new())));
    assert_eq!(*heard.lock().unwrap(), want, "each reached Fleet once, on its own path, with its body as signed");
}
