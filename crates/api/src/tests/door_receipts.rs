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
const MOST_A_RECEIPT_MAY_WEIGH: usize = 600;

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
        steps: Vec::new(),
        place,
        concern: None,
        cites: Vec::new(),
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
        loadout_summary: None,
        helm: false,
        auto_land: false,
        activity: None,
        build: None,
        workflow: None,
        proposal: None,
        plan: None,
        touches: Vec::new(),
        workflow_step: None,
        judging: false,
        judge: None,
        settled: Vec::new(),
        waits: None,
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
const WITHDRAW: &str = r#"{"url":"http://localhost:5173/"}"#;

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
async fn withdraw_window_answers_the_address_it_took_back() {
    let (app, _) = a_fleet();
    let said = through_the_door(&app, "withdraw_window", WITHDRAW).await;
    assert!(said.len() < MOST_A_RECEIPT_MAY_WEIGH, "{} bytes: {said}", said.len());
    assert_eq!(said, r#"{"session_id":"session-1","withdrawn":"http://localhost:5173/"}"#);
}

#[tokio::test]
async fn show_sketch_answers_the_title_it_kept() {
    let (app, _) = a_fleet();
    let said = through_the_door(&app, "show_sketch", SKETCH).await;
    assert!(said.len() < MOST_A_RECEIPT_MAY_WEIGH, "{} bytes: {said}", said.len());
    assert_eq!(said, r#"{"session_id":"session-1","sketch":"Request flow"}"#);
}

const PLAN: &str = r#"{"groups":[{"title":"Rotate","agent":"rust-engineer","paths":["crates/store"],"tasks":[{"title":"Rotate at midnight"}]}]}"#;

fn pending_plan() -> ipc::SessionPlan {
    ipc::SessionPlan {
        version: 2,
        recorded_at: at(),
        approved: false,
        approved_at: None,
        scope: vec!["crates/store".to_string()],
        groups: vec![ipc::SessionPlanGroup {
            id: "g1".to_string(),
            agent: "rust-engineer".to_string(),
            title: "Rotate".to_string(),
            paths: vec!["crates/store".to_string()],
            issue: None,
            tasks: vec![ipc::SessionPlanTask {
                id: "t1".to_string(),
                text: "Rotate at midnight".to_string(),
                state: ipc::PlanTaskState::Open,
                seen: false,
            }],
            worked_at: None,
        }],
        changes: None,
        sketch: None,
        held_by: Vec::new(),
    }
}

#[tokio::test]
async fn record_plan_answers_the_version_and_each_id_fleet_gave() {
    let called = waiting("plan:2", "Plan to approve", WaitingSource::Plan, None);
    let daemon = FakeDaemon::new(Broadcaster::new());
    *daemon.session_row.lock().expect("session row") = Some(SessionRecord { plan: Some(pending_plan()), waiting_for: vec![called], ..long_lived() });
    let app = router(Served::by(daemon, run_id(), Broadcaster::new()));
    let said = through_the_door(&app, "record_plan", PLAN).await;
    assert!(said.len() < MOST_A_RECEIPT_MAY_WEIGH, "{} bytes: {said}", said.len());
    assert_eq!(
        said,
        r#"{"session_id":"session-1","plan":{"version":2,"task_list":"Your task list is this plan: TaskCreate each task word for word, and TaskUpdate it as you work.","groups":[{"id":"g1","title":"Rotate","tasks":[{"id":"t1","title":"Rotate at midnight"}]}]}}"#
    );
}

#[tokio::test]
async fn record_plan_says_so_when_the_plan_is_not_put_to_the_owner_yet() {
    let daemon = FakeDaemon::new(Broadcaster::new());
    *daemon.session_row.lock().expect("session row") =
        Some(SessionRecord { plan: Some(pending_plan()), waiting_for: Vec::new(), ..long_lived() });
    let app = router(Served::by(daemon, run_id(), Broadcaster::new()));
    let said = through_the_door(&app, "record_plan", PLAN).await;
    assert!(
        said.contains(r#""held":"The owner is not asked to approve this plan until he has approved every page you showed. A page you changed after showing it waits until you show it again with show_window.""#),
        "{said}"
    );
}

#[tokio::test]
async fn record_plan_names_each_window_that_holds_it_and_what_settles_it() {
    let daemon = FakeDaemon::new(Broadcaster::new());
    let held_by = vec![
        ipc::PlanHold { url: "http://localhost:5173/one".into(), why: ipc::PlanHoldWhy::Unapproved },
        ipc::PlanHold { url: "http://localhost:5173/two".into(), why: ipc::PlanHoldWhy::Changed },
    ];
    let plan = ipc::SessionPlan { held_by, ..pending_plan() };
    *daemon.session_row.lock().expect("session row") =
        Some(SessionRecord { plan: Some(plan), waiting_for: Vec::new(), ..long_lived() });
    let app = router(Served::by(daemon, run_id(), Broadcaster::new()));
    let said = through_the_door(&app, "record_plan", PLAN).await;
    assert!(
        said.contains(" http://localhost:5173/one: not approved since you last showed it. It is in his Needs you list, and his Approve settles it. A walk he has moved past is taken back with withdraw_window. http://localhost:5173/two: changed since you last showed it. Show it again with show_window when it is ready, and his Approve settles it.\""),
        "{said}"
    );
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
        ("/sessions/withdraw_window", WITHDRAW),
        ("/sessions/window/ask", ASK),
        ("/sessions/waiting", WAIT),
        ("/sessions/done", "{}"),
    ] {
        assert_eq!(on_the_route(&app, path, body).await, whole, "{path}");
    }
}
