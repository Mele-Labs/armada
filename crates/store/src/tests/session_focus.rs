//! A Job's focus: the person's latest message, and the plan task in progress that began last. Kept for
//! an attachment's version to read when it is written.

use crate::tests::{open, TempDir};
use crate::{KeptFocus, KeptPlan, KeptPlanGroup, KeptPlanTask};

/// A plan of two groups, `g1` with t1 and t2 and `g2` with t3. `states` is each task's `(listed, started_at)`.
fn plan(states: [(&str, Option<&str>); 3]) -> KeptPlan {
    let task = |id: &str, (listed, started_at): (&str, Option<&str>)| KeptPlanTask {
        id: id.to_string(),
        text: format!("Task {id}"),
        listed: Some(listed.to_string()),
        completed_at: None,
        edited: false,
        started_at: started_at.map(str::to_string),
    };
    let group = |id: &str, tasks: Vec<KeptPlanTask>| KeptPlanGroup {
        id: id.to_string(),
        gives: None,
        agent: "rust-engineer".to_string(),
        title: id.to_string(),
        paths: Vec::new(),
        issue: None,
        worked_at: None,
        tasks,
    };
    KeptPlan {
        version: 1,
        summary: String::new(),
        recorded_at: "2026-10-22T09:00:00.000Z".to_string(),
        approved_at: None,
        set_aside_at: None,
        sketch: None,
        walk: None,
        changes: None,
        next_group: 3,
        next_task: 4,
        groups: vec![
            group("g1", vec![task("t1", states[0]), task("t2", states[1])]),
            group("g2", vec![task("t3", states[2])]),
        ],
    }
}

fn task_of(store: &crate::Store) -> Option<String> {
    store.session_focus("s1").unwrap().and_then(|focus| focus.task_id)
}

#[test]
fn the_latest_message_replaces_the_one_before_and_survives_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.session_focus("s1").unwrap(), None);
    store.keep_session_message("s1", "m1").unwrap();
    store.keep_session_message("s1", "m2").unwrap();
    store.keep_session_message("s2", "m9").unwrap();
    drop(store);

    let store = open(&dir);
    assert_eq!(
        store.session_focus("s1").unwrap(),
        Some(KeptFocus { message_id: Some("m2".to_string()), task_id: None })
    );
    assert_eq!(store.session_focus("s2").unwrap().and_then(|focus| focus.message_id), Some("m9".to_string()));
}

#[test]
fn the_task_in_progress_that_began_last_is_the_focus_and_a_finished_one_is_not() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_session_message("s1", "m1").unwrap();

    let mut kept = plan([
        ("in_progress", Some("2026-10-22T09:01:00.000Z")),
        ("in_progress", Some("2026-10-22T09:03:00.000Z")),
        ("completed", None),
    ]);
    store.keep_session_plan("s1", &kept).unwrap();
    assert_eq!(task_of(&store), Some("t2".to_string()), "of two in progress, the later start");
    assert_eq!(
        store.session_focus("s1").unwrap().and_then(|focus| focus.message_id),
        Some("m1".to_string()),
        "a plan written leaves the message"
    );

    kept.groups[0].tasks[1].listed = Some("completed".to_string());
    kept.groups[0].tasks[1].started_at = None;
    store.keep_session_plan_progress("s1", &kept).unwrap();
    assert_eq!(task_of(&store), Some("t1".to_string()), "a completed task does not stay the focus");

    kept.groups[0].tasks[0].listed = Some("completed".to_string());
    kept.groups[0].tasks[0].started_at = None;
    store.keep_session_plan_progress("s1", &kept).unwrap();
    assert_eq!(task_of(&store), None, "none in progress, none kept");
    assert_eq!(store.session_focus("s1").unwrap().and_then(|focus| focus.message_id), Some("m1".to_string()));
}

#[test]
fn two_tasks_that_began_together_are_told_apart_by_their_place_and_one_with_no_time_is_the_oldest() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let at = Some("2026-10-22T09:02:00.000Z");

    store.keep_session_plan("s1", &plan([("in_progress", at), ("pending", None), ("in_progress", at)])).unwrap();
    assert_eq!(task_of(&store), Some("t3".to_string()), "the later in the plan");

    store.keep_session_plan("s1", &plan([("in_progress", None), ("in_progress", at), ("pending", None)])).unwrap();
    assert_eq!(task_of(&store), Some("t2".to_string()), "one with no time is older than any with one");
}

fn artifact(detail: &[(&str, &str)], at: &str) -> crate::KeptAttachment {
    crate::KeptAttachment {
        holder: crate::Holder::session("s1"),
        kind: "sketch".to_string(),
        manifest_id: "armada".to_string(),
        target: "flow".to_string(),
        state: crate::AttachmentState::Standing,
        detail: detail.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect(),
        since: at.to_string(),
        changed_at: at.to_string(),
    }
}

fn origins(store: &crate::Store) -> Vec<(Option<String>, Option<String>)> {
    store
        .attachment_versions(&crate::Holder::session("s1"))
        .unwrap()
        .into_iter()
        .map(|version| (version.from_message, version.from_task))
        .collect()
}

#[test]
fn a_version_reads_the_message_and_task_in_focus_when_it_was_written() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let pending = ("pending", None);
    store.keep_session_message("s1", "m1").unwrap();
    store
        .keep_session_plan("s1", &plan([pending, pending, ("in_progress", Some("2026-10-22T09:01:00.000Z"))]))
        .unwrap();
    store.attach(&artifact(&[], "2026-10-22T09:02:00.000Z"), false).unwrap();

    store.keep_session_message("s1", "m2").unwrap();
    store.keep_session_plan("s1", &plan([pending, pending, ("completed", None)])).unwrap();
    store.attach(&artifact(&[("lines", "12")], "2026-10-22T09:03:00.000Z"), false).unwrap();

    let some = |text: &str| Some(text.to_string());
    assert_eq!(origins(&store), [(some("m1"), some("t3")), (some("m2"), None)]);
}

#[test]
fn a_version_written_before_any_focus_or_by_another_kind_of_holder_reads_nothing() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.attach(&artifact(&[], "2026-10-22T09:00:00.000Z"), false).unwrap();
    assert_eq!(origins(&store), [(None, None)], "no focus yet");

    store.keep_session_message("s1", "m1").unwrap();
    store
        .conn
        .execute(
            "INSERT INTO ledger_attachments (holder_kind, holder_id, kind, manifest_id, target, state, detail, since, changed_at)
             VALUES ('autopilot', 's1', 'sketch', '', 'flow', 'standing', '{}', 't1', 't1')",
            [],
        )
        .unwrap();
    let read: (Option<String>, Option<String>) = store
        .conn
        .query_row(
            "SELECT o.from_message, o.from_task FROM ledger_attachment_versions v
             LEFT JOIN ledger_attachment_version_origins o ON o.seq = v.seq
             WHERE v.holder_kind = 'autopilot'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(read, (None, None), "a holder that is not a session has no focus");
}
