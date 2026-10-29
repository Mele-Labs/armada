//! What an agent is told when it lists Sessions, against a Fleet whose ledgers are long.
//!
//! The case that made these: 9 live Sessions answered 168 KB and every Session ever 696 KB,
//! the door cut both at 64 KiB, and Helm reasoned from the two Sessions it could see.

use std::collections::BTreeMap;

use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use ipc::door::MOST_A_TOOL_MAY_ANSWER;
use serde::Deserialize;
use ipc::{
    Attachment, AttachmentState, Instant, ManifestId, SessionId, SessionList, SessionOrigin,
    SessionRecord, SessionState, SessionUsage,
};
use tower::ServiceExt;

use crate::tests::fake::FakeDaemon;
use crate::tests::shapes::run_id;
use crate::{router, Broadcaster, Served, DOOR_PATH};

fn at() -> Instant {
    Instant::carried("2026-10-13T09:00:00.000Z")
}

fn held(kind: &str, target: String, state: AttachmentState, detail: &[(&str, String)]) -> Attachment {
    Attachment {
        kind: kind.to_string(),
        manifest_id: Some(ManifestId::carried("armada")),
        target,
        state,
        detail: detail.iter().map(|(key, value)| (key.to_string(), value.clone())).collect::<BTreeMap<_, _>>(),
        since: at(),
        changed_at: at(),
    }
}

/// A Session with a long past: `history` pull requests it merged, each with the body the forge
/// gave, and a slot, a branch, one open pull request and a Job it holds now.
fn lived_in(number: usize, history: usize, state: SessionState, title: String) -> SessionRecord {
    let mut attachments = Vec::new();
    for old in 0..history {
        attachments.push(held(
            "pr",
            format!("{}", 1_000 + old),
            AttachmentState::Spent,
            &[
                ("title", format!("An earlier change number {old}")),
                ("state", "merged".to_string()),
                ("body", "What this changes, and why. ".repeat(80)),
                ("address", "https://example.test/pull/1".to_string()),
            ],
        ));
        attachments.push(held("subagent", format!("agent-{old}"), AttachmentState::Spent, &[]));
    }
    attachments.push(held("slot", format!("{}", number % 20), AttachmentState::Standing, &[]));
    attachments.push(held("branch", format!("work/session-{number}"), AttachmentState::Standing, &[]));
    attachments.push(held(
        "pr",
        format!("{}", 2_000 + number),
        AttachmentState::Standing,
        &[
            ("title", format!("Change number {number}")),
            ("state", "open".to_string()),
            ("body", "What this changes, and why. ".repeat(80)),
        ],
    ));
    attachments.push(held("job", format!("01JOB{number:04}"), AttachmentState::Standing, &[]));
    SessionRecord {
        id: SessionId::carried(format!("session-{number}")),
        harness: "a-harness".to_string(),
        origin: SessionOrigin::Terminal,
        manifest_id: Some(ManifestId::carried("armada")),
        cwd: "/work/armada".to_string(),
        title: Some(title),
        state,
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
        tools_missing: None,
        waiting_for: Vec::new(),
        done_since: None,
        standing: None,
        held_until: None,
        context_breakdown: None,
        loadout_summary: None,
        helm: false,
        r#mod: false,
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
        closed: Vec::new(),
        waits: None,
        later: Vec::new(),
        held_open_by: None,
        started: None,
        workflow_visits: Vec::new(),
        skipped_steps: Vec::new(),
    }
}

/// `live` Sessions and `ended` ones, each with `history` merged pull requests behind it.
fn a_fleet_of(live: usize, ended: usize, history: usize) -> (axum::Router, std::sync::Arc<FakeDaemon>) {
    let daemon = std::sync::Arc::new(FakeDaemon::new(Broadcaster::new()));
    {
        let mut held = daemon.sessions_held.lock().expect("sessions held");
        for number in 0..live {
            held.push(lived_in(number, history, SessionState::Live, format!("Session number {number}")));
        }
        for number in live..live + ended {
            held.push(lived_in(number, history, SessionState::Ended, format!("Session number {number}")));
        }
    }
    (router(Served::sharing(daemon.clone(), run_id(), Broadcaster::new())), daemon)
}

