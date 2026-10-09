//! The read routes against a fake Fleet: a local axum server that answers the
//! routes the Gateway reads, with a marker in every field the phone must not see.

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use std::sync::atomic::Ordering;

use axum::http::Method;

use crate::pairing_tests::{call, paired, rig_with, signed, Rig};

const SECRET: &str = "SECRET-BRIEF-9f3";

fn summary(id: &str, status: &str, extra: &str) -> String {
    format!(
        r#"{{"id":"{id}","handle":"h-{id}","title":"Title {id}","status":"{status}","created_at":"2026-10-08T0{}:00:00.000Z","workflow_id":"bug","owner_manifest_id":"m1","origin":"manual","urgency":"normal","atomic":false,"model":"m","facts":"{SECRET}"{extra}}}"#,
        id.len() % 9
    )
}

fn detail(id: &str, status: &str, extra: &str) -> String {
    format!(
        r#"{{"job":{job},"created_at":"2026-10-08T01:00:00.000Z","steps":[
          {{"step_id":"s1","label":"Plan","ordinal":0,"state":"advanced","check_runs":[],"overridden":false,"judged":[],"flagged":[],"entered_at":"2026-10-08T01:00:00.000Z","updated_at":"2026-10-08T01:00:00.000Z"}},
          {{"step_id":"s2","label":"Implement","ordinal":1,"state":"running","check_runs":[
            {{"attempt":1,"name":"typecheck","outcome":"failed","output_path":"/logs/{SECRET}"}},
            {{"attempt":2,"name":"typecheck","outcome":"passed","output_path":"/logs/{SECRET}"}},
            {{"attempt":2,"name":"store_test","outcome":"failed","produced":"{SECRET}"}}],
           "overridden":false,"judged":[{{"attempt":2,"criterion_id":"c1","verdict":"not_met","consequence":"{SECRET}"}}],"flagged":[],"entered_at":"2026-10-08T01:00:00.000Z","updated_at":"2026-10-08T01:00:00.000Z"}}],
         "acceptance_criteria":[],"facts":"{SECRET}","dependencies":[],
         "delivery":{{"commit":"{SECRET}","pull_request":"https://example.test/pull/7"}}}}"#,
        job = summary(id, status, extra)
    )
}

async fn events(upgrade: WebSocketUpgrade) -> impl IntoResponse {
    upgrade.on_upgrade(|mut socket| async move {
        let changed = r#"{"message":"event","cursor":2,"event":{"kind":"job.state_changed","job_id":"j2","from":"running","to":"escalated","reason":{"named":"stalled"},"actor":"fleet","at":"2026-10-08T02:00:00.000Z"}}"#;
        let files = r#"{"message":"event","cursor":3,"event":{"kind":"job.files_changed","job_id":"j2","step_id":"s2","drone_id":"d1","plan_declared":true,"files":[],"actor":"drone","at":"2026-10-08T02:00:01.000Z"}}"#;
        for text in [files, changed] {
            socket.send(Message::Text(text.to_string())).await.ok();
        }
    })
}

/// Fleet's port, serving: two Jobs that need the owner (one escalated, one
/// asking), one that does not (queued), one running, one done.
async fn fake_fleet() -> u16 {
    let jobs = format!(
        r#"{{"jobs":[{},{},{},{},{}]}}"#,
        summary("j2", "escalated", r#","reason":{"named":"stalled"}"#),
        summary("j1", "awaiting_review", ""),
        summary("j3", "queued", ""),
        summary("j4", "running", ""),
        summary("j5", "completed_success", ""),
    );
    let jobs = Arc::new(jobs);
    let alerts = r#"{"blocked":[{"job_id":"j2","handle":"h","status":"escalated","since":"2026-10-08T01:30:00.000Z"}],"waiting":[{"job_id":"j1","handle":"h","status":"awaiting_review"}]}"#;
    let manifests = r#"[{"id":"m1","repository":"armada","path":"/srv/repo/armada.yml","records_root":"/r","version":1,"checks":[]}]"#;
    let app = Router::new()
        .route("/jobs", get({ let jobs = jobs.clone(); move || async move { (*jobs).clone() } }))
        .route("/alerts", get(move || async move { alerts }))
        .route("/manifests", get(move || async move { manifests }))
        .route(
            "/jobs/:id",
            get(|axum::extract::Path(id): axum::extract::Path<String>| async move {
                match id.as_str() {
                    "j2" => detail("j2", "escalated", r#","reason":{"named":"stalled"},"current_step_id":"s2""#).into_response(),
                    "j1" => detail("j1", "awaiting_review", r#","current_step_id":"s2""#).into_response(),
                    _ => StatusCode::NOT_FOUND.into_response(),
                }
            }),
        )
        .route("/events", get(events));
    let listener = crate::bind(0).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, app).await });
    port
}

/// A Gateway on this Fleet, with one phone paired to it.
struct App {
    rig: Rig,
    device: String,
}

async fn app(fleet: crate::Fleet) -> App {
    let rig = rig_with(fleet);
    let device = paired(&rig).await;
    App { rig, device }
}

async fn on(port: u16) -> App {
    app(Arc::new(move || Ok(port))).await
}

