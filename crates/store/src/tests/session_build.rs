//! What a Session built, kept and read back: the instants and the task list survive a reopen, and the
//! road is read off the timeline by subject and kind. `docs/concepts/session.md`, *The build record*.

use std::collections::BTreeMap;

use crate::session_build::{LANDED_PULLS, PREVIOUS_LANDING};
use crate::tests::{open, TempDir};
use crate::{AttachmentState, Holder, KeptAttachment, KeptBuild, KeptTask, RoadAsk, Store, TimelineEntry};

fn entry(at: &str, kind: &str, subject_kind: &str, subject_id: &str) -> TimelineEntry {
    TimelineEntry {
        at: at.to_string(),
        kind: kind.to_string(),
        subject_kind: subject_kind.to_string(),
        subject_id: subject_id.to_string(),
        subject_url: None,
        title: None,
        repo: Some("armada".to_string()),
        tone: "idle".to_string(),
        says: "x".to_string(),
        detail: None,
        items: Vec::new(),
        service: None,
        service_name: None,
        to_id: None,
        live: false,
        over: false,
    }
}

fn holds_pull(store: &mut Store, session: &str, number: &str) {
    store
        .attach(
            &KeptAttachment {
                holder: Holder::session(session),
                kind: "pr".to_string(),
                manifest_id: "armada".to_string(),
                target: number.to_string(),
                state: AttachmentState::Standing,
                detail: BTreeMap::new(),
                since: "2026-10-14T09:00:00.000Z".to_string(),
                changed_at: "2026-10-14T09:00:00.000Z".to_string(),
            },
            false,
        )
        .expect("attached");
}

#[test]
fn the_instants_and_the_list_survive_a_reopen_and_a_second_write_replaces_the_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let build = KeptBuild {
        begun_at: "2026-10-14T09:00:00.000Z".to_string(),
        first_edit_at: Some("2026-10-14T09:05:00.000Z".to_string()),
        last_edit_at: Some("2026-10-14T09:20:00.000Z".to_string()),
        last_tool_at: Some("2026-10-14T09:21:00.000Z".to_string()),
        plan_left_at: None,
        tasks_at: Some("2026-10-14T09:01:00.000Z".to_string()),
    };
    let task = |id: &str, completed: Option<&str>, edited: bool| KeptTask {
        id: id.to_string(),
        text: format!("Task {id}"),
        status: if completed.is_some() { "completed" } else { "pending" }.to_string(),
        first_seen_at: "2026-10-14T09:01:00.000Z".to_string(),
        completed_at: completed.map(str::to_string),
        edited,
        made_by: None,
    };
    let tasks = vec![
        task("1", Some("2026-10-14T09:10:00.000Z"), true),
        KeptTask { made_by: Some("Explore: Map the claims".to_string()), ..task("2", None, false) },
    ];
    store.keep_session_build("s1", &build).expect("kept");
    store.keep_session_build_tasks("s1", &tasks).expect("kept");
    drop(store);

    let mut store = open(&dir);
    assert_eq!(store.session_build("s1").expect("reads"), Some(build.clone()));
    assert_eq!(store.session_build_tasks("s1").expect("reads"), tasks, "in the order the agent wrote them");
    assert_eq!(store.session_build("s2").expect("reads"), None, "a Session that did nothing has no row");

    store.keep_session_build_tasks("s1", &tasks[1..]).expect("kept");
    assert_eq!(store.session_build_tasks("s1").expect("reads"), &tasks[1..]);
    store.forget_session_build("s1").expect("forgotten");
    assert_eq!(store.session_build("s1").expect("reads"), None);
    assert!(store.session_build_tasks("s1").expect("reads").is_empty());
}

