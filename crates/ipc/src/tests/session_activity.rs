//! What a hosted Session is doing, on the row. `docs/practices/protocol.md`, *What a Session is
//! doing, from its tool calls*.

use crate::{
    decode, encode, Instant, SessionActivity, SessionId, SessionList, SessionOrigin, SessionRecord, SessionState,
    SessionTodo, SessionTodoState, SessionUsage,
};

fn row(activity: Option<SessionActivity>) -> SessionRecord {
    SessionRecord {
        id: SessionId::carried("s1"),
        harness: "a-harness".to_string(),
        origin: SessionOrigin::Bridge,
        manifest_id: None,
        cwd: "/work".to_string(),
        title: None,
        state: SessionState::Live,
        started_at: Instant::carried("2026-10-14T09:00:00.000Z"),
        last_seen_at: Instant::carried("2026-10-14T09:00:00.000Z"),
        last_turn_at: None,
        ended_at: None,
        end_reason: None,
        usage: SessionUsage::default(),
        attachments: Vec::new(),
        hosted: None,
        terminal: None,
        mod_out_of_date: false,
        waiting_for: Vec::new(),
        done_since: None,
        held_until: None,
        context_breakdown: None,
        loadout_summary: None,
        helm: false,
        auto_land: false,
        standing: None,
        activity,
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

/// **The row carries the two fields under the names a wire field has**, and a reader that
/// does not know them reads a row without them, so the field is additive.
#[test]
fn activity_round_trips_under_the_names_the_wire_gives_it() {
    let activity = SessionActivity {
        now: Some("Editing gate.rs".to_string()),
        todos: vec![
            SessionTodo { text: "Add the field".to_string(), state: SessionTodoState::Done },
            SessionTodo { text: "Detect in the gate".to_string(), state: SessionTodoState::Doing },
            SessionTodo { text: "Write the docs".to_string(), state: SessionTodoState::Todo },
        ],
        subagents: Vec::new(),
    };
    let sent = encode(&row(Some(activity.clone()))).unwrap();
    assert!(sent.contains(r#""now":"Editing gate.rs""#), "{sent}");
    assert!(sent.contains(r#""state":"doing""#), "{sent}");
    let read: SessionRecord = decode("a session", sent.as_bytes()).unwrap();
    assert_eq!(read.activity, Some(activity));
}

#[test]
fn what_is_absent_is_left_out_and_reads_back_absent() {
    let bare = encode(&row(None)).unwrap();
    assert!(!bare.contains("activity"), "no activity, no key: {bare}");
    let read: SessionRecord = decode("a session", bare.as_bytes()).unwrap();
    assert_eq!(read.activity, None);

    let only_now = SessionActivity { now: Some("Searching".to_string()), ..SessionActivity::default() };
    let sent = encode(&row(Some(only_now.clone()))).unwrap();
    assert!(!sent.contains("todos"), "an empty list is left out: {sent}");
    let list: SessionList = decode("sessions", encode(&SessionList { sessions: vec![row(Some(only_now.clone()))] }).unwrap().as_bytes()).unwrap();
    assert_eq!(list.sessions[0].activity, Some(only_now));
}
