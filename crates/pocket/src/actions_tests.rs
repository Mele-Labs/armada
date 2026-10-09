//! The act routes against a fake Fleet that records what it is sent.

use std::sync::{Arc, Mutex};

use axum::http::{HeaderMap, Method, StatusCode, Uri};
use axum::Router;
use std::sync::atomic::Ordering;

use crate::pairing_tests::{call, paired, rig_with, signed, Rig};

/// (method, path, had Origin, body)
type Seen = Arc<Mutex<Vec<(String, String, bool, String)>>>;

async fn fleet_answering(status: StatusCode, body: &'static str) -> (u16, Seen) {
    let seen: Seen = Arc::default();
    let log = seen.clone();
    let app = Router::new().fallback(move |method: Method, uri: Uri, headers: HeaderMap, text: String| {
        let log = log.clone();
        async move {
            log.lock().unwrap().push((
                method.to_string(),
                uri.path().to_string(),
                headers.contains_key("origin"),
                text,
            ));
            (status, body)
        }
    });
    let listener = crate::bind(0).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, app).await });
    (port, seen)
}

struct App {
    rig: Rig,
    device: String,
}

async fn on(fleet: crate::Fleet) -> App {
    let rig = rig_with(fleet);
    let device = paired(&rig).await;
    App { rig, device }
}

async fn post(app: &App, path: &str, body: &str) -> (StatusCode, String) {
    let time = app.rig.now.fetch_add(1, Ordering::Relaxed) + 1;
    let headers = signed(&app.device, "POST", path, time, body);
    call(&app.rig.app, Method::POST, path, &headers, body).await
}

const ROUTES: [(&str, &str, &str, &str); 7] = [
    ("approve", "approve_dispatch", "{}", ""),
    ("redirect", "redirect", r#"{"text":"use the other table"}"#, r#"{"instruction":"use the other table"}"#),
    ("restart_step", "restart_step", "{}", ""),
    ("kill", "kill_job", "{}", ""),
    ("redispatch", "redispatch", "{}", ""),
    ("approve_review", "approve_review", "{}", ""),
    ("request_changes", "request_changes", r#"{"reason":"add a test"}"#, r#"{"note":"add a test"}"#),
];

#[tokio::test]
async fn each_act_reaches_its_one_fleet_route_with_a_body_built_here() {
    let (port, seen) = fleet_answering(StatusCode::OK, "{}").await;
    let app = on(Arc::new(move || Ok(port))).await;
    for (gateway, fleet, sent, expected) in ROUTES {
        let (status, body) = post(&app, &format!("/api/jobs/j-1/{gateway}"), sent).await;
        assert_eq!(status, StatusCode::NO_CONTENT, "{gateway}: {body}");
        let log = seen.lock().unwrap();
        let last = log.last().unwrap();
        assert_eq!(last.0, "POST");
        assert_eq!(last.1, format!("/jobs/j-1/{fleet}"));
        assert_eq!(last.3, expected, "{gateway}");
        assert!(!last.2, "{gateway} sent an Origin");
    }
    assert_eq!(seen.lock().unwrap().len(), 7);
}

#[tokio::test]
async fn a_field_the_phone_may_not_set_is_refused_and_never_forwarded() {
    let (port, seen) = fleet_answering(StatusCode::OK, "{}").await;
    let app = on(Arc::new(move || Ok(port))).await;
    for (path, body) in [
        ("approve", r#"{"title":"mine"}"#),
        ("kill", r#"{"by":"helm"}"#),
        ("redirect", r#"{"text":"x","instruction":"y"}"#),
        ("redirect", "{}"),
        ("request_changes", r#"{"reason":"x","with_walk_notes":true}"#),
    ] {
        let (status, _) = post(&app, &format!("/api/jobs/j1/{path}"), body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{path} {body}");
    }
    assert!(seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn unsigned_is_401_and_a_bad_id_is_404_before_fleet_hears_of_it() {
    let (port, seen) = fleet_answering(StatusCode::OK, "{}").await;
    let app = on(Arc::new(move || Ok(port))).await;
    for (gateway, ..) in ROUTES {
        let path = format!("/api/jobs/j1/{gateway}");
        let (status, _) = call(&app.rig.app, Method::POST, &path, &[], "{}").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{gateway}");
        let bad = format!("/api/jobs/..%2Fadmin/{gateway}");
        assert_eq!(post(&app, &bad, "{}").await.0, StatusCode::NOT_FOUND, "{gateway}");
    }
    assert!(seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_fleet_refusal_passes_through_with_its_status_and_sentence() {
    let refusal = r#"{"code":"x","message":"That Job is not waiting for review.","run_id":"r","fields":{},"chain":[]}"#;
    let (port, _) = fleet_answering(StatusCode::CONFLICT, refusal).await;
    let app = on(Arc::new(move || Ok(port))).await;
    let (status, body) = post(&app, "/api/jobs/j1/approve_review", "{}").await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body, "That Job is not waiting for review.");
}

#[tokio::test]
async fn fleet_being_down_says_what_to_do() {
    let off = on(Arc::new(|| Err("no runtime file at /secret".to_string()))).await;
    let (status, body) = post(&off, "/api/jobs/j1/kill", "{}").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body, "Armada is not running on your Mac. Open it there, then try again.");
    let closed = {
        let listener = crate::bind(0).await.unwrap();
        listener.local_addr().unwrap().port()
    };
    let closed = on(Arc::new(move || Ok(closed))).await;
    let (status, body) = post(&closed, "/api/jobs/j1/kill", "{}").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body, "Armada is not answering on your Mac. Open it there, then try again.");
}
