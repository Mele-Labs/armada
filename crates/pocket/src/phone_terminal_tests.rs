//! A paired phone lists a Job's terminals, types into one and follows it, each call passed to Fleet and
//! Fleet's answer passed back as it came.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use axum::extract::ws::{Message, WebSocketUpgrade};
use axum::extract::{Path, RawQuery};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use ipc::{ProtocolId, SessionRunMessage};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::pairing_tests::{call, paired, rig_with, signed, Rig};

/// What Fleet was asked: the path, the caller header, and the query or body.
type Seen = Arc<Mutex<Vec<(String, String, String)>>>;

const LISTED: &str = r#"{"terminals":[{"terminal":"t1","name":"Terminal 1","run_id":"session-run-1","running":false},{"terminal":"t2","name":"dev server","run_id":"session-run-2","running":true}]}"#;

fn caller(headers: &HeaderMap) -> String {
    headers.get("x-armada-caller").and_then(|v| v.to_str().ok()).unwrap_or("").to_string()
}

/// Fleet's observe: what the run printed, then how it ended, then the socket closes.
async fn observe(Path(run_id): Path<String>, upgrade: WebSocketUpgrade) -> impl IntoResponse {
    upgrade.on_upgrade(move |mut socket| async move {
        let messages = [
            SessionRunMessage::Opened { protocol_id: ProtocolId::current(), run_id },
            SessionRunMessage::Output { text: "$ ls\r\n".into() },
            SessionRunMessage::Output { text: "notes\r\n".into() },
            SessionRunMessage::Exited { code: Some(0), stopped: false },
        ];
        for message in messages {
            socket.send(Message::Text(ipc::encode(&message).unwrap())).await.ok();
        }
    })
}

async fn fake_fleet(seen: Seen) -> u16 {
    let listed = seen.clone();
    let wrote = seen;
    let app = Router::new()
        .route(
            "/sessions/terminal/list",
            get(move |headers: HeaderMap, RawQuery(query): RawQuery| async move {
                let query = query.unwrap_or_default();
                listed.lock().unwrap().push(("/sessions/terminal/list".into(), caller(&headers), query.clone()));
                match query.as_str() {
                    "session_id=h1" => (StatusCode::OK, LISTED.to_string()),
                    _ => (StatusCode::UNPROCESSABLE_ENTITY, r#"{"code":"fleet.no_job","message":"No such Job."}"#.to_string()),
                }
            }),
        )
        .route(
            "/sessions/run/write",
            post(move |headers: HeaderMap, body: String| async move {
                wrote.lock().unwrap().push(("/sessions/run/write".into(), caller(&headers), body.clone()));
                if body.contains("session-run-1") {
                    (StatusCode::OK, r#"{"run_id":"session-run-1","terminal":"t1"}"#.to_string())
                } else {
                    (StatusCode::UNPROCESSABLE_ENTITY, r#"{"code":"fleet.run_ended","message":"That run has ended."}"#.to_string())
                }
            }),
        )
        .route("/sessions/run/:run_id/observe", get(observe));
    let listener = crate::bind(0).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, app).await });
    port
}

struct App {
    rig: Rig,
    device: String,
}

async fn on(port: u16) -> App {
    let rig = rig_with(Arc::new(move || Ok(port)));
    let device = paired(&rig).await;
    App { rig, device }
}

async fn send(app: &App, method: Method, path: &str, body: &str) -> (StatusCode, String) {
    let time = app.rig.now.fetch_add(1, Ordering::Relaxed) + 1;
    let headers = signed(&app.device, method.as_str(), path, time, body);
    call(&app.rig.app, method, path, &headers, body).await
}

async fn rig() -> (App, Seen) {
    let seen = Seen::default();
    (on(fake_fleet(seen.clone()).await).await, seen)
}

#[tokio::test]
async fn the_phone_lists_a_jobs_terminals_as_fleet_answers_them() {
    let (app, seen) = rig().await;
    let (status, body) = send(&app, Method::GET, "/api/sessions/terminals?session_id=h1", "").await;
    assert_eq!((status, body.as_str()), (StatusCode::OK, LISTED));
    assert_eq!(*seen.lock().unwrap(), [("/sessions/terminal/list".to_string(), "phone".to_string(), "session_id=h1".to_string())]);
    // Fleet's refusal comes back with its status and words, not a Gateway sentence.
    let (status, body) = send(&app, Method::GET, "/api/sessions/terminals?session_id=nope", "").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert!(body.contains("fleet.no_job"), "{body}");
}

