//! The claim: **a subagent the agent waits on is drawn on its own `Agent` row**, running with its
//! latest call and then finished or cut off, and what the person writes while it holds the turn is
//! marked held until the agent reads it. armada-d3h.1.
//!
//! Driven by stream lines sent as the reader sends them, and the mod's reports, read off the thread.

use std::time::Duration;

use adapter_traits::AgentHarness;
use adapters::HeadlessAgent;
use api::Sessions;
use ipc::{
    AttachmentNamed, AttachmentReport, AttachmentState, SessionFact, SessionId, SessionReport, SessionRow, ToolSubagent,
    ToolSubagentState,
};

use super::session_host::{eventually, rig_placing, Rig};
use super::session_plan::running;
use crate::session_host::Heard;

const FOREGROUND: &str = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_50","name":"Agent","input":{"subagent_type":"bridge-engineer","description":"Make the thread resizable"}}]}}"#;
const BACKGROUND: &str = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_50","name":"Agent","input":{"subagent_type":"bridge-engineer","description":"Make the thread resizable","run_in_background":true}}]}}"#;
const INSIDE: &str = r#"{"type":"assistant","parent_tool_use_id":"toolu_50","message":{"content":[{"type":"tool_use","id":"toolu_51","name":"Bash","input":{"command":"pnpm vitest run thread"}}]}}"#;
const RETURNED: &str = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_50"}]}}"#;

/// One stream line, sent to Fleet the way the hosted reader sends it.
fn line(rig: &Rig, text: &str) {
    let spawned = adapters::subagent_calls(text);
    if !spawned.is_empty() {
        rig.stand_in.hears(0, Heard::Spawned(spawned));
    }
    let events = HeadlessAgent::at("agent").read(text);
    let heard = match adapters::inside_a_subagent(text) {
        true => Heard::Subagent { events, call: adapters::subagent_parent(text) },
        false => Heard::Events(events),
    };
    rig.stand_in.hears(0, heard);
}

async fn told(rig: &Rig, id: &SessionId, fact: SessionFact) {
    let report = SessionReport { harness: adapters::HOSTED_HARNESS.into(), session_id: id.clone(), fact };
    rig.fleet.report_session(report).await.expect("kept");
}

fn attached(target: &str) -> SessionFact {
    let detail = [("type", "bridge-engineer"), ("description", "Make the thread resizable"), ("call", "toolu_50")];
    let detail = detail.map(|(key, value)| (key.to_string(), value.to_string())).into();
    SessionFact::Attached { attachment: AttachmentReport { kind: "subagent".into(), target: target.into(), detail } }
}

fn settled(target: &str, state: AttachmentState) -> SessionFact {
    SessionFact::Settled { attachment: AttachmentNamed { kind: "subagent".into(), target: target.into() }, state }
}

/// `read` once `done` holds of it, or as it stands after five seconds, so a failing assert shows it.
async fn once<T, F: std::future::Future<Output = T>>(mut read: impl FnMut() -> F, done: impl Fn(&T) -> bool) -> T {
    for _ in 0..500 {
        let now = read().await;
        if done(&now) {
            return now;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    read().await
}

/// The thread's `Agent` rows, each with its id and subagent.
async fn agent_rows(rig: &Rig, id: &SessionId) -> Vec<(String, Option<ToolSubagent>)> {
    rig.rows(id)
        .await
        .into_iter()
        .filter_map(|row| match row {
            SessionRow::Tool { id, text, subagent, .. } if text.starts_with("Agent") => Some((id, subagent)),
            _ => None,
        })
        .collect()
}

/// The person's rows saying `text`, each with whether it is held.
async fn queued(rig: &Rig, id: &SessionId, text: &str) -> Vec<bool> {
    rig.rows(id)
        .await
        .into_iter()
        .filter_map(|row| match row {
            SessionRow::Message { text: said, queued, .. } if said == text => Some(queued),
            _ => None,
        })
        .collect()
}

/// A foreground subagent at work behind its `Agent` row, on the attachment `a7`: the row it stands on.
async fn working(rig: &Rig, id: &SessionId) -> String {
    line(rig, FOREGROUND);
    eventually(|| async { agent_rows(rig, id).await.len() == 1 }).await;
    told(rig, id, attached("a7")).await;
    line(rig, INSIDE);
    // A latest call within a second of the last put is put on a trailing timer.
    let rows = once(
        || agent_rows(rig, id),
        |rows| rows.first().and_then(|(_, subagent)| subagent.as_ref()?.latest.as_ref()).is_some(),
    )
    .await;
    let [(row, Some(subagent))] = &rows[..] else { panic!("one Agent row carrying its subagent: {rows:?}") };
    assert_eq!(subagent.state, ToolSubagentState::Running);
    assert_eq!(subagent.agent_type.as_deref(), Some("bridge-engineer"));
    assert_eq!(subagent.task, "Make the thread resizable");
    assert_eq!(subagent.latest.as_deref(), Some("Bash pnpm vitest run thread"), "the subagent's own call, not a row of its own");
    assert!(subagent.ended.is_none());
    row.clone()
}

#[tokio::test]
async fn a_foreground_subagent_runs_on_its_agent_row_and_is_cut_off_there_when_given_back() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    let row = working(&rig, &id).await;

    told(&rig, &id, settled("a7", AttachmentState::GivenBack)).await;
    let rows = agent_rows(&rig, &id).await;
    let [(settled_row, Some(subagent))] = &rows[..] else { panic!("still one Agent row: {rows:?}") };
    assert_eq!(settled_row, &row, "the same row, put again");
    assert_eq!(subagent.state, ToolSubagentState::CutOff);
    assert!(subagent.ended.is_some());
}

#[tokio::test]
async fn a_spent_subagent_is_finished_on_its_agent_row() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    let row = working(&rig, &id).await;

    told(&rig, &id, settled("a7", AttachmentState::Spent)).await;
    let rows = agent_rows(&rig, &id).await;
    let [(settled_row, Some(subagent))] = &rows[..] else { panic!("still one Agent row: {rows:?}") };
    assert_eq!(settled_row, &row);
    assert_eq!(subagent.state, ToolSubagentState::Finished);
    assert!(subagent.ended.is_some());
}

#[tokio::test]
async fn words_sent_while_a_foreground_agent_call_holds_the_turn_are_queued_until_it_returns() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    line(&rig, FOREGROUND);
    eventually(|| async { agent_rows(&rig, &id).await.len() == 1 }).await;

    rig.send(&id, "Hello?").await;
    assert_eq!(queued(&rig, &id, "Hello?").await, [true], "held by the CLI, and shown so");

    line(&rig, RETURNED);
    let read = once(|| queued(&rig, &id, "Hello?"), |rows| rows == &[false]).await;
    assert_eq!(read, [false], "read by the agent once its call returned");
}

#[tokio::test]
async fn words_sent_while_a_background_agent_call_runs_are_not_queued() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    line(&rig, BACKGROUND);
    eventually(|| async { agent_rows(&rig, &id).await.len() == 1 }).await;

    rig.send(&id, "Hello?").await;
    assert_eq!(queued(&rig, &id, "Hello?").await, [false], "a background call returns at once and holds nothing");
}