#[test]
fn the_road_is_the_first_of_each_kind_inside_the_stretch_asked_for() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for (at, kind, subject, id) in [
        ("2026-10-14T08:00:00.000Z", "session.walked", "session", "s1"),
        ("2026-10-14T09:30:00.000Z", "session.walked", "session", "s1"),
        ("2026-10-14T09:40:00.000Z", "session.walked", "session", "s1"),
        ("2026-10-14T09:50:00.000Z", "session.pushed", "session", "s1"),
        ("2026-10-14T09:55:00.000Z", "session.pushed", "session", "other"),
        ("2026-10-14T10:00:00.000Z", "pr.opened", "pull_request", "12"),
        ("2026-10-14T10:05:00.000Z", "pr.checks", "pull_request", "12"),
        ("2026-10-14T10:06:00.000Z", "pr.checks", "pull_request", "12"),
        ("2026-10-14T10:07:00.000Z", "pr.opened", "pull_request", "13"),
        ("2026-10-14T10:20:00.000Z", "pr.merged", "pull_request", "12"),
    ] {
        store.append_timeline(&entry(at, kind, subject, id)).expect("appended");
    }
    let ask = RoadAsk {
        session: "s1",
        manifest: "armada",
        pull: Some("12"),
        from: "2026-10-14T09:00:00.000Z",
        walk_from: "2026-10-14T09:35:00.000Z",
        until: None,
    };
    let road = store.session_road(&ask).expect("reads");
    assert_eq!(road.walked.as_deref(), Some("2026-10-14T09:40:00.000Z"), "a walk counts from the first edit");
    assert_eq!(road.pushed.as_deref(), Some("2026-10-14T09:50:00.000Z"), "the Session's own push, not another's");
    assert_eq!(road.opened.as_deref(), Some("2026-10-14T10:00:00.000Z"), "its pull request's, not 13's");
    assert_eq!(road.checked.as_deref(), Some("2026-10-14T10:05:00.000Z"), "the first");
    assert_eq!(road.queued, None);
    assert_eq!(road.merged.as_deref(), Some("2026-10-14T10:20:00.000Z"));

    let before_merge = store.session_road(&RoadAsk { until: Some("2026-10-14T10:10:00.000Z"), ..ask }).expect("reads");
    assert_eq!(before_merge.merged, None, "the stretch ends where asked");
    let none = store.session_road(&RoadAsk { pull: None, ..ask }).expect("reads");
    assert_eq!((none.opened, none.merged), (None, None), "no pull request, no stops of one");
}

#[test]
fn the_landed_pulls_are_those_a_session_held_and_the_forge_merged() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    holds_pull(&mut store, "s1", "12");
    holds_pull(&mut store, "s2", "14");
    for (at, number) in [("2026-10-14T10:20:00.000Z", "12"), ("2026-10-14T11:00:00.000Z", "13"), ("2026-10-14T12:00:00.000Z", "14")] {
        store.append_timeline(&entry(at, "pr.merged", "pull_request", number)).expect("appended");
    }
    let landed = store.landed_pulls("armada", 10).expect("reads");
    assert_eq!(
        landed.iter().map(|one| (one.session.as_str(), one.number.as_str())).collect::<Vec<_>>(),
        [("s2", "14"), ("s1", "12")],
        "newest first; 13 was nobody's"
    );
    assert!(store.landed_pulls("another", 10).expect("reads").is_empty());
    assert_eq!(store.landed_pulls("armada", 1).expect("reads").len(), 1);
    assert_eq!(
        store.previous_landing("s1", "armada", "2026-10-14T13:00:00.000Z").expect("reads").as_deref(),
        Some("2026-10-14T10:20:00.000Z")
    );
    assert_eq!(store.previous_landing("s1", "armada", "2026-10-14T10:20:00.000Z").expect("reads"), None);
}

#[test]
fn the_landed_pulls_are_read_off_an_index_of_merged_pulls_not_a_walk_of_the_timeline() {
    let dir = TempDir::new();
    let store = open(&dir);
    for (name, sql, bound) in [("landed_pulls", LANDED_PULLS, 2), ("previous_landing", PREVIOUS_LANDING, 3)] {
        let mut explain = store.conn.prepare(&format!("EXPLAIN QUERY PLAN {sql}")).expect("explains");
        let details: Vec<String> = if bound == 2 {
            explain.query_map(rusqlite::params!["armada", 10], |row| row.get(3)).expect("plans").collect::<Result<_, _>>()
        } else {
            explain.query_map(rusqlite::params!["s1", "armada", "2026-10-14T13:00:00.000Z"], |row| row.get(3)).expect("plans").collect::<Result<_, _>>()
        }
        .expect("rows");
        let on_timeline: Vec<&String> = details.iter().filter(|line| line.contains(" t ")).collect();
        assert!(
            on_timeline.iter().any(|line| line.starts_with("SEARCH") && line.contains("timeline_events_by_repo_kind")),
            "{name} searches the merged-pull index; plan: {details:?}"
        );
        assert!(!on_timeline.iter().any(|line| line.starts_with("SCAN")), "{name} does not walk the timeline; plan: {details:?}");
        assert!(!details.iter().any(|line| line.contains("TEMP B-TREE")), "{name} needs no sort; plan: {details:?}");
    }
}
