//! The sums the Usage page reads time from: burn and four medians an hour and a build, the Jobs'
//! time together and the usual Job of a workflow. Medians are of what happened, so an hour with
//! nothing has no figure, and a question to the owner is no tool's time.

use crate::session_turn_usage::KeptTurnUsage;
use crate::tests::{open, TempDir};
use crate::{KeptBuildSplit, KeptJobTime, KeptPullRequestWait, NewBuild, Store};

/// A time on 3 Nov, the seconds carrying into the minutes and the hours.
fn at(hour: u32, minute: u32, second: u32) -> String {
    let total = hour * 3600 + minute * 60 + second;
    format!("2026-11-03T{:02}:{:02}:{:02}.000Z", total / 3600, total / 60 % 60, total % 60)
}

/// A main-thread turn at `when`: its model began `waited` seconds before, and its tools, if it
/// names any, ran for `ran` seconds after.
fn turn(store: &mut Store, session: &str, id: &str, model: &str, when: (u32, u32, u32), waited: u32, tools: &[(&str, u32)]) {
    let (hour, minute, second) = when;
    let turn = KeptTurnUsage {
        message_id: id.into(),
        at: at(hour, minute, second),
        model: model.into(),
        input_tokens: 1_000_000,
        cache_creation_tokens: 0,
        cache_read_tokens: 0,
        output_tokens: 0,
        tools: tools.iter().map(|(name, _)| name.to_string()).collect(),
        subagent: false,
        bash_use: None,
    };
    store.keep_turn_usage(session, &turn).unwrap();
    let began = at(hour, minute, second - waited);
    let ran = tools.first().map(|(_, ran)| *ran);
    let ended = ran.map(|ran| at(hour, minute, second + ran));
    store
        .conn
        .execute(
            "UPDATE session_turn_usage SET model_began = ?3, tools_began = ?4, tools_ended = ?5 WHERE session_id = ?1 AND message_id = ?2",
            (session, id, began, ran.map(|_| at(hour, minute, second)), ended),
        )
        .unwrap();
}

fn check(store: &Store, session: &str, started: String, ended: String) {
    store
        .conn
        .execute(
            "INSERT INTO session_check_runs (session_id, manifest_id, slot, name, state, started_at, ended_at) VALUES (?1, 'armada', 1, 'test', 'passed', ?2, ?3)",
            (session, started, ended),
        )
        .unwrap();
}

#[test]
fn an_hour_has_its_burn_by_model_and_the_median_of_what_ran_in_it_and_an_empty_hour_has_nothing() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    // Hour 0: model waits of 10 and 30 seconds (median 20), a 40 second tool, a question to the owner
    // that took an hour and is no tool's time. Hour 2: one wait of 7.
    turn(&mut store, "s1", "a", "big", (10, 5, 40), 10, &[("Bash", 40)]);
    turn(&mut store, "s1", "b", "big", (10, 6, 40), 30, &[("AskUserQuestion", 3600)]);
    turn(&mut store, "s1", "c", "small", (12, 30, 20), 7, &[]);
    check(&store, "s1", at(10, 10, 0), at(10, 10, 30));
    check(&store, "s1", at(10, 20, 0), at(10, 21, 30));
    store
        .keep_job_time(&KeptJobTime {
            waits: vec![KeptPullRequestWait { target: "9".into(), began_at: at(12, 0, 0), ended_at: at(12, 40, 0), wait_ms: 600_000 }],
            ..crate::KeptJobTime { session_id: "s1".into(), began_at: at(9, 0, 0), ended_at: at(13, 0, 0), kept_at: at(13, 0, 0), ..Default::default() }
        })
        .unwrap();

    let read = store.usage_hours(Some("2026-11-03T10:00:00.000Z"), 3, &at(12, 59, 0)).unwrap().unwrap();

    assert_eq!(read.hours.iter().map(|one| one.hour).collect::<Vec<_>>(), [0, 2], "hour 1 had nothing");
    let (first, last) = (&read.hours[0], &read.hours[1]);
    assert_eq!(first.at, at(10, 0, 0));
    assert_eq!(first.burn, [("big".to_string(), 2.0)]);
    assert_eq!(first.medians.model, Some(20.0));
    assert_eq!(first.medians.tools, Some(40.0), "the hour of waiting on the owner is not a tool's");
    assert_eq!(first.medians.checks, Some(60.0), "30 and 90 seconds");
    assert_eq!(first.medians.pull_request, None);
    assert_eq!(last.burn, [("small".to_string(), 1.0)]);
    assert_eq!((last.medians.model, last.medians.tools, last.medians.checks), (Some(7.0), None, None));
    assert_eq!(last.medians.pull_request, Some(600.0));
}

#[test]
fn a_window_not_named_ends_with_the_hour_it_is_now_and_a_time_that_is_not_one_is_refused() {
    let dir = TempDir::new();
    let store = open(&dir);
    let read = store.usage_hours(None, 48, &at(12, 59, 59)).unwrap().unwrap();
    assert_eq!((read.since.as_str(), read.until.as_str()), ("2026-11-01T13:00:00.000Z", "2026-11-03T13:00:00.000Z"));
    assert!(store.usage_hours(Some("tomorrow"), 48, &at(12, 0, 0)).unwrap().is_none());
}

