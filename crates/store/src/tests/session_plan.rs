//! A Session's plan and the files it touched, kept and read back: a recording replaces the plan whole,
//! approval holds to the version read, progress and edits move only what they name, and the touches
//! are a bounded window in which a repeat is one. `docs/concepts/session.md`, *The plan*.

use crate::tests::{open, TempDir};
use crate::{KeptPlan, KeptPlanChange, KeptPlanGroup, KeptPlanTask, KeptTouch};

fn task(id: &str, text: &str) -> KeptPlanTask {
    KeptPlanTask { id: id.to_string(), text: text.to_string(), listed: None, completed_at: None, edited: false }
}

fn plan(version: u32, tasks: &[(&str, &str)]) -> KeptPlan {
    KeptPlan {
        version,
        recorded_at: format!("2026-10-16T09:0{version}:00.000Z"),
        approved_at: None,
        sketch: Some(("sketch:Flow".to_string(), "Flow".to_string())),
        walk: (version == 1).then(|| "http://localhost:5173/?walk=plan".to_string()),
        changes: (version > 1).then(|| vec![KeptPlanChange { kind: "added".into(), id: "t9".into(), other: String::new() }]),
        next_group: 2,
        next_task: 10,
        groups: vec![KeptPlanGroup {
            id: "g1".to_string(),
            agent: "rust-engineer".to_string(),
            title: "Store".to_string(),
            paths: vec!["crates/store".to_string(), "docs".to_string()],
            issue: Some(2214),
            worked_at: None,
            tasks: tasks.iter().map(|(id, text)| task(id, text)).collect(),
        }],
    }
}

fn touch(by: Option<(&str, &str)>, path: &str, at: u32) -> KeptTouch {
    KeptTouch {
        by: by.map(|(agent, id)| (agent.to_string(), id.to_string())),
        path: path.to_string(),
        act: "edit".to_string(),
        at: format!("2026-10-16T09:00:{at:02}.000Z"),
    }
}

#[test]
fn a_plan_survives_a_reopen_with_its_order_paths_and_changes() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.session_plan("s1").unwrap(), None);
    let kept = plan(2, &[("t1", "Add the tables"), ("t2", "Add the migration")]);
    store.keep_session_plan("s1", &kept).unwrap();
    drop(store);
    assert_eq!(open(&dir).session_plan("s1").unwrap(), Some(kept));
}

#[test]
fn recording_again_replaces_the_plan_whole_and_leaves_another_sessions_alone() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_session_plan("s1", &plan(1, &[("t1", "One"), ("t2", "Two")])).unwrap();
    store.keep_session_plan("s2", &plan(1, &[("t1", "Other")])).unwrap();
    store.keep_session_plan("s1", &plan(2, &[("t2", "Two")])).unwrap();

    let held = store.session_plan("s1").unwrap().expect("a plan");
    assert_eq!(held.version, 2);
    assert_eq!(held.groups[0].tasks.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["t2"]);
    assert_eq!(store.session_plan("s2").unwrap().unwrap().groups[0].tasks[0].text, "Other");
}

#[test]
fn approval_holds_to_the_version_read_and_is_given_once() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_session_plan("s1", &plan(2, &[("t1", "One")])).unwrap();
    assert!(!store.approve_session_plan("s1", 1, "2026-10-16T10:00:00.000Z").unwrap(), "not the version held");
    assert!(store.approve_session_plan("s1", 2, "2026-10-16T10:00:00.000Z").unwrap());
    assert!(!store.approve_session_plan("s1", 2, "2026-10-16T10:05:00.000Z").unwrap(), "already approved");
    assert_eq!(store.session_plan("s1").unwrap().unwrap().approved_at.as_deref(), Some("2026-10-16T10:00:00.000Z"));
    // A new recording is pending again.
    store.keep_session_plan("s1", &plan(3, &[("t1", "One")])).unwrap();
    assert_eq!(store.session_plan("s1").unwrap().unwrap().approved_at, None);
}

#[test]
fn progress_and_an_edit_move_only_what_they_name() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_session_plan("s1", &plan(1, &[("t1", "One"), ("t2", "Two")])).unwrap();
    store.keep_session_plan_edit("s1", &["g1".to_string()], &["t2".to_string()], "2026-10-16T09:30:00.000Z").unwrap();
    let held = store.session_plan("s1").unwrap().unwrap();
    assert_eq!(held.groups[0].worked_at.as_deref(), Some("2026-10-16T09:30:00.000Z"));
    assert_eq!((held.groups[0].tasks[0].edited, held.groups[0].tasks[1].edited), (false, true));

    let mut moved = held.clone();
    moved.groups[0].tasks[0].listed = Some("completed".to_string());
    moved.groups[0].tasks[0].completed_at = Some("2026-10-16T09:40:00.000Z".to_string());
    store.keep_session_plan_progress("s1", &moved).unwrap();
    let held = store.session_plan("s1").unwrap().unwrap();
    assert_eq!(held.groups[0].tasks[0].listed.as_deref(), Some("completed"));
    assert_eq!(held.groups[0].tasks[0].completed_at.as_deref(), Some("2026-10-16T09:40:00.000Z"));
    assert_eq!(held.version, 1, "the head is not touched");
}

#[test]
fn touches_are_a_bounded_window_oldest_first_and_a_repeat_by_the_same_hand_is_one() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.keep_session_touch("s1", &touch(None, "src/a.rs", 1), 3).unwrap());
    assert!(!store.keep_session_touch("s1", &touch(None, "src/a.rs", 2), 3).unwrap(), "the same file by the same hand");
    assert!(store.keep_session_touch("s1", &touch(Some(("rust-engineer", "toolu_1")), "src/a.rs", 3), 3).unwrap(), "another hand");
    let held = store.session_touches("s1").unwrap();
    assert_eq!(held.len(), 2);
    assert_eq!(held[0].at, "2026-10-16T09:00:02.000Z", "the repeat moved the instant");

    for (nth, path) in ["b", "c", "d"].iter().enumerate() {
        store.keep_session_touch("s1", &touch(None, &format!("src/{path}.rs"), 10 + nth as u32), 3).unwrap();
    }
    let held = store.session_touches("s1").unwrap();
    assert_eq!(held.iter().map(|one| one.path.as_str()).collect::<Vec<_>>(), ["src/b.rs", "src/c.rs", "src/d.rs"]);
    assert!(store.session_touches("s2").unwrap().is_empty(), "another session's window is its own");
}
