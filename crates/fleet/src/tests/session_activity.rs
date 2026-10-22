//! What a hosted session is doing, read off the tool calls its gate sees: the agent's own list kept as
//! it said it, one short fact for the latest call, and a row published for a change at most once in a
//! while. `docs/concepts/session.md`, *Working on*.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{DroneEvent, Speaker};
use api::{HostedSessions, Next, Sessions, Subscription};
use ipc::{CloseSession, Event, SessionActivity, SessionGate, SessionId, SessionRecord, SessionTodo, SessionTodoState, SubagentActivity};

use super::session_host::{eventually, rig_placing, Rig};
use crate::session_host::activity::now_of;

fn call(tool: &str, input: &str) -> SessionGate {
    let call = format!(r#"{{"session_id":"s","cwd":"/work","tool_name":"{tool}","tool_input":{input}}}"#);
    ipc::decode("a gate", call.as_bytes()).expect("a gate")
}

fn now(tool: &str, input: &str) -> Option<String> {
    now_of(&call(tool, input))
}

async fn row(rig: &Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session
}

async fn activity(rig: &Rig, id: &SessionId) -> Option<SessionActivity> {
    row(rig, id).await.activity
}

/// The rows of `id` published until the stream has been quiet for `quiet`.
async fn published(watching: &mut Subscription, id: &SessionId, quiet: Duration) -> Vec<SessionRecord> {
    let mut seen = Vec::new();
    while let Ok(Some(Next::Send(delivered))) = tokio::time::timeout(quiet, watching.next()).await {
        if let Event::SessionChanged(record) = delivered.event {
            if record.id == *id {
                seen.push(record);
            }
        }
    }
    seen
}

#[test]
fn a_file_tool_is_editing_or_reading_the_file_by_its_name() {
    assert_eq!(now("Edit", r#"{"file_path":"/work/crates/fleet/src/gate.rs"}"#).as_deref(), Some("Editing gate.rs"));
    assert_eq!(now("Write", r#"{"file_path":"notes.md"}"#).as_deref(), Some("Editing notes.md"));
    assert_eq!(now("MultiEdit", r#"{"file_path":"/a/b.ts"}"#).as_deref(), Some("Editing b.ts"));
    assert_eq!(now("NotebookEdit", r#"{"notebook_path":"/a/run.ipynb"}"#).as_deref(), Some("Editing run.ipynb"));
    assert_eq!(now("Read", r#"{"file_path":"/work/Cargo.toml"}"#).as_deref(), Some("Reading Cargo.toml"));
    assert_eq!(now("Edit", "{}"), None, "no path, nothing to name");
}

#[test]
fn the_other_named_tools_are_one_word_or_phrase() {
    assert_eq!(now("Grep", r#"{"pattern":"x"}"#).as_deref(), Some("Searching"));
    assert_eq!(now("Glob", r#"{"pattern":"*.rs"}"#).as_deref(), Some("Searching"));
    assert_eq!(now("WebFetch", r#"{"url":"https://example.com"}"#).as_deref(), Some("Fetching"));
    assert_eq!(now("WebSearch", r#"{"query":"x"}"#).as_deref(), Some("Fetching"));
    assert_eq!(now("Agent", r#"{"prompt":"x"}"#).as_deref(), Some("Running a subagent"));
    assert_eq!(now("Task", r#"{"prompt":"x"}"#).as_deref(), Some("Running a subagent"));
}

#[test]
fn a_tool_fleet_does_not_name_changes_nothing() {
    assert_eq!(now("mcp__armada-fleet__list_jobs", "{}"), None);
    assert_eq!(now("ToolSearch", r#"{"query":"x"}"#), None);
    assert_eq!(now("TodoWrite", r#"{"todos":[]}"#), None, "the list is not a fact about now");
}

#[test]
fn a_shell_line_is_its_first_one_or_two_words_after_the_scene_is_set() {
    let bash = |command: &str| {
        let quoted = command.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
        now("Bash", &format!(r#"{{"command":"{quoted}"}}"#))
    };
    assert_eq!(bash("cargo check -p fleet").as_deref(), Some("Running cargo check"));
    assert_eq!(bash("pnpm test").as_deref(), Some("Running pnpm test"));
    assert_eq!(bash("cd crates/fleet && cargo test --lib").as_deref(), Some("Running cargo test"));
    assert_eq!(bash("cd a; cd b\ngit status -s").as_deref(), Some("Running git status"));
    assert_eq!(bash("RUST_LOG=debug FOO=1 cargo build").as_deref(), Some("Running cargo build"));
    assert_eq!(bash("taskpolicy -c utility armada check test x").as_deref(), Some("Running armada check"));
    assert_eq!(bash("time ./scripts/preview --restart").as_deref(), Some("Running preview"));
    assert_eq!(bash("ls -la /tmp").as_deref(), Some("Running ls"), "a plain command keeps its arguments out");
    assert_eq!(bash("cat src/main.rs | head").as_deref(), Some("Running cat"));
    assert_eq!(bash("cargo --version").as_deref(), Some("Running cargo"), "a flag is no subcommand");
    assert_eq!(bash(r#"cd "a && b" && npx vitest run"#).as_deref(), Some("Running npx vitest"));
    assert_eq!(bash("cd crates"), None, "only the scene");
    assert_eq!(bash("$(which x) --flag"), None, "not a plain command name");
}

#[tokio::test]
async fn a_todo_write_replaces_the_list_and_leaves_now_alone() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    assert_eq!(activity(&rig, &id).await, None, "nothing before the first call");

    rig.gate(&id, "Read", r#"{"file_path":"/elsewhere/a.rs"}"#).await;
    rig.gate(
        &id,
        "TodoWrite",
        r#"{"todos":[
            {"content":"Add the field","status":"completed","activeForm":"Adding the field"},
            {"content":"Detect in the gate","status":"in_progress","activeForm":"Detecting in the gate"},
            {"content":"Write the docs","status":"pending","activeForm":"Writing the docs"}]}"#,
    )
    .await;
    let seen = activity(&rig, &id).await.expect("a call was made");
    assert_eq!(seen.now.as_deref(), Some("Reading a.rs"), "a TodoWrite is not what it is doing");
    let todo = |text: &str, state| SessionTodo { text: text.into(), state };
    assert_eq!(
        seen.todos,
        [
            todo("Add the field", SessionTodoState::Done),
            todo("Detect in the gate", SessionTodoState::Doing),
            todo("Write the docs", SessionTodoState::Todo),
        ]
    );

    rig.gate(&id, "TodoWrite", r#"{"todos":[{"content":"Write the docs","status":"in_progress","activeForm":"x"}]}"#).await;
    let replaced = activity(&rig, &id).await.expect("still there").todos;
    assert_eq!(replaced, [todo("Write the docs", SessionTodoState::Doing)], "the whole list is replaced");

    rig.gate(&id, "TodoWrite", r#"{"todos":[]}"#).await;
    assert!(activity(&rig, &id).await.expect("still there").todos.is_empty(), "an empty list clears it");
}

#[tokio::test]
async fn a_shell_call_and_an_edit_set_now() {
    let rig = rig_placing(None);
    let id = rig.start().await;

    rig.gate(&id, "Bash", r#"{"command":"git status --short"}"#).await;
    let first = activity(&rig, &id).await.expect("a call was made");
    assert_eq!(first.now.as_deref(), Some("Running git status"));

    rig.gate(&id, "Edit", r#"{"file_path":"/elsewhere/gate.rs"}"#).await;
    let second = activity(&rig, &id).await.expect("a call was made");
    assert_eq!(second.now.as_deref(), Some("Editing gate.rs"));

    rig.gate(&id, "mcp__somewhere__tool", "{}").await;
    assert_eq!(activity(&rig, &id).await.expect("kept").now.as_deref(), Some("Editing gate.rs"));
}

/// A subagent's calls carry its `agent_id`. They set its own `now` and leave the Session's.
#[tokio::test]
async fn a_subagents_call_sets_its_own_now_and_leaves_the_sessions() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let by = |agent: &str, tool: &str, input: &str| -> SessionGate {
        let call = format!(
            r#"{{"session_id":"{}","cwd":"/work","tool_name":"{tool}","tool_input":{input},"agent_id":"{agent}"}}"#,
            id.as_str()
        );
        ipc::decode("a gate", call.as_bytes()).expect("a gate")
    };

    rig.gate(&id, "Read", r#"{"file_path":"/elsewhere/own.rs"}"#).await;
    Arc::clone(&rig.fleet).gate_session_call(by("sub-1", "Edit", r#"{"file_path":"/elsewhere/gate.rs"}"#)).await.unwrap();
    let seen = activity(&rig, &id).await.expect("calls were made");
    assert_eq!(seen.now.as_deref(), Some("Reading own.rs"), "the Session's own now is left alone");
    assert_eq!(seen.subagents, [SubagentActivity { id: "sub-1".into(), now: "Editing gate.rs".into() }]);

    Arc::clone(&rig.fleet).gate_session_call(by("sub-1", "Grep", r#"{"pattern":"x"}"#)).await.unwrap();
    Arc::clone(&rig.fleet).gate_session_call(by("sub-2", "Read", r#"{"file_path":"/elsewhere/b.rs"}"#)).await.unwrap();
    let both = activity(&rig, &id).await.expect("kept").subagents;
    assert_eq!(
        both,
        [
            SubagentActivity { id: "sub-1".into(), now: "Searching".into() },
            SubagentActivity { id: "sub-2".into(), now: "Reading b.rs".into() },
        ]
    );

    let settled = |agent: &str| ipc::SessionReport {
        harness: String::from(adapters::HOSTED_HARNESS),
        session_id: id.clone(),
        fact: ipc::SessionFact::Settled {
            attachment: ipc::AttachmentNamed { kind: String::from("subagent"), target: agent.into() },
            state: ipc::AttachmentState::Spent,
        },
    };
    rig.fleet.report_session(settled("sub-1")).await.unwrap();
    let left = activity(&rig, &id).await.expect("kept").subagents;
    assert_eq!(left, [SubagentActivity { id: "sub-2".into(), now: "Reading b.rs".into() }], "an ended subagent's line goes");

    rig.fleet.hosts().of(id.as_str()).state().activity.turn_began();
    assert_eq!(activity(&rig, &id).await, None, "a turn beginning clears the subagents' lines with the Session's own");
}

#[tokio::test]
async fn quick_changes_publish_the_first_and_one_trailing_and_a_repeat_publishes_nothing() {
    let rig = rig_placing(None);
    rig.fleet.hosts().publish_activity_every(Duration::from_millis(400));
    let id = rig.start().await;
    let mut watching = rig.fleet.events().subscribe();

    for name in ["a.rs", "b.rs", "c.rs"] {
        rig.gate(&id, "Read", &format!(r#"{{"file_path":"/elsewhere/{name}"}}"#)).await;
    }
    let rows = published(&mut watching, &id, Duration::from_millis(900)).await;
    assert_eq!(rows.len(), 2, "the first change and the trailing one, not a row a call: {rows:?}");
    let last = rows[1].activity.clone().expect("carried");
    assert_eq!(last.now.as_deref(), Some("Reading c.rs"), "the trailing row is the last change");

    rig.gate(&id, "Read", r#"{"file_path":"/elsewhere/c.rs"}"#).await;
    let none = published(&mut watching, &id, Duration::from_millis(900)).await;
    assert!(none.is_empty(), "a call that changes nothing publishes nothing: {none:?}");
}

#[tokio::test]
async fn an_ended_session_has_no_activity_and_a_late_call_records_none() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.gate(&id, "Read", r#"{"file_path":"/elsewhere/a.rs"}"#).await;
    rig.gate(&id, "TodoWrite", r#"{"todos":[{"content":"x","status":"pending","activeForm":"x"}]}"#).await;
    assert!(activity(&rig, &id).await.is_some());

    Arc::clone(&rig.fleet).close_session(CloseSession { session_id: id.clone() }).await.unwrap();
    assert_eq!(activity(&rig, &id).await, None, "nothing is kept of a session that ended");

    rig.gate(&id, "Read", r#"{"file_path":"/elsewhere/b.rs"}"#).await;
    assert_eq!(activity(&rig, &id).await, None, "the gate passes an ended session without noting it");
    assert_eq!(rig.fleet.activity_of(id.as_str()), None, "nor is it kept behind the row");
}

/// **A turn that begins is a new turn.** What the last one was doing goes at once, and the agent's list,
/// which is still true, stays. A turn Fleet's own notice begins clears it the same.
#[tokio::test]
async fn a_turn_starting_clears_now_and_keeps_the_list_and_publishes() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "go").await;
    rig.stand_in.init(0);
    eventually(|| async { rig.turn(&id).await != ipc::SessionTurn::Idle }).await;

    rig.gate(&id, "TodoWrite", r#"{"todos":[{"content":"Write the docs","status":"in_progress","activeForm":"x"}]}"#).await;
    rig.gate(&id, "Read", r#"{"file_path":"/elsewhere/a.rs"}"#).await;
    let busy = activity(&rig, &id).await.expect("a turn's calls");
    assert!(busy.now.is_some());

    for by_fleet in [false, true] {
        rig.stand_in.finishes(0, "done");
        eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;
        rig.gate(&id, "Read", r#"{"file_path":"/elsewhere/b.rs"}"#).await;
        assert!(activity(&rig, &id).await.expect("held").now.is_some());
        rig.send(&id, "again").await;
        if by_fleet {
            rig.fleet.hosts().of(id.as_str()).state().fleet_told += 1;
        }
        let mut watching = rig.fleet.events().subscribe();
        rig.stand_in.init(0);

        let rows = published(&mut watching, &id, Duration::from_millis(500)).await;
        let began = rows.iter().rev().find_map(|one| one.activity.clone()).expect("a row for the turn carries the list");
        assert_eq!(began.now, None, "nothing of the last turn's calls (by fleet: {by_fleet})");
        assert_eq!(began.todos.len(), 1, "the list stays");
        assert_eq!(activity(&rig, &id).await.expect("held").now, None);
    }
}

/// **What the agent just said is on the row as a line**, and the next turn starts without it.
#[tokio::test]
async fn a_turns_text_sets_said_to_its_first_line_and_the_next_turn_clears_it() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.send(&id, "go").await;
    rig.stand_in.init(0);
    eventually(|| async { rig.turn(&id).await != ipc::SessionTurn::Idle }).await;

    let long = "x".repeat(400);
    rig.stand_in.says(
        0,
        vec![DroneEvent::Said { text: format!("\n  Looking at the gate first.  \nThen the store.\n{long}"), by: Speaker::Drone }],
    );
    eventually(|| async { activity(&rig, &id).await.is_some_and(|seen| seen.said.is_some()) }).await;
    let seen = activity(&rig, &id).await.expect("said something");
    assert_eq!(seen.said.as_deref(), Some("Looking at the gate first."), "the first line, trimmed");
    assert_eq!(seen.now, None, "what it said is not what it is doing");

    rig.stand_in.says(0, vec![DroneEvent::Said { text: long.clone(), by: Speaker::Drone }]);
    eventually(|| async { activity(&rig, &id).await.and_then(|seen| seen.said).is_some_and(|said| said.starts_with("xxx")) }).await;
    let cut = activity(&rig, &id).await.and_then(|seen| seen.said).expect("said");
    assert_eq!(cut.chars().count(), 160, "at most 160 characters, ellipsis included");

    rig.stand_in.finishes(0, "done");
    eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;
    assert!(activity(&rig, &id).await.is_some_and(|seen| seen.said.is_some()), "a turn ending clears nothing");
    rig.send(&id, "again").await;
    rig.stand_in.init(0);
    eventually(|| async { activity(&rig, &id).await.and_then(|seen| seen.said).is_none() }).await;
}