/// The clock moves a second each call, so no two signatures repeat.
fn headers(app: &App, path: &str) -> Vec<(&'static str, String)> {
    let time = app.rig.now.fetch_add(1, Ordering::Relaxed) + 1;
    signed(&app.device, "GET", path, time, "")
}

async fn get_body(app: &App, path: &str) -> (StatusCode, String) {
    call(&app.rig.app, Method::GET, path, &headers(app, path), "").await
}

#[tokio::test]
async fn needs_you_holds_what_bridge_holds_and_not_the_queue() {
    let (status, body) = get_body(&on(fake_fleet().await).await, "/api/needs").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    // Oldest first; the queued Job is not waiting on the owner.
    let (j1, j2) = (body.find("\"j1\"").unwrap(), body.find("\"j2\"").unwrap());
    assert!(j1 < j2, "{body}");
    for absent in ["\"j3\"", "\"j4\"", "\"j5\""] {
        assert!(!body.contains(absent), "{absent} in {body}");
    }
    assert!(body.contains(r#""repository":"armada""#), "{body}");
    assert!(body.contains(r#""reason":"stalled""#), "{body}");
    assert!(body.contains(r#""waiting_since":"2026-10-08T01:30:00.000Z""#), "{body}");
    assert!(body.contains(r#""step":{"at":2,"of":2,"name":"Implement"}"#), "{body}");
}

#[tokio::test]
async fn a_job_shows_its_checks_verdict_and_pull_request_and_nothing_outside_the_allowlist() {
    let (status, body) = get_body(&on(fake_fleet().await).await, "/api/jobs/j2").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    // Only the latest attempt, one row per Check name.
    assert!(
        body.contains(r#""checks":[{"name":"typecheck","passed":true},{"name":"store_test","passed":false}]"#),
        "{body}"
    );
    assert!(body.contains(r#""verdict":"veto""#), "{body}");
    assert!(body.contains(r#""pull_request":"https://example.test/pull/7""#), "{body}");
    // The brief, the Judge's consequence, a Check's output path and produced
    // text and the commit all carry the marker in Fleet's answer.
    assert!(!body.contains(SECRET), "{body}");
    for field in ["facts", "handle", "branch", "footprint", "output_path", "delivery", "model"] {
        assert!(!body.contains(&format!("\"{field}\"")), "{field} in {body}");
    }
}

#[tokio::test]
async fn the_lists_are_chosen_by_state_and_carry_no_marker() {
    let app = on(fake_fleet().await).await;
    let (_, running) = get_body(&app, "/api/jobs?state=running").await;
    assert!(running.contains("\"j4\"") && !running.contains("\"j3\"") && !running.contains("\"j2\""), "{running}");
    let (_, done) = get_body(&app, "/api/jobs?state=done").await;
    assert!(done.contains("\"j5\"") && !done.contains("\"j4\""), "{done}");
    assert!(!running.contains(SECRET) && !done.contains(SECRET));
    assert_eq!(get_body(&app, "/api/jobs").await.0, StatusCode::BAD_REQUEST);
    assert_eq!(get_body(&app, "/api/jobs/nope").await.0, StatusCode::NOT_FOUND);
    assert_eq!(get_body(&app, "/api/jobs/..%2Fadmin").await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn live_forwards_a_job_change_and_drops_everything_else() {
    let port = fake_fleet().await;
    let gateway_listener = crate::bind(0).await.unwrap();
    let at = gateway_listener.local_addr().unwrap().port();
    let app = on(port).await;
    let signed: String = headers(&app, "/api/live")
        .iter()
        .map(|(name, value)| format!("{name}: {value}\r\n"))
        .collect();
    let served = app.rig.app.clone();
    tokio::spawn(async move { axum::serve(gateway_listener, served).await });
    let mut socket = tokio::net::TcpStream::connect(("127.0.0.1", at)).await.unwrap();
    socket
        .write_all(format!("GET /api/live HTTP/1.1\r\nHost: x\r\n{signed}Connection: close\r\n\r\n").as_bytes())
        .await
        .unwrap();
    let mut raw = String::new();
    socket.read_to_string(&mut raw).await.unwrap();
    assert!(raw.contains("text/event-stream"), "{raw}");
    assert!(
        raw.contains(r#"data: {"change":"status","job_id":"j2","status":"escalated","reason":"stalled"}"#),
        "{raw}"
    );
    assert!(!raw.contains("files_changed") && !raw.contains("drone_id"), "{raw}");
}

#[tokio::test]
async fn fleet_being_down_says_what_to_do() {
    // Not running: the runtime file names no Fleet.
    let off = app(Arc::new(|| Err("no runtime file at /secret/path".to_string()))).await;
    let (status, body) = get_body(&off, "/api/needs").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body, "Armada is not running on your Mac. Open it there, then try again.");
    // Named, and nothing listening.
    let closed = {
        let listener = crate::bind(0).await.unwrap();
        listener.local_addr().unwrap().port()
    };
    let closed = on(closed).await;
    for path in ["/api/needs", "/api/jobs?state=running", "/api/jobs/j1", "/api/live"] {
        let (status, body) = get_body(&closed, path).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{path}");
        assert_eq!(body, "Armada is not answering on your Mac. Open it there, then try again.", "{path}");
    }
}