async fn body_of(app: &axum::Router, request: Request<Body>) -> String {
    let response = app.clone().oneshot(request).await.expect("the router answers");
    let bytes = response.into_body().collect().await.expect("a readable body").to_bytes();
    String::from_utf8_lossy(&bytes).to_string()
}

/// The three levels of a tool answer this reads, and nothing else of it.
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

async fn listed(app: &axum::Router, arguments: &str) -> String {
    let request = Request::builder()
        .method("POST")
        .uri(DOOR_PATH)
        .header("content-type", "application/json")
        .body(Body::from(format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"list_sessions","arguments":{arguments}}}}}"#
        )))
        .expect("a well-formed request");
    let answered = body_of(app, request).await;
    let message: Message = ipc::decode("a tool answer", answered.as_bytes()).expect("JSON-RPC");
    message.result.content.into_iter().next().expect("text content").text
}

/// **The owner's 9 live Sessions, or 50, fit far under the cap**, and the ended ones behind them
/// do not reach the agent at all.
#[tokio::test]
async fn fifty_live_sessions_with_long_ledgers_reach_an_agent_whole() {
    let (app, daemon) = a_fleet_of(50, 20, 30);
    let whole = ipc::encode(&SessionList { sessions: daemon.sessions_held.lock().expect("held").clone() })
        .expect("a list encodes");
    let said = listed(&app, "{}").await;
    assert!(whole.len() > 10 * MOST_A_TOOL_MAY_ANSWER, "the fixture is not as heavy as the real one");
    assert!(said.len() < MOST_A_TOOL_MAY_ANSWER / 3, "{} bytes", said.len());
    assert!(!said.contains("[cut at"), "the agent was handed a cut list");
    for number in 0..50 {
        assert!(said.contains(&format!("\"session-{number}\"")), "session-{number} is missing");
    }
    assert!(!said.contains("session-50\""), "an ended Session was listed unasked");
    assert!(said.contains("\"pull_requests\":[{\"number\":2000,\"title\":\"Change number 0\""));
    assert!(said.contains("\"branch\":\"work/session-0\""));
    assert!(!said.contains("What this changes"), "a pull request's body reached the agent");
    assert_eq!(*daemon.sessions_asked.lock().expect("asked"), [Some(SessionState::Live)]);
}

#[tokio::test]
async fn an_agent_can_ask_for_the_ended_ones_and_cannot_ask_for_the_wide_row() {
    let (app, daemon) = a_fleet_of(3, 2, 5);
    let said = listed(&app, r#"{"state":"ended","view":"full"}"#).await;
    assert!(said.contains("\"session-3\"") && said.contains("\"session-4\"") && !said.contains("\"session-0\""), "{said}");
    assert!(!said.contains("attachments") && !said.contains("\"cwd\""), "the wide row got through: {said}");
    assert_eq!(*daemon.sessions_asked.lock().expect("asked"), [Some(SessionState::Ended)]);
}

/// Bridge and Pocket send no `view`, and are answered every Session with its ledger.
#[tokio::test]
async fn bridges_list_is_the_whole_record_as_it_was() {
    let (app, daemon) = a_fleet_of(3, 2, 5);
    let request = Request::builder().uri("/sessions").body(Body::empty()).expect("a request");
    let answered = body_of(&app, request).await;
    let expected = ipc::encode(&SessionList { sessions: daemon.sessions_held.lock().expect("held").clone() })
        .expect("a list encodes");
    assert_eq!(answered, expected);
    assert_eq!(*daemon.sessions_asked.lock().expect("asked"), [None], "no state is defaulted on Bridge's route");
    let request = Request::builder().uri("/sessions?state=live").body(Body::empty()).expect("a request");
    assert!(body_of(&app, request).await.contains("\"attachments\""));
}

/// Where a list is still too long for the door, the note says which parameters ask for less.
#[tokio::test]
async fn a_list_still_cut_says_how_to_ask_for_less() {
    let (app, _) = a_fleet_of(700, 0, 1);
    let said = listed(&app, "{}").await;
    assert!(said.contains("[cut at"), "{} bytes and not cut", said.len());
    assert!(said.contains("call this tool again with `q` or `state` set"), "{}", &said[said.len() - 400..]);
}
