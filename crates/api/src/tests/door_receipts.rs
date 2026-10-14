//! What an agent is told after it acts on its own Session, against a row that has grown long.
//!
//! The row of a long Session weighs 36 to 62 KB and every answer was pasted into the agent's
//! context for good. Through the door the four acts answer a receipt; Bridge's route, the same
//! handler without the door's mark, answers the row as it was.

use std::collections::BTreeMap;

use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use ipc::{
    Attachment, AttachmentState, Instant, ManifestId, SessionId, SessionOrigin, SessionRecord,
    SessionState, SessionUsage, WaitingItem, WaitingSource, WindowPlace,
};
use serde::Deserialize;
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served, DOOR_PATH};

/// A receipt for a normal call is well under this, whatever the row weighs.
const MOST_A_RECEIPT_MAY_WEIGH: usize = 500;

fn at() -> Instant {
    Instant::carried("2026-10-13T09:00:00.000Z")
}

fn waiting(id: &str, text: &str, source: WaitingSource, place: Option<WindowPlace>) -> WaitingItem {
    WaitingItem {
        id: id.to_string(),
        text: text.to_string(),
        since: at(),
        source,
        act: None,
        options: Vec::new(),
        place,
    }
}

/// A Session with a long past: forty merged pull requests, each carrying the body the forge gave.
fn long_lived() -> SessionRecord {
    let attachments = (0..40)
        .map(|old| Attachment {
            kind: "pr".to_string(),
            manifest_id: Some(ManifestId::carried("armada")),
            target: format!("{}", 1_000 + old),
            state: AttachmentState::Spent,
            detail: BTreeMap::from([
                ("title".to_string(), format!("An earlier change number {old}")),
                ("body".to_string(), "What this changes, and why. ".repeat(80)),
            ]),
            since: at(),
            changed_at: at(),
        })
        .collect();
    SessionRecord {
        id: SessionId::carried("session-1"),
        harness: "a-harness".to_string(),
        origin: SessionOrigin::Terminal,
        manifest_id: Some(ManifestId::carried("armada")),
        cwd: "/work/armada".to_string(),
        title: Some("A long Session".to_string()),
        state: SessionState::Live,
        started_at: at(),
        last_seen_at: at(),
        last_turn_at: Some(at()),
        ended_at: None,
        end_reason: None,
        usage: SessionUsage::default(),
        attachments,
        hosted: None,
        terminal: None,
        mod_out_of_date: false,
        waiting_for: vec![
            waiting("merge", "Approve the pull request", WaitingSource::Agent, None),
            waiting("pick", "Which of the two", WaitingSource::Agent, None),
            waiting("walk:http://localhost:5173/", "Walk the page", WaitingSource::Walk, None),
            waiting(
                "window_ask:7",
                "Is this the right label?",
                WaitingSource::WindowAsk,
                Some(WindowPlace {
                    url: "http://localhost:5173/".to_string(),
                    selector: ".label".to_string(),
                    text: None,
                }),
            ),
        ],
        done_since: Some(at()),
        standing: None,
        held_until: None,
        context_breakdown: None,
        helm: false,
        activity: None,
    }
}

fn a_fleet() -> (axum::Router, SessionRecord) {
    let row = long_lived();
    let daemon = FakeDaemon::new(Broadcaster::new());
    *daemon.session_row.lock().expect("session row") = Some(row.clone());
    (router(Served::by(daemon, run_id(), Broadcaster::new())), row)
}

async fn text_of(response: axum::response::Response) -> String {
    let bytes = response.into_body().collect().await.expect("a readable body").to_bytes();
    String::from_utf8_lossy(&bytes).to_string()
}

#[derive(Deserialize)]
struct Message {
    result: Outcome,
}

#[derive(Deserialize)]
struct Outcome {
    content: Vec<Block>,
}

#[derive(Deserialize)]
struct Block {
    text: String,
}