fn build(commit: &str, hour: u32) -> NewBuild {
    NewBuild { commit_id: commit.into(), started_at: at(hour, 0, 0), source: Some("main".into()), looked: true, ..Default::default() }
}

#[test]
fn a_build_is_credited_the_turns_kept_while_it_ran_and_the_one_running_at_the_start_is_in() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    for (commit, hour) in [("a", 8), ("b", 10), ("c", 12)] {
        store.keep_build(&build(commit, hour)).unwrap();
    }
    // Build a: 8-10 (before the window begins at 11, yet running into it? no: it ended at 10).
    turn(&mut store, "s1", "1", "m", (9, 30, 20), 20, &[]);
    // Build b: 10-12, running at 11. Waits of 4 and 8: median 6.
    turn(&mut store, "s1", "2", "m", (10, 30, 20), 4, &[]);
    turn(&mut store, "s2", "3", "m", (11, 30, 20), 8, &[("Bash", 12)]);
    // Build c: 12 on.
    turn(&mut store, "s2", "4", "m", (12, 30, 59), 50, &[]);
    check(&store, "s2", at(11, 0, 0), at(11, 0, 20));
    let part = |build_id, you_ms| KeptBuildSplit { build_id, you_ms, idle_ms: 1, ..Default::default() };
    store.keep_job_time(&KeptJobTime { session_id: "s1".into(), builds: vec![part(0, 5), part(2, 100)], ..Default::default() }).unwrap();
    store.keep_job_time(&KeptJobTime { session_id: "s2".into(), builds: vec![part(2, 50), part(3, 7)], ..Default::default() }).unwrap();

    let read = store.usage_builds(Some("2026-11-03T11:00:00.000Z"), &at(13, 0, 0)).unwrap().unwrap();

    let names: Vec<&str> = read.iter().map(|one| one.build.build.commit_id.as_str()).collect();
    assert_eq!(names, ["b", "c"], "a ended before the window, b was running as it began");
    let (b, c) = (&read[0], &read[1]);
    assert_eq!((b.turns, b.units, b.jobs.clone()), (2, 2.0, vec!["s1".to_string(), "s2".to_string()]));
    assert_eq!(b.until.as_deref(), Some(at(12, 0, 0).as_str()));
    assert_eq!((b.medians.model, b.medians.tools, b.medians.checks), (Some(6.0), Some(12.0), Some(20.0)));
    assert_eq!(b.split.you, 150, "both Jobs' time in b, and none of the stretch before any build");
    assert_eq!((c.turns, c.until.clone(), c.medians.model, c.split.you), (1, None, Some(50.0), 7));
}

#[test]
fn the_time_of_the_jobs_that_ended_in_a_window_the_slowest_and_the_usual_of_each_workflow() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = |id: &str, workflow: &str, ended: &str, you_ms: i64, idle_ms: i64| KeptJobTime {
        session_id: id.into(),
        workflow: workflow.into(),
        began_at: "2026-11-01T00:00:00.000Z".into(),
        ended_at: ended.into(),
        you_ms,
        idle_ms,
        kept_at: ended.into(),
        ..Default::default()
    };
    for one in [
        job("a", "bug", "2026-11-02T10:00:00.000Z", 10_000, 0),
        job("b", "bug", "2026-11-02T11:00:00.000Z", 30_000, 5_000),
        job("c", "bug", "2026-11-02T12:00:00.000Z", 20_000, 1_000),
        job("d", "feature", "2026-11-02T13:00:00.000Z", 1_000, 1_000),
        job("e", "", "2026-11-02T14:00:00.000Z", 9_000, 0),
        job("old", "bug", "2026-10-01T10:00:00.000Z", 999_000, 0),
    ] {
        store.keep_job_time(&one).unwrap();
    }
    let read = store.usage_time(Some("2026-11-02T00:00:00.000Z"), Some("2026-11-03T00:00:00.000Z"), &at(0, 0, 0), 2).unwrap().unwrap();

    assert_eq!(read.jobs, 5, "old ended before the window");
    assert_eq!((read.total.you, read.total.idle), (70_000, 7_000));
    let order: Vec<&str> = read.slowest.iter().map(|one| one.session_id.as_str()).collect();
    assert_eq!(order, ["b", "c"], "longest first, as many as asked for, and none from outside the window");
    // The usual is of every Job kept of the workflow, the old one too: 10, 20, 30 and 999 seconds.
    let usual: Vec<(&str, u32, i64, i64)> = read.usual.iter().map(|(name, one)| (name.as_str(), one.jobs, one.split.you, one.split.idle)).collect();
    assert_eq!(usual, [("bug", 4, 25_000, 500), ("feature", 1, 1_000, 1_000)], "a Job with no workflow is in none");
}
