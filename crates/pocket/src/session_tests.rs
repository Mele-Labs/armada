//! The Session routes and dispatch against a fake Fleet, with a marker in every
//! field the phone must not see.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use axum::extract::ws::{Message, WebSocketUpgrade};
use axum::http::{Method, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::pairing_tests::{call, paired, rig_with, signed, Rig};

const SECRET: &str = "SECRET-TRANSCRIPT-4c1";
const TERMINAL_ASK: &str = "TERMINAL-ASK-TEXT-77";

type Seen = Arc<Mutex<Vec<(String, String)>>>;

fn ask(call: &str, tool: &str, detail: &str, questions: &str) -> String {
    format!(
        r#"{{"call":"{call}","manifest_id":"m1","asked_at":"2026-10-08T05:00:00.000Z","tool":"{tool}","detail":"{detail}","truncated":false,"rule":"{tool}","offers":["allow_once","allow_and_remember","refuse"],"holding_for_seconds":600{questions}}}"#
    )
}

fn record(id: &str, origin: &str, state: &str, tail: &str) -> String {
    format!(
        r#"{{"id":"{id}","harness":"term","origin":"{origin}","manifest_id":"m1","cwd":"/{SECRET}","title":"Title {id}","state":"{state}","started_at":"2026-10-08T01:00:00.000Z","last_seen_at":"2026-10-08T01:00:00.000Z","end_reason":"{SECRET}","usage":{{}},"attachments":[]{tail}}}"#
    )
}

fn hosted(asked: &str) -> String {
    format!(
        r#","hosted":{{"turn":{{"state":"idle"}},"mode":"auto","running":true,"model":"{SECRET}"{asked}}}"#
    )
}

fn sessions() -> String {
    let question = r#","questions":[{"question":"Which branch?","header":"Branch","multi_select":false,"options":[{"label":"main","description":"the trunk"},{"label":"dev"}]}]"#;
    let rows = [
        record("h1", "bridge", "live", &hosted(&format!(r#","asked":{}"#, ask("c1", "AskUserQuestion", "", question)))),
        record("h2", "bridge", "live", &hosted(&format!(r#","asked":{}"#, ask("c2", "Bash", "ls -la", "")))),
        record("h3", "bridge", "live", &format!(r#"{}{}"#, hosted(""), r#","standing":"working""#)),
        record(
            "t1",
            "terminal",
            "live",
            &format!(
                r#","terminal":{{"listening":true,"commands":[{{"name":"{SECRET}"}}],"asked":{}}}"#,
                ask("tc", "AskUserQuestion", TERMINAL_ASK, &format!(r#","questions":[{{"question":"{TERMINAL_ASK}"}}]"#))
            ),
        ),
        // Asked at the same minute, held for 60 seconds: lapsed by the test's clock.
        record(
            "t3",
            "terminal",
            "live",
            &format!(
                r#","terminal":{{"listening":true,"asked":{}}}"#,
                ask("tc3", "Bash", "ls", "").replace(r#""holding_for_seconds":600"#, r#""holding_for_seconds":60"#)
            ),
        ),
        // Held, but its mod has stopped listening: nobody would collect an answer.
        record(
            "t4",
            "terminal",
            "live",
            &format!(r#","terminal":{{"listening":false,"asked":{}}}"#, ask("tc4", "Bash", "ls", "")),
        ),
        record("t2", "terminal", "live", ""),
        record(
            "e1",
            "bridge",
            "ended",
            &format!(r#"{}{}"#, hosted(&format!(r#","asked":{}"#, ask("c9", "Bash", "x", ""))), r#","ended_at":"2026-10-08T04:00:00.000Z""#),
        ),
        record("e2", "terminal", "ended", r#","ended_at":"2026-10-08T05:00:00.000Z""#),
    ];
    format!(r#"{{"sessions":[{}]}}"#, rows.join(","))
}

/// Fleet's events: a Session that changed, and one that is not a Session.
async fn events(upgrade: WebSocketUpgrade) -> impl IntoResponse {
    upgrade.on_upgrade(|mut socket| async move {
        let changed = record("h3", "bridge", "live", r#","standing":"stuck""#).replacen('{', r#"{"kind":"session.changed","#, 1);
        let changed = format!(r#"{{"message":"event","cursor":2,"event":{changed}}}"#);
        let other = r#"{"message":"event","cursor":3,"event":{"kind":"repositories.changed","repositories":[]}}"#;
        for text in [other.to_string(), changed] {
            socket.send(Message::Text(text)).await.ok();
        }
    })
}

async fn fake_fleet(seen: Seen, start_status: u16) -> u16 {
    let manifests = r#"[{"id":"m1","repository":"armada","path":"/a/armada.yml","records_root":"/r","version":1,"checks":[]},{"id":"m2","repository":"notes","path":"/n/armada.yml","records_root":"/r","version":1,"checks":[]}]"#;
    let answered = seen.clone();
    let started = seen;
    let app = Router::new()
        .route("/sessions", get(|| async { sessions() }))
        .route("/manifests", get(move || async move { manifests }))
        .route("/events", get(events))
        .route(
            "/sessions/ask/answer",
            post(move |body: String| async move {
                answered.lock().unwrap().push(("/sessions/ask/answer".into(), body));
                StatusCode::OK
            }),
        )
        .route(
            "/sessions/start",
            post(move |body: String| async move {
                started.lock().unwrap().push(("/sessions/start".into(), body));
                if start_status == 200 {
                    (StatusCode::OK, record("s9", "bridge", "live", &hosted("")))
                } else {
                    (
                        StatusCode::from_u16(start_status).unwrap(),
                        format!(r#"{{"code":"x","message":"Say what you want done.","run_id":"r","fields":{{}},"chain":["{SECRET}"]}}"#),
                    )
                }
            }),
        );
    let listener = crate::bind(0).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, app).await });
    port
}

struct App {
    rig: Rig,
    device: String,
}

/// 2026-10-08T05:05:00Z: five minutes into a 600-second hold, past a 60-second one.
const FIVE_PAST: i64 = 1_791_435_900;

async fn on(port: u16) -> App {
    let rig = rig_with(Arc::new(move || Ok(port)));
    let device = paired(&rig).await;
    rig.now.store(FIVE_PAST, Ordering::Relaxed);
    App { rig, device }
}

async fn send(app: &App, method: Method, path: &str, body: &str) -> (StatusCode, String) {
    let time = app.rig.now.fetch_add(1, Ordering::Relaxed) + 1;
    let headers = signed(&app.device, method.as_str(), path, time, body);
    call(&app.rig.app, method, path, &headers, body).await
}

async fn rig() -> (App, Seen) {
    let seen = Seen::default();
    (on(fake_fleet(seen.clone(), 200).await).await, seen)
}

#[tokio::test]
async fn sessions_are_trimmed_and_a_terminal_one_carries_its_ask_while_held() {
    let (app, _) = rig().await;
    let (status, body) = send(&app, Method::GET, "/api/sessions", "").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(!body.contains(SECRET), "{body}");
    for field in ["cwd", "model", "commands", "end_reason", "usage", "attachments", "hosted", "terminal"] {
        assert!(!body.contains(&format!("\"{field}\":")), "{field} in {body}");
    }
    assert!(!body.contains("\"e1\"") && !body.contains("\"e2\""), "{body}");
    // Only a Session with a turn running is working; each carries when it began.
    let working = &body[body.find(r#""id":"h3""#).unwrap()..];
    let working = &working[..working.find('}').unwrap()];
    assert!(working.contains(r#""working":true"#) && working.contains(r#""started_at":"2026-10-08T01:00:00.000Z""#), "{working}");
    assert!(body.matches(r#""working":true"#).count() == 1, "{body}");
    // The hosted question: its text, its options, its id.
    assert!(body.contains(r#""ask_id":"c1""#) && body.contains("Which branch?") && body.contains(r#""label":"dev""#), "{body}");
    assert!(body.contains(r#""repository":"armada""#), "{body}");
    // The permission ask offers allow_once and refuse, and not the remembered rule.
    assert!(body.contains(r#""offers":["allow_once","refuse"]"#), "{body}");
    // A Terminal Session whose question Fleet holds shows it, and stays a terminal one.
    let held = &body[body.find(r#""id":"t1""#).unwrap()..];
    let held = &held[..held.find(r#""id":"t3""#).unwrap_or(held.len())];
    assert!(held.contains(r#""kind":"terminal""#) && held.contains(r#""waiting":true"#), "{held}");
    assert!(held.contains(r#""ask_id":"tc""#) && held.contains(TERMINAL_ASK), "{held}");
    assert!(held.contains(r#""offers":["allow_once","refuse"]"#), "{held}");
    // Once the hold has lapsed it is still waiting, with nothing to answer.
    let lapsed = &body[body.find(r#""id":"t3""#).unwrap()..];
    let lapsed = &lapsed[..lapsed.find('}').unwrap()];
    assert!(lapsed.contains(r#""kind":"terminal""#) && lapsed.contains(r#""waiting":true"#), "{lapsed}");
    assert!(!lapsed.contains("ask"), "{lapsed}");
    // Held but not listening: waiting, and nothing to answer.
    let deaf = &body[body.find(r#""id":"t4""#).unwrap()..];
    let deaf = &deaf[..deaf.find('}').unwrap()];
    assert!(deaf.contains(r#""waiting":true"#) && !deaf.contains("ask"), "{deaf}");
}

#[tokio::test]
async fn the_ended_sessions_come_newest_first_and_carry_no_marker() {
    let (app, _) = rig().await;
    let (status, body) = send(&app, Method::GET, "/api/sessions?state=ended", "").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (e2, e1) = (body.find("\"e2\"").unwrap(), body.find("\"e1\"").unwrap());
    assert!(e2 < e1, "{body}");
    assert!(body.contains(r#""ended_at":"2026-10-08T05:00:00.000Z""#), "{body}");
    assert!(!body.contains("\"h1\"") && !body.contains(SECRET) && !body.contains("ask_id"), "{body}");
    assert_eq!(send(&app, Method::GET, "/api/sessions?state=nope", "").await.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn needs_holds_the_waiting_sessions_and_not_the_idle_ones() {
    let (app, _) = rig().await;
    let (status, body) = send(&app, Method::GET, "/api/needs", "").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let sessions = &body[body.find(r#""sessions""#).unwrap()..];
    for present in ["\"h1\"", "\"h2\"", "\"t1\"", "\"t3\""] {
        assert!(sessions.contains(present), "{present} in {sessions}");
    }
    for absent in ["\"h3\"", "\"t2\"", "\"e1\""] {
        assert!(!sessions.contains(absent), "{absent} in {sessions}");
    }
    assert!(!body.contains(SECRET), "{body}");
}

#[tokio::test]
async fn a_hosted_answer_reaches_fleet_with_its_body() {
    let (app, seen) = rig().await;
    let chosen = r#"{"session_id":"h1","answer":[{"question":"Which branch?","chosen":["main"]}]}"#;
    assert_eq!(send(&app, Method::POST, "/api/sessions/answer", chosen).await.0, StatusCode::NO_CONTENT);
    let refuse = r#"{"session_id":"h2","ask_id":"c2","answer":"refuse"}"#;
    assert_eq!(send(&app, Method::POST, "/api/sessions/answer", refuse).await.0, StatusCode::NO_CONTENT);
    let seen = seen.lock().unwrap();
    assert_eq!(
        seen[0].1,
        r#"{"session_id":"h1","call":"c1","answer":"allow_once","answers":[{"question":"Which branch?","chosen":["main"]}]}"#
    );
    assert_eq!(seen[1].1, r#"{"session_id":"h2","call":"c2","answer":"refuse"}"#);
}

#[tokio::test]
async fn a_terminal_answer_reaches_fleet_with_its_body() {
    let (app, seen) = rig().await;
    let refuse = r#"{"session_id":"t1","ask_id":"tc","answer":"refuse"}"#;
    assert_eq!(send(&app, Method::POST, "/api/sessions/answer", refuse).await.0, StatusCode::NO_CONTENT);
    let chosen = r#"{"session_id":"t1","answer":[{"question":"TERMINAL-ASK-TEXT-77","chosen":["a"]}]}"#;
    assert_eq!(send(&app, Method::POST, "/api/sessions/answer", chosen).await.0, StatusCode::NO_CONTENT);
    let seen = seen.lock().unwrap();
    assert_eq!(seen[0], ("/sessions/ask/answer".to_string(), r#"{"session_id":"t1","call":"tc","answer":"refuse"}"#.to_string()));
    assert_eq!(
        seen[1].1,
        r#"{"session_id":"t1","call":"tc","answer":"allow_once","answers":[{"question":"TERMINAL-ASK-TEXT-77","chosen":["a"]}]}"#
    );
}

#[tokio::test]
async fn answers_that_cannot_be_given_are_refused_before_fleet() {
    let (app, seen) = rig().await;
    let cases = [
        (r#"{"session_id":"t3","answer":"refuse"}"#, StatusCode::CONFLICT, "Armada is no longer holding that question. Answer it in the terminal."),
        (r#"{"session_id":"t4","answer":"refuse"}"#, StatusCode::CONFLICT, "Armada is no longer holding that question. Answer it in the terminal."),
        (r#"{"session_id":"t2","answer":"refuse"}"#, StatusCode::CONFLICT, "Armada is no longer holding that question. Answer it in the terminal."),
        (r#"{"session_id":"t1","ask_id":"old","answer":"refuse"}"#, StatusCode::CONFLICT, "That question has been answered or has changed. Open it again."),
        (r#"{"session_id":"h3","answer":"refuse"}"#, StatusCode::CONFLICT, "That Job is not waiting on an answer."),
        (r#"{"session_id":"h2","ask_id":"old","answer":"refuse"}"#, StatusCode::CONFLICT, "That question has been answered or has changed. Open it again."),
        (r#"{"session_id":"h1","answer":"allow_once"}"#, StatusCode::BAD_REQUEST, "That answer does not fit the question. Open it again."),
        (r#"{"session_id":"nope","answer":"refuse"}"#, StatusCode::NOT_FOUND, "That Job is not here."),
        (r#"{"session_id":"h2","answer":"refuse","extra":1}"#, StatusCode::BAD_REQUEST, "Armada could not read that from this phone. Update the app."),
        (r#"{"session_id":"h2","answer":"allow_and_remember"}"#, StatusCode::BAD_REQUEST, "Armada could not read that from this phone. Update the app."),
    ];
    for (body, status, sentence) in cases {
        assert_eq!(send(&app, Method::POST, "/api/sessions/answer", body).await, (status, sentence.to_string()), "{body}");
    }
    assert!(seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn dispatch_starts_a_session_on_the_manifest_the_label_maps_to() {
    let (app, seen) = rig().await;
    let (status, body) = send(&app, Method::POST, "/api/sessions", r#"{"text":"Fix the thing","repository":"notes"}"#).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert!(body.contains(r#""id":"s9""#) && body.contains(r#""repository":"notes""#), "{body}");
    assert!(!body.contains(SECRET), "{body}");
    assert_eq!(
        seen.lock().unwrap()[0],
        ("/sessions/start".to_string(), r#"{"manifest_id":"m2","request":"Fix the thing"}"#.to_string())
    );
}

#[tokio::test]
async fn dispatch_refuses_what_it_does_not_know_and_passes_fleets_refusal() {
    let (app, seen) = rig().await;
    for body in [
        r#"{"text":"x","repository":"m1"}"#,
        r#"{"text":"x","repository":"nope"}"#,
        r#"{"text":"x","repository":"armada","manifest_id":"m2"}"#,
    ] {
        let (status, _) = send(&app, Method::POST, "/api/sessions", body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    }
    assert!(seen.lock().unwrap().is_empty());
    let refusing = on(fake_fleet(Seen::default(), 422).await).await;
    let (status, body) = send(&refusing, Method::POST, "/api/sessions", r#"{"text":"","repository":"armada"}"#).await;
    assert_eq!((status, body.as_str()), (StatusCode::UNPROCESSABLE_ENTITY, "Say what you want done."));
}

#[tokio::test]
async fn repositories_are_the_labels() {
    let (app, _) = rig().await;
    assert_eq!(send(&app, Method::GET, "/api/repositories", "").await, (StatusCode::OK, r#"["armada","notes"]"#.to_string()));
}

#[tokio::test]
async fn live_forwards_a_session_change_and_drops_everything_else() {
    let port = fake_fleet(Seen::default(), 200).await;
    let gateway_listener = crate::bind(0).await.unwrap();
    let at = gateway_listener.local_addr().unwrap().port();
    let app = on(port).await;
    let time = app.rig.now.fetch_add(1, Ordering::Relaxed) + 1;
    let signed: String = signed(&app.device, "GET", "/api/live", time, "")
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
    assert!(raw.contains(r#"data: {"change":"session","session_id":"h3"}"#), "{raw}");
    assert!(!raw.contains("repositories") && !raw.contains(SECRET), "{raw}");
}

#[tokio::test]
async fn unsigned_is_401_and_fleet_down_says_what_to_do() {
    let (app, _) = rig().await;
    for (method, path) in [
        (Method::GET, "/api/sessions"),
        (Method::POST, "/api/sessions/answer"),
        (Method::POST, "/api/sessions"),
        (Method::GET, "/api/needs"),
        (Method::GET, "/api/repositories"),
    ] {
        assert_eq!(call(&app.rig.app, method, path, &[], "{}").await.0, StatusCode::UNAUTHORIZED, "{path}");
    }
    let closed = {
        let listener = crate::bind(0).await.unwrap();
        listener.local_addr().unwrap().port()
    };
    let down = on(closed).await;
    for (method, path, body) in [
        (Method::GET, "/api/sessions", ""),
        (Method::GET, "/api/sessions?state=ended", ""),
        (Method::GET, "/api/needs", ""),
        (Method::GET, "/api/live", ""),
        (Method::GET, "/api/repositories", ""),
        (Method::POST, "/api/sessions/answer", r#"{"session_id":"h2","answer":"refuse"}"#),
        (Method::POST, "/api/sessions", r#"{"text":"x","repository":"armada"}"#),
    ] {
        let (status, said) = send(&down, method, path, body).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{path}");
        assert_eq!(said, "Armada is not answering on your Mac. Open it there, then try again.", "{path}");
    }
}
