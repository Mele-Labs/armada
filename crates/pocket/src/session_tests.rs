//! The Session routes and dispatch against a fake Fleet, with a marker in every
//! field the phone must not see.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use axum::extract::ws::{Message, WebSocketUpgrade};
use axum::extract::RawQuery;
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
    fake_fleet_serving(seen, start_status, sessions()).await
}

async fn fake_fleet_serving(seen: Seen, start_status: u16, rows: String) -> u16 {
    fake_fleet_with(seen, start_status, 202, rows).await
}

/// A Fleet whose `/sessions/message` answers `message_status`: 202 as Fleet does, else a refusal.
async fn fake_fleet_messaging(seen: Seen, message_status: u16) -> u16 {
    fake_fleet_with(seen, 200, message_status, sessions()).await
}

/// A thread as `/sessions/one` answers it: the record, then its rows.
fn thread(session: String, rows: &[String]) -> String {
    format!(r#"{{"session":{session},"rows":[{}]}}"#, rows.join(","))
}

fn said(id: &str, voice: &str) -> String {
    format!(r#"{{"kind":"message","id":"{id}","at":"2026-10-08T01:00:00.000Z","from":{{"kind":"{voice}"}},"text":"words"}}"#)
}

/// The Jobs a fork can name. `live1` ends on the person's words, after two of the agent's.
fn threads(id: &str) -> Option<String> {
    let live = || record("live1", "bridge", "live", &hosted(""));
    let ended = format!(r#"{}{}"#, hosted(""), r#","ended_at":"2026-10-08T04:00:00.000Z""#);
    Some(match id {
        "live1" => thread(live(), &[said("m-1", "you"), said("a-1", "agent"), said("a-2", "agent"), said("m-3", "you")]),
        "quiet1" => thread(record("quiet1", "bridge", "live", &hosted("")), &[said("m-1", "you")]),
        "done1" => thread(record("done1", "bridge", "ended", &ended), &[said("m-1", "you"), said("a-1", "agent")]),
        "bare1" => thread(record("bare1", "bridge", "ended", &ended).replace(r#""manifest_id":"m1","#, ""), &[said("a-1", "agent")]),
        _ => return None,
    })
}

/// One project of this Mac, shared, holding both repositories: where a Job the phone starts goes.
fn shared_projects() -> String {
    let members = ipc::decode::<Vec<ipc::ProjectMember>>("members", br#"[{"manifest_id":"m1"},{"manifest_id":"m2"}]"#).unwrap();
    let shop = ipc::Project { id: "shop".into(), name: "Shop".into(), members, shared_with_macs: true, ..Default::default() };
    ipc::encode(&ipc::Projects { projects: vec![shop], loose: Vec::new(), left_out: Vec::new() }).unwrap()
}

async fn fake_fleet_with(seen: Seen, start_status: u16, message_status: u16, rows: String) -> u16 {
    let manifests = r#"[{"id":"m1","repository":"armada","path":"/a/armada.yml","records_root":"/r","version":1,"checks":[]},{"id":"m2","repository":"notes","path":"/n/armada.yml","records_root":"/r","version":1,"checks":[]}]"#;
    let answered = seen.clone();
    let messaged = seen.clone();
    let dispatched = seen.clone();
    let started = seen;
    let app = Router::new()
        .route("/sessions", get(move || async move { rows }))
        .route("/manifests", get(move || async move { manifests }))
        .route("/projects", get(|| async { shared_projects() }))
        .route("/events", get(events))
        .route(
            "/sessions/one",
            get(|RawQuery(query): RawQuery| async move {
                match query.as_deref().and_then(|query| query.strip_prefix("session_id=")).and_then(threads) {
                    Some(thread) => (StatusCode::OK, thread),
                    None => (StatusCode::NOT_FOUND, r#"{"code":"fleet.no_session","message":"No such job."}"#.to_string()),
                }
            }),
        )
        .route(
            "/sessions/message",
            post(move |body: String| async move {
                messaged.lock().unwrap().push(("/sessions/message".into(), body));
                match message_status {
                    202 => (StatusCode::ACCEPTED, String::new()),
                    other => (
                        StatusCode::from_u16(other).unwrap(),
                        format!(r#"{{"code":"x","message":"That Job has ended.","run_id":"r","fields":{{}},"chain":["{SECRET}"]}}"#),
                    ),
                }
            }),
        )
        .route(
            "/sessions/ask/answer",
            post(move |body: String| async move {
                answered.lock().unwrap().push(("/sessions/ask/answer".into(), body));
                StatusCode::OK
            }),
        )
        .route(
            "/sessions/dispatch",
            post(move |body: String| async move {
                dispatched.lock().unwrap().push(("/sessions/dispatch".into(), body));
                (StatusCode::OK, r#"{"operator":"d1"}"#)
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

fn owed(id: &str, source: &str, since: &str) -> String {
    format!(r#"{{"id":"{id}","text":"Owed {id}","since":"2026-10-08T{since}:00.000Z","source":"{source}"}}"#)
}

/// A live hosted Job: `turn` is the hosted turn, `tail` the rest of the record.
fn job(id: &str, turn: &str, tail: &str) -> String {
    let hosted = format!(r#","hosted":{{"turn":{{"state":"{turn}"}},"mode":"auto","running":true}}"#);
    record(id, "bridge", "live", &format!("{hosted}{tail}"))
}

fn owing(items: &[String]) -> String {
    format!(r#","waiting_for":[{}]"#, items.join(","))
}

/// Rows that Bridge counts as waiting or not by what the Job owes.
fn owing_sessions() -> String {
    let asked = format!(r#","asked":{}"#, ask("ca", "Bash", "ls", ""));
    let ask_and_plan = record(
        "p6",
        "bridge",
        "live",
        &format!(
            r#","hosted":{{"turn":{{"state":"idle"}},"mode":"auto","running":true{asked}}}{}"#,
            owing(&[owed("plan:1", "plan", "03:00")])
        ),
    );
    let rows = [
        job("p1", "idle", &owing(&[owed("plan:1", "plan", "03:00")])),
        job("p2", "idle", &owing(&[owed("done:1", "done", "03:00")])),
        job("p3", "working", &owing(&[owed("a1", "agent", "03:00")])),
        job("p4", "working", &owing(&[owed("delivery", "delivery", "02:00")])),
        job("p5", "idle", &format!(r#"{}{}"#, r#","standing":"stuck""#, owing(&[owed("a1", "agent", "03:00")]))),
        ask_and_plan,
        job(
            "p7",
            "idle",
            &owing(&[owed("a1", "agent", "04:00"), owed("w1", "walk", "03:30"), owed("done:1", "done", "01:00")]),
        ),
        job("p8", "working", &owing(&[owed("a1", "agent", "02:00"), owed("delivery", "delivery", "04:00")])),
        record("p9", "bridge", "ended", &format!(r#"{}{}"#, owing(&[owed("plan:1", "plan", "03:00")]), r#","ended_at":"2026-10-08T04:00:00.000Z""#)),
    ];
    format!(r#"{{"sessions":[{}]}}"#, rows.join(","))
}

/// One row of a `/api/sessions` or `/api/needs` body, up to the next row.
fn row<'a>(body: &'a str, id: &str) -> &'a str {
    let from = &body[body.find(&format!(r#""id":"{id}""#)).unwrap_or_else(|| panic!("{id} not in {body}"))..];
    &from[..from[1..].find(r#""id":""#).map_or(from.len(), |at| at + 1)]
}

#[tokio::test]
async fn a_job_owing_the_person_something_is_waiting_as_bridge_counts_it() {
    let app = on(fake_fleet_serving(Seen::default(), 200, owing_sessions()).await).await;
    let (_, body) = send(&app, Method::GET, "/api/sessions", "").await;
    let (_, needs) = send(&app, Method::GET, "/api/needs", "").await;
    for waiting in ["p1", "p4", "p6", "p7", "p8"] {
        assert!(row(&body, waiting).contains(r#""waiting":true"#), "{waiting} in {body}");
        assert!(needs.contains(&format!(r#""id":"{waiting}""#)), "{waiting} in {needs}");
    }
    // Only a done, or an agent's item on a Job at work, is nothing owed.
    for idle in ["p2", "p3", "p5"] {
        assert!(!row(&body, idle).contains(r#""waiting":true"#), "{idle} in {body}");
        assert!(!needs.contains(&format!(r#""id":"{idle}""#)), "{idle} in {needs}");
    }
    assert!(!body.contains(r#""id":"p9""#) && !needs.contains(r#""id":"p9""#), "{body}");
}

#[tokio::test]
async fn the_wait_is_dated_from_the_ask_or_else_the_oldest_item_that_made_it_wait() {
    let app = on(fake_fleet_serving(Seen::default(), 200, owing_sessions()).await).await;
    let (_, body) = send(&app, Method::GET, "/api/sessions", "").await;
    for (id, since) in [
        // The ask, though the plan is older.
        ("p6", "2026-10-08T05:00:00.000Z"),
        // The walk, not the newer agent item and not the older done.
        ("p7", "2026-10-08T03:30:00.000Z"),
        // The delivery, not the agent item the working turn sets aside.
        ("p8", "2026-10-08T04:00:00.000Z"),
        ("p4", "2026-10-08T02:00:00.000Z"),
    ] {
        assert!(row(&body, id).contains(&format!(r#""waiting_since":"{since}""#)), "{id} in {body}");
    }
}

#[tokio::test]
async fn sessions_are_the_live_ones_and_a_state_is_not_asked_for() {
    let (app, _) = rig().await;
    let (status, body) = send(&app, Method::GET, "/api/sessions?state=ended", "").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body.contains("\"h1\"") && !body.contains("\"e1\"") && !body.contains("\"e2\""), "{body}");
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
    // The words name the Job, then reach the agent as its first message, to the Job just started.
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 2, "{seen:?}");
    assert_eq!(seen[0], ("/sessions/start".to_string(), r#"{"manifest_id":"m2","request":"Fix the thing","project":"shop"}"#.to_string()));
    assert_eq!(seen[1], ("/sessions/message".to_string(), r#"{"session_id":"s9","text":"Fix the thing"}"#.to_string()));
}

#[tokio::test]
async fn a_dispatch_whose_words_do_not_reach_the_job_says_the_job_started_without_them() {
    let seen = Seen::default();
    let app = on(fake_fleet_messaging(seen.clone(), 409).await).await;
    let (status, body) = send(&app, Method::POST, "/api/sessions", r#"{"text":"Fix the thing","repository":"notes"}"#).await;
    assert_eq!(
        (status, body.as_str()),
        (StatusCode::CONFLICT, "The Job started, but your message did not reach it. That Job has ended.")
    );
    assert_eq!(seen.lock().unwrap().len(), 2, "the start was made and the message tried");

    let down = on(fake_fleet_messaging(Seen::default(), 500).await).await;
    let (status, body) = send(&down, Method::POST, "/api/sessions", r#"{"text":"Fix the thing","repository":"notes"}"#).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert!(body.starts_with("The Job started, but your message did not reach it. "), "{body}");
    assert!(!body.contains(SECRET), "{body}");
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
async fn an_operator_dispatch_reaches_fleet_as_a_dispatch_with_its_kind_and_no_mac_on_this_mac() {
    let (app, seen) = rig().await;
    let (status, body) = send(&app, Method::POST, "/api/sessions/dispatch", r#"{"text":"Fix the thing","repository":"notes","kind":"bug"}"#).await;
    assert_eq!((status, body.as_str()), (StatusCode::OK, r#"{"operator":"d1"}"#));
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1, "{seen:?}");
    let (path, sent) = &seen[0];
    assert_eq!(path, "/sessions/dispatch");
    assert!(sent.contains(r#""manifest_id":"m2""#) && sent.contains(r#""request":"Fix the thing""#) && sent.contains(r#""kind":"bug""#), "{sent}");
    assert!(!sent.contains(r#""mac""#) && !sent.contains("on_your_words"), "{sent}");
    assert!(sent.contains(r#""project":"shop""#), "a Job the phone starts here goes into the shared project: {sent}");
}

#[tokio::test]
async fn an_operator_dispatch_names_only_a_mac_this_one_knows_and_takes_nothing_else() {
    let (app, seen) = rig().await;
    let (status, _) = send(&app, Method::POST, "/api/sessions/dispatch", r#"{"text":"x","repository":"notes","mac":"nobody"}"#).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = send(&app, Method::POST, "/api/sessions/dispatch", r#"{"text":"x","repository":"notes","on_your_words":"y"}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(seen.lock().unwrap().is_empty());
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
async fn unsigned_is_400_and_fleet_down_says_what_to_do() {
    let (app, _) = rig().await;
    for (method, path) in [
        (Method::GET, "/api/sessions"),
        (Method::POST, "/api/sessions/answer"),
        (Method::POST, "/api/sessions"),
        (Method::GET, "/api/needs"),
        (Method::GET, "/api/repositories"),
    ] {
        assert_eq!(call(&app.rig.app, method, path, &[], "{}").await.0, StatusCode::BAD_REQUEST, "{path}");
    }
    let closed = {
        let listener = crate::bind(0).await.unwrap();
        listener.local_addr().unwrap().port()
    };
    let down = on(closed).await;
    for (method, path, body) in [
        (Method::GET, "/api/sessions", ""),
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

#[tokio::test]
async fn a_live_job_is_forked_at_its_agents_last_message_and_not_at_the_last_row() {
    let (app, seen) = rig().await;
    let (status, body) = send(&app, Method::POST, "/api/sessions/fork", r#"{"session_id":"live1"}"#).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert!(body.contains(r#""id":"s9""#) && body.contains(r#""repository":"armada""#), "{body}");
    assert!(!body.contains(SECRET), "{body}");
    let seen = seen.lock().unwrap();
    assert_eq!(
        *seen,
        [("/sessions/start".to_string(), r#"{"manifest_id":"m1","fork":{"session_id":"live1","at":"a-2"},"project":"shop"}"#.to_string())],
        "one start, with no message after it"
    );
}

#[tokio::test]
async fn an_ended_job_is_forked_whole() {
    let (app, seen) = rig().await;
    let (status, body) = send(&app, Method::POST, "/api/sessions/fork", r#"{"session_id":"done1"}"#).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(
        *seen.lock().unwrap(),
        [("/sessions/start".to_string(), r#"{"manifest_id":"m1","fork":{"session_id":"done1"},"project":"shop"}"#.to_string())]
    );
}

#[tokio::test]
async fn a_fork_that_cannot_be_made_is_refused_before_fleet_starts_anything() {
    let (app, seen) = rig().await;
    let cases = [
        (r#"{"session_id":"nope"}"#, StatusCode::NOT_FOUND, "That Job is not here."),
        (r#"{"session_id":"../h1"}"#, StatusCode::NOT_FOUND, "That Job is not here."),
        (r#"{"session_id":"live1&x=1"}"#, StatusCode::NOT_FOUND, "That Job is not here."),
        (r#"{"session_id":""}"#, StatusCode::NOT_FOUND, "That Job is not here."),
        (
            r#"{"session_id":"bare1"}"#,
            StatusCode::CONFLICT,
            "That Job has no repository, so Armada cannot copy it. Fork it in Armada on your Mac.",
        ),
        (
            r#"{"session_id":"quiet1"}"#,
            StatusCode::CONFLICT,
            "That Job has not answered yet, so there is nothing to copy. Try again once it has.",
        ),
        (r#"{"session_id":"live1","at":"a-1"}"#, StatusCode::BAD_REQUEST, "Armada could not read that from this phone. Update the app."),
    ];
    for (body, status, sentence) in cases {
        assert_eq!(send(&app, Method::POST, "/api/sessions/fork", body).await, (status, sentence.to_string()), "{body}");
    }
    assert!(seen.lock().unwrap().is_empty());
}

fn phone_row(tail: &str, usage: &str) -> String {
    let record = record("j1", "bridge", "live", tail).replace(r#""usage":{}"#, &format!(r#""usage":{usage}"#));
    let record = ipc::decode::<ipc::SessionRecord>("a job", record.as_bytes()).unwrap();
    ipc::encode(&crate::phone_sessions::PhoneSession::of(&record, Some("armada"), FIVE_PAST)).unwrap()
}

#[test]
fn a_job_held_on_its_start_card_is_neither_waiting_nor_working() {
    // A turn is running and the delivery is owed, so only the hold keeps it from waiting and working.
    let job = |start: &str| {
        let hosted = format!(r#","hosted":{{"turn":{{"state":"working"}},"mode":"auto","running":true{start}}}"#);
        phone_row(&format!(r#"{hosted},"standing":"working"{}"#, owing(&[owed("delivery", "delivery", "02:00")])), "{}")
    };
    let held = job(r#","start":{"asks_delivery":false}"#);
    assert!(held.contains(r#""held":true"#), "{held}");
    assert!(held.contains(r#""working":false"#) && held.contains(r#""waiting":false"#), "{held}");
    assert!(!held.contains("waiting_since"), "{held}");
    // Begun, the same Job is working and owes what it owes, and is no longer held.
    let begun = job("");
    assert!(begun.contains(r#""held":false"#) && begun.contains(r#""working":true"#) && begun.contains(r#""waiting":true"#), "{begun}");
}

#[test]
fn the_job_that_started_it_and_its_context_figures_are_named_and_left_out_when_absent() {
    let by = r#","started":{"by":{"id":"p0","title":"Parent job"}}"#;
    let row = phone_row(by, r#"{"context_tokens":120000,"context_window":1000000,"cost_micros":77}"#);
    assert!(row.contains(r#""from":{"id":"p0","title":"Parent job"}"#), "{row}");
    assert!(row.contains(r#""context_tokens":120000"#) && row.contains(r#""context_window":1000000"#), "{row}");
    assert!(!row.contains("cost") && !row.contains("77"), "{row}");

    let plain = phone_row("", "{}");
    for field in ["from", "context_tokens", "context_window"] {
        assert!(!plain.contains(&format!("\"{field}\"")), "{field} in {plain}");
    }
}

#[test]
fn a_live_job_that_said_it_is_done_carries_the_claim_and_its_summary() {
    let said = phone_row(r#","done_since":"2025-01-01T01:00:00Z","done_summary":"The pool shrinks to four.""#, "{}");
    assert!(said.contains(r#""done_since":"2025-01-01T01:00:00Z""#), "{said}");
    assert!(said.contains(r#""done_summary":"The pool shrinks to four.""#), "{said}");

    let plain = phone_row("", "{}");
    assert!(!plain.contains("done_since") && !plain.contains("done_summary"), "{plain}");
}

/// A hosted Job whose turn is `turn`, with `facts` added inside its hosted facts and `tail` after them.
fn hosted_row(turn: &str, facts: &str, tail: &str) -> String {
    let hosted = format!(r#","hosted":{{"turn":{{"state":"{turn}"}},"mode":"auto","running":true{facts}}}"#);
    phone_row(&format!("{hosted}{tail}"), "{}")
}

#[test]
fn every_kind_of_ask_holds_a_job_at_work_as_waiting_and_a_slot_being_made_does_not() {
    let working = |item: String| hosted_row("working", "", &format!(r#","standing":"working"{}"#, owing(&[item])));
    for source in ["plan", "task_done", "sketch", "document", "delivery", "slot_choice"] {
        let row = working(owed(source, source, "02:00"));
        assert!(row.contains(r#""waiting":true"#), "{source} in {row}");
        assert!(row.contains(r#""waiting_since":"2026-10-08T02:00:00.000Z""#), "{source} in {row}");
    }
    // Fleet is at it: nothing here needs him yet.
    let making = working(owed("slot", "slot_choice", "02:00").replace(r#""source""#, r#""creating_since":"2026-10-08T02:01:00.000Z","source""#));
    assert!(making.contains(r#""waiting":false"#), "{making}");
    // Another item waits until the turn ends.
    let agent = working(owed("a1", "agent", "02:00"));
    assert!(agent.contains(r#""waiting":false"#), "{agent}");
    let ended = hosted_row("idle", "", &owing(&[owed("a1", "agent", "02:00")]));
    assert!(ended.contains(r#""waiting":true"#), "{ended}");
}

#[test]
fn a_job_parked_for_any_reason_asks_nothing_of_him_and_is_not_working() {
    // Each also owes a plan and has a turn running, so only the parking keeps it from waiting and working.
    let owes = format!(r#","standing":"working"{}"#, owing(&[owed("plan:1", "plan", "02:00")]));
    let wait = r#","waits":{"on":{"id":"p0","title":"Parent"},"condition":{"kind":"ends"},"base":"main","brief":"B","by":"you","held":false}"#;
    let cases = [
        ("paused", hosted_row("working", r#","paused":{"at":"2026-10-08T03:00:00.000Z","resuming":false}"#, &owes)),
        ("start_failed", hosted_row("working", r#","start_failed":{"cause":"no such program"}"#, &owes)),
        ("starts_when", hosted_row("working", "", &format!("{owes}{wait}"))),
        ("limited_until", hosted_row("working", "", &format!(r#"{owes},"held_until":"2026-10-08T09:00:00.000Z""#))),
    ];
    for (name, row) in cases {
        assert!(row.contains(r#""waiting":false"#) && row.contains(r#""working":false"#), "{name} in {row}");
        assert!(!row.contains("waiting_since"), "{name} in {row}");
    }
}

#[test]
fn a_stopped_job_asks_nothing_of_him_and_says_why_without_what_its_process_said() {
    let stop = r#","standing":"stopped","stopped":{"reason":"process_gone","at":"2026-10-08T03:00:00.000Z","said":"PROCESS-SAID-3","tasks":["TASK-OPEN-3"],"restart":"failed","repair":"failed","gave_up":true}"#;
    let row = hosted_row("idle", "", &format!("{stop}{}", owing(&[owed("plan:1", "plan", "02:00")])));
    assert!(row.contains(r#""waiting":false"#) && row.contains(r#""working":false"#), "{row}");
    assert!(
        row.contains(r#""stopped":{"reason":"process_gone","restart":"failed","repair":"failed","gave_up":true}"#),
        "{row}"
    );
    assert!(!row.contains("PROCESS-SAID-3") && !row.contains("TASK-OPEN-3"), "{row}");
    assert!(!hosted_row("idle", "", "").contains(r#""stopped""#));
}

#[test]
fn a_row_names_its_first_open_pull_request_on_failed_checks() {
    let pr = |target: &str, state: &str, detail: &str| {
        format!(
            r#"{{"kind":"pr","target":"{target}","state":"{state}","detail":{{{detail}}},"since":"2026-10-08T02:00:00.000Z","changed_at":"2026-10-08T02:00:00.000Z"}}"#
        )
    };
    let row = |attached: &[String]| {
        let record = record("j1", "bridge", "live", "").replace(r#""attachments":[]"#, &format!(r#""attachments":[{}]"#, attached.join(",")));
        let record = ipc::decode::<ipc::SessionRecord>("a job", record.as_bytes()).unwrap();
        ipc::encode(&crate::phone_sessions::PhoneSession::of(&record, Some("armada"), FIVE_PAST)).unwrap()
    };
    let failed = r#""checks":"failed""#;
    // Merged, spent and given back no longer count; passing Checks never did.
    let failing = row(&[
        pr("40", "standing", r#""checks":"passed""#),
        pr("41", "spent", failed),
        pr("42", "given_back", failed),
        pr("43", "standing", &format!(r#""state":"merged",{failed}"#)),
        pr("44", "standing", failed),
        pr("45", "standing", failed),
    ]);
    assert!(failing.contains(r#""failing_pull":44"#), "{failing}");
    let passing = row(&[pr("40", "standing", r#""checks":"passed""#)]);
    assert!(!passing.contains("failing_pull"), "{passing}");
}

#[test]
fn a_row_says_a_job_is_blank_only_with_no_title_no_turn_and_nothing_attached() {
    let plain = hosted_row("idle", "", "");
    assert!(!plain.contains(r#""blank""#), "{plain}");
    let record = record("j1", "bridge", "live", r#","hosted":{"turn":{"state":"idle"},"mode":"auto","running":true}"#);
    let mut record = ipc::decode::<ipc::SessionRecord>("a job", record.as_bytes()).unwrap();
    record.title = None;
    record.last_turn_at = None;
    record.attachments.clear();
    let blank = ipc::encode(&crate::phone_sessions::PhoneSession::of(&record, Some("armada"), FIVE_PAST)).unwrap();
    assert!(blank.contains(r#""blank":true"#), "{blank}");
}

#[test]
fn a_row_says_the_proposer_is_reading_the_request() {
    let reading = hosted_row("idle", "", r#","proposal":{"reading":true}"#);
    assert!(reading.contains(r#""reading":true"#), "{reading}");
    let read = hosted_row("idle", "", r#","proposal":{"reading":false}"#);
    assert!(!read.contains(r#""reading""#), "{read}");
}

#[test]
fn what_holds_a_job_is_carried_as_facts_and_left_out_when_absent() {
    let plain = hosted_row("idle", "", "");
    for field in ["standing", "limited_until", "held_open_by", "starts_when", "start_failed", "stopped", "failing_pull", "reading"] {
        assert!(!plain.contains(&format!("\"{field}\"")), "{field} in {plain}");
    }
    assert!(plain.contains(r#""paused":false"#), "{plain}");

    let held = hosted_row(
        "idle",
        r#","paused":{"at":"2026-10-08T03:00:00.000Z","resuming":true},"start_failed":{"cause":"no such program","log":["LOG-LINE-9"]}"#,
        &format!(
            r#","standing":"idle","held_until":"2026-10-08T09:00:00.000Z","held_open_by":[{{"kind":"watch","name":"QA deploy"}},{{"kind":"pull_request","number":7,"on":{{"kind":"queue"}}}},{{"kind":"pull_request","number":8,"on":{{"kind":"checks","auto_merge":true}}}},{{"kind":"pull_request","number":9,"on":{{"kind":"auto_merge"}}}}],"waits":{{"on":{{"id":"p0","title":"Parent"}},"condition":{{"kind":"task_done","task":"t1"}},"base":"main","brief":"BRIEF-SECRET","by":"agent","held":false,"pull":{{"number":4,"branch":"b"}},"part_title":"Build it"}}"#
        ),
    );
    assert!(held.contains(r#""standing":"idle""#) && held.contains(r#""paused":true"#), "{held}");
    assert!(held.contains(r#""limited_until":"2026-10-08T09:00:00.000Z""#), "{held}");
    assert!(held.contains(r#""start_failed":"no such program""#) && !held.contains("LOG-LINE-9"), "{held}");
    assert!(
        held.contains(r#""held_open_by":[{"kind":"watch","name":"QA deploy"},{"kind":"pull_request","number":7,"on":"queue"},{"kind":"pull_request","number":8,"on":"checks","auto_merge":true},{"kind":"pull_request","number":9,"on":"auto_merge"}]"#),
        "{held}"
    );
    assert!(
        held.contains(r#""starts_when":{"on":{"id":"p0","title":"Parent"},"when":"plan","part_title":"Build it","pull_number":4}"#),
        "{held}"
    );
    assert!(!held.contains("BRIEF-SECRET"), "{held}");
}