#[tokio::test]
async fn the_phone_types_into_a_terminal_and_sees_fleets_refusal_of_one_that_ended() {
    let (app, seen) = rig().await;
    let typed = r#"{"run_id":"session-run-1","data":"ls\r"}"#;
    let (status, body) = send(&app, Method::POST, "/api/sessions/terminal/write", typed).await;
    assert_eq!((status, body.as_str()), (StatusCode::OK, r#"{"run_id":"session-run-1","terminal":"t1"}"#));
    assert_eq!(*seen.lock().unwrap(), [("/sessions/run/write".to_string(), "phone".to_string(), typed.to_string())]);
    let ended = r#"{"run_id":"session-run-9","data":"x"}"#;
    let (status, body) = send(&app, Method::POST, "/api/sessions/terminal/write", ended).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert!(body.contains("fleet.run_ended"), "{body}");
}

#[tokio::test]
async fn an_unsigned_call_to_a_terminal_route_is_refused_and_reaches_nothing() {
    let (app, seen) = rig().await;
    for (method, path) in [
        (Method::GET, "/api/sessions/terminals?session_id=h1"),
        (Method::POST, "/api/sessions/terminal/write"),
        (Method::GET, "/api/sessions/terminal/observe?run_id=session-run-1"),
    ] {
        assert_eq!(call(&app.rig.app, method, path, &[], "{}").await.0, StatusCode::BAD_REQUEST, "{path}");
    }
    assert!(seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_terminal_that_cannot_be_reached_is_refused_at_once_and_never_queued() {
    let closed = {
        let listener = crate::bind(0).await.unwrap();
        listener.local_addr().unwrap().port()
    };
    let down = on(closed).await;
    for (method, path, body) in [
        (Method::GET, "/api/sessions/terminals?session_id=h1", ""),
        (Method::POST, "/api/sessions/terminal/write", r#"{"run_id":"session-run-1","data":"x"}"#),
        (Method::GET, "/api/sessions/terminal/observe?run_id=session-run-1", ""),
    ] {
        let (status, said) = send(&down, method, path, body).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{path}");
        assert_eq!(said, "Armada is not answering on your Mac. Open it there, then try again.", "{path}");
    }
}

#[tokio::test]
async fn following_a_terminal_relays_each_message_as_a_line_and_ends_after_exited() {
    let app = on(fake_fleet(Seen::default()).await).await;
    let listener = crate::bind(0).await.unwrap();
    let at = listener.local_addr().unwrap().port();
    let path = "/api/sessions/terminal/observe?run_id=session-run-1";
    let time = app.rig.now.fetch_add(1, Ordering::Relaxed) + 1;
    let headers: String = signed(&app.device, "GET", path, time, "").iter().map(|(name, value)| format!("{name}: {value}\r\n")).collect();
    let served = app.rig.app.clone();
    tokio::spawn(async move { axum::serve(listener, served).await });
    let mut socket = tokio::net::TcpStream::connect(("127.0.0.1", at)).await.unwrap();
    socket.write_all(format!("GET {path} HTTP/1.1\r\nHost: x\r\n{headers}Connection: close\r\n\r\n").as_bytes()).await.unwrap();
    let mut raw = String::new();
    // `read_to_string` returns only where the stream ended, which is the claim.
    tokio::time::timeout(std::time::Duration::from_secs(5), socket.read_to_string(&mut raw)).await.expect("the stream did not end").unwrap();
    assert!(raw.contains("application/x-ndjson"), "{raw}");
    let lines: Vec<&str> = raw.split("\r\n\r\n").nth(1).unwrap().lines().filter(|line| line.starts_with('{')).collect();
    let kinds: Vec<&str> = lines.iter().map(|line| &line[..line.find(',').unwrap()]).collect();
    assert_eq!(kinds, [r#"{"kind":"opened""#, r#"{"kind":"output""#, r#"{"kind":"output""#, r#"{"kind":"exited""#], "{raw}");
    assert_eq!(lines[1], r#"{"kind":"output","text":"$ ls\r\n"}"#);
    assert_eq!(lines[2], r#"{"kind":"output","text":"notes\r\n"}"#);
    assert_eq!(lines[3], r#"{"kind":"exited","code":0,"stopped":false}"#);
}
