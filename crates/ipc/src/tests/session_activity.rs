//! What a hosted Session is doing, on the row. `docs/practices/protocol.md`, *What a Session is
//! doing, from its tool calls*.

use crate::{
    decode, encode, Instant, SessionAccount, SessionActivity, SessionId, SessionList, SessionLive, SessionOrigin,
    SessionRecord, SessionState, SessionTodo, SessionTodoState, SessionUsage,
};

pub(super) fn row(activity: Option<SessionActivity>) -> SessionRecord {
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
        tools_missing: None,
        waiting_for: Vec::new(),
        done_since: None,
        done_summary: None,
        held_until: None,
        context_breakdown: None,
        loadout_summary: None,
        helm: false,
        r#mod: false,
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
        closed: Vec::new(),
        waits: None,
        later: Vec::new(),
        held_open_by: Vec::new(),
        started: None,
        workflow_visits: Vec::new(),
        skipped_steps: Vec::new(),
        workflow_waiting_on: Vec::new(),
        workflow_copy: None,
    }
}

/// **The row carries the two fields under the names a wire field has**, and a reader that
/// does not know them reads a row without them, so the field is additive.
#[test]
fn activity_round_trips_under_the_names_the_wire_gives_it() {
    let activity = SessionActivity {
        now: Some("Editing gate.rs".to_string()),
        todos: vec![
            SessionTodo { text: "Add the field".to_string(), state: SessionTodoState::Done, task: None },
            SessionTodo { text: "Detect in the gate".to_string(), state: SessionTodoState::Doing, task: None },
            SessionTodo { text: "Write the docs".to_string(), state: SessionTodoState::Todo, task: None },
        ],
        said: Some("Reading the gate first".to_string()),
        subagents: Vec::new(),
        ..SessionActivity::default()
    };
    let sent = encode(&row(Some(activity.clone()))).unwrap();
    assert!(sent.contains(r#""now":"Editing gate.rs""#), "{sent}");
    assert!(sent.contains(r#""state":"doing""#), "{sent}");
    assert!(sent.contains(r#""said":"Reading the gate first""#), "{sent}");
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

/// **`for` is the wire's name for the message an account answers**, a Rust keyword the field spells
/// `for_message`. Every field after it is left out where it holds nothing.
#[test]
fn an_account_and_a_live_page_travel_as_the_wire_names_them() {
    let activity = SessionActivity {
        account: Some(SessionAccount {
            for_message: "m7".to_string(),
            done: vec!["Looked through the settings".to_string()],
            now: Some("Making the page".to_string()),
            next: None,
            group: Some("g2".to_string()),
            at: Instant::carried("2026-10-22T09:00:00.000Z"),
        }),
        live: Some(SessionLive { url: "http://localhost:5173".to_string(), title: "The page".to_string() }),
        ..SessionActivity::default()
    };
    let sent = encode(&row(Some(activity.clone()))).unwrap();
    assert!(sent.contains(r#""for":"m7""#), "{sent}");
    assert!(sent.contains(r#""group":"g2""#), "{sent}");
    assert!(!sent.contains("for_message") && !sent.contains(r#""next""#), "{sent}");
    assert!(sent.contains(r#""live":{"url":"http://localhost:5173","title":"The page"}"#), "{sent}");
    let read: SessionRecord = decode("a session", sent.as_bytes()).unwrap();
    assert_eq!(read.activity, Some(activity));
}