/// The tool's answer as the agent reads it.
async fn through_the_door(app: &axum::Router, tool: &str, body: &str) -> String {
    let request = Request::builder()
        .method("POST")
        .uri(DOOR_PATH)
        .header("content-type", "application/json")
        .body(Body::from(format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"{tool}","arguments":{{"body":{body}}}}}}}"#
        )))
        .expect("a well-formed request");
    let answered = text_of(app.clone().oneshot(request).await.expect("the router answers")).await;
    let message: Message = ipc::decode("a tool answer", answered.as_bytes()).expect("JSON-RPC");
    message.result.content.into_iter().next().expect("text content").text
}

/// Bridge's route, or any caller the door did not make.
async fn on_the_route(app: &axum::Router, path: &str, body: &str) -> String {
    let request = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("a well-formed request");
    text_of(app.clone().oneshot(request).await.expect("the router answers")).await
}

const SHOW: &str = r#"{"url":"http://localhost:5173/","title":"The label"}"#;
const SKETCH: &str = r#"{"title":" Request flow ","scene":{"nodes":[{"id":"a","kind":"box","x":0,"y":0}],"edges":[]}}"#;
const ASK: &str = r#"{"question":"Is this the right label?","url":"http://localhost:5173/","selector":".label"}"#;
const WAIT: &str = r#"{"items":[{"id":"merge","text":"Approve the pull request"}]}"#;

#[tokio::test]
async fn waiting_for_answers_the_ids_and_texts_it_set() {
    let (app, row) = a_fleet();
    assert!(ipc::encode(&row).expect("a row encodes").len() > 40_000, "the fixture is too light");
    let said = through_the_door(&app, "waiting_for", WAIT).await;
    assert!(said.len() < MOST_A_RECEIPT_MAY_WEIGH, "{} bytes: {said}", said.len());
    assert_eq!(
        said,
        r#"{"session_id":"session-1","waiting_for":[{"id":"merge","text":"Approve the pull request"},{"id":"pick","text":"Which of the two"}]}"#
    );
}

#[tokio::test]
async fn show_window_answers_the_address_it_showed() {
    let (app, _) = a_fleet();
    let said = through_the_door(&app, "show_window", SHOW).await;
    assert!(said.len() < MOST_A_RECEIPT_MAY_WEIGH, "{} bytes: {said}", said.len());
    assert_eq!(
        said,
        r#"{"session_id":"session-1","window":"http://localhost:5173/","title":"The label"}"#
    );
}

#[tokio::test]
async fn show_sketch_answers_the_title_it_kept() {
    let (app, _) = a_fleet();
    let said = through_the_door(&app, "show_sketch", SKETCH).await;
    assert!(said.len() < MOST_A_RECEIPT_MAY_WEIGH, "{} bytes: {said}", said.len());
    assert_eq!(said, r#"{"session_id":"session-1","sketch":"Request flow"}"#);
}

#[tokio::test]
async fn ask_in_window_answers_the_id_of_the_question() {
    let (app, _) = a_fleet();
    let said = through_the_door(&app, "ask_in_window", ASK).await;
    assert!(said.len() < MOST_A_RECEIPT_MAY_WEIGH, "{} bytes: {said}", said.len());
    assert_eq!(said, r#"{"session_id":"session-1","asked":"window_ask:7"}"#);
}

#[tokio::test]
async fn all_done_answers_that_it_is_done() {
    let (app, _) = a_fleet();
    let said = through_the_door(&app, "all_done", "{}").await;
    assert!(said.len() < MOST_A_RECEIPT_MAY_WEIGH, "{} bytes: {said}", said.len());
    assert_eq!(
        said,
        r#"{"session_id":"session-1","done":true,"done_since":"2026-10-13T09:00:00.000Z"}"#
    );
}

/// Bridge, the mod's hook and anything else that is not the door read the row whole.
#[tokio::test]
async fn off_the_door_each_act_answers_the_whole_row_as_it_did() {
    let (app, row) = a_fleet();
    let whole = ipc::encode(&row).expect("a row encodes");
    for (path, body) in [
        ("/sessions/window", SHOW),
        ("/sessions/window/ask", ASK),
        ("/sessions/waiting", WAIT),
        ("/sessions/done", "{}"),
    ] {
        assert_eq!(on_the_route(&app, path, body).await, whole, "{path}");
    }
}
