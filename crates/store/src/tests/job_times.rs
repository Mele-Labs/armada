//! Where a Job's time went, kept: a Job is waiting to be split until a row of the current reading
//! of the fold stands for its end, so a fix to the fold reaches the rows already kept.

use crate::{KeptBuildSplit, KeptJobTime, KeptPullRequestWait, KeptSession, NewBuild, SessionFigures, SessionState, Store};
use crate::tests::{open, TempDir};

fn ended(id: &str, at: &str) -> KeptSession {
    KeptSession {
        id: id.into(),
        harness: "a_harness".into(),
        origin: "bridge".into(),
        manifest_id: Some("armada".into()),
        cwd: "/repos/armada".into(),
        title: None,
        state: SessionState::Ended,
        started_at: "2026-11-01T09:00:00.000Z".into(),
        last_seen_at: at.into(),
        last_turn_at: None,
        ended_at: Some(at.into()),
        end_reason: None,
        figures: SessionFigures::default(),
        mod_version: None,
    }
}

fn kept(id: &str, ended_at: &str, fold: i64) -> KeptJobTime {
    KeptJobTime {
        session_id: id.into(),
        began_at: "2026-11-01T09:00:00.000Z".into(),
        ended_at: ended_at.into(),
        idle_ms: 1,
        kept_at: "2026-11-03T09:00:00.000Z".into(),
        fold,
        ..Default::default()
    }
}

#[test]
fn a_job_is_split_again_when_its_row_is_of_an_earlier_reading_of_the_fold_or_an_earlier_end() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let end = "2026-11-02T09:00:00.000Z";
    store.keep_session(&ended("s1", end)).unwrap();
    assert_eq!(store.job_awaiting_time(2).unwrap().as_deref(), Some("s1"), "no row yet");

    store.keep_job_time(&kept("s1", end, 2)).unwrap();
    assert_eq!(store.job_awaiting_time(2).unwrap(), None, "a row of this reading for this end");

    assert_eq!(store.job_awaiting_time(3).unwrap().as_deref(), Some("s1"), "a later reading of the fold is waiting");

    store.keep_job_time(&kept("s1", "2026-11-02T08:00:00.000Z", 3)).unwrap();
    assert_eq!(store.job_awaiting_time(3).unwrap().as_deref(), Some("s1"), "a row for an earlier end");
}

#[test]
fn a_row_kept_before_the_fold_was_counted_reads_as_the_first_reading() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let end = "2026-11-02T09:00:00.000Z";
    store.keep_session(&ended("s1", end)).unwrap();
    store.keep_job_time(&kept("s1", end, 0)).unwrap();
    assert_eq!(store.job_awaiting_time(1).unwrap().as_deref(), Some("s1"));
    assert_eq!(store.job_time("s1").unwrap().unwrap().fold, 0);
}

fn build(commit: &str, at: &str) -> NewBuild {
    NewBuild { commit_id: commit.into(), started_at: at.into(), ..Default::default() }
}

#[test]
fn builds_are_kept_in_the_order_they_began_and_the_commit_already_running_is_not_another() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(store.keep_build(&build("b", "2026-11-02T10:00:00.000Z")).unwrap());
    assert!(store.keep_build(&build("a", "2026-11-01T10:00:00.000Z")).unwrap(), "a build of an earlier time, whichever order it is told in");
    assert!(!store.keep_build(&build("b", "2026-11-02T11:00:00.000Z")).unwrap(), "the newest build's commit again");
    assert!(store.keep_build(&build("c", "2026-11-03T10:00:00.000Z")).unwrap());
    assert!(store.keep_build(&build("b", "2026-11-04T10:00:00.000Z")).unwrap(), "back to an earlier commit is a new run of it");
    let commits: Vec<String> = store.builds().unwrap().into_iter().map(|one| one.build.commit_id).collect();
    assert_eq!(commits, ["a", "b", "c", "b"]);
}

#[test]
fn what_git_said_of_a_build_is_kept_and_a_build_not_asked_about_is_listed() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_build(&build("a", "2026-11-01T10:00:00.000Z")).unwrap();
    store.keep_build(&build("b", "2026-11-02T10:00:00.000Z")).unwrap();
    let [first, _] = store.builds_unlooked().unwrap().try_into().unwrap();
    let said = NewBuild { source: Some("preview".into()), main_commit: Some("m".into()), branches: vec!["x".into(), "y".into()], ..first.build };
    store.build_looked(first.id, &said).unwrap();
    let unlooked: Vec<String> = store.builds_unlooked().unwrap().into_iter().map(|one| one.build.commit_id).collect();
    assert_eq!(unlooked, ["b"]);
    let kept = &store.builds().unwrap()[0].build;
    assert_eq!((kept.source.as_deref(), kept.main_commit.as_deref(), kept.branches.clone(), kept.looked), (Some("preview"), Some("m"), vec!["x".to_string(), "y".to_string()], true));
}

#[test]
fn a_jobs_split_by_build_and_its_pull_request_waits_are_kept_with_its_row_and_replaced_with_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let end = "2026-11-02T09:00:00.000Z";
    let part = |build_id, you_ms| KeptBuildSplit { build_id, you_ms, ..Default::default() };
    let wait = |target: &str, wait_ms| KeptPullRequestWait { target: target.into(), began_at: "2026-11-02T08:00:00.000Z".into(), ended_at: end.into(), wait_ms };
    store.keep_job_time(&KeptJobTime { builds: vec![part(0, 5), part(3, 7)], waits: vec![wait("12", 100)], ..kept("s1", end, 1) }).unwrap();
    store.keep_job_time(&kept("s2", end, 1)).unwrap();
    let read = store.job_time("s1").unwrap().unwrap();
    assert_eq!((read.builds, read.waits), (vec![part(0, 5), part(3, 7)], vec![wait("12", 100)]));

    store.keep_job_time(&KeptJobTime { builds: vec![part(4, 9)], waits: vec![], ..kept("s1", end, 2) }).unwrap();
    let read = store.job_time("s1").unwrap().unwrap();
    assert_eq!((read.builds, read.waits), (vec![part(4, 9)], vec![]), "the earlier parts are gone");
    assert!(store.job_time("s2").unwrap().unwrap().builds.is_empty(), "another Job's are not touched");
}

#[test]
fn the_builds_jobs_were_told_of_become_builds_when_the_migration_runs() {
    let dir = TempDir::new();
    let store = open(&dir);
    for (commit, who, at) in [("a", "s1", "2026-10-13T10:05:00.000Z"), ("a", "s2", "2026-10-13T10:01:00.000Z"), ("b", "s1", "2026-10-12T09:00:00.000Z")] {
        store.conn.execute("INSERT INTO build_notices (commit_id, recipient, noted_at) VALUES (?1, ?2, ?3)", (commit, who, at)).unwrap();
    }
    let sql = include_str!("../../migrations/20261103T1510Z-fleet_builds.table.sql");
    let backfill = &sql[sql.find("INSERT INTO fleet_builds").unwrap()..];
    store.conn.execute_batch(backfill).unwrap();
    let kept: Vec<(String, String, Option<String>, bool)> = store
        .builds()
        .unwrap()
        .into_iter()
        .map(|one| (one.build.commit_id, one.build.started_at, one.build.source, one.build.looked))
        .collect();
    assert_eq!(
        kept,
        [("b".into(), "2026-10-12T09:00:00.000Z".into(), None, false), ("a".into(), "2026-10-13T10:01:00.000Z".into(), None, false)],
        "a build begins when the first Job was told of it, and its contents are unknown"
    );
}

#[test]
fn a_row_kept_before_the_fold_column_existed_comes_up_as_the_first_reading() {
    let dir = TempDir::new();
    let before = crate::migrations::MIGRATIONS.iter().position(|one| one.name == "job_times.fold").unwrap();
    drop(Store::open_with(&dir.db(), &crate::migrations::MIGRATIONS[..before]).expect("the store as it was"));
    let conn = rusqlite::Connection::open(dir.db()).unwrap();
    conn.execute(
        "INSERT INTO job_times (session_id, began_at, ended_at, model_ms, tools_ms, checks_ms, subagents_ms, slot_ms, you_ms, pull_request_ms,
         stopped_ms, idle_ms, tool_times_missing, model_starts_guessed, timeline_cut, kept_at)
         VALUES ('old', 'b', 'e', 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 'k')",
        [],
    )
    .unwrap();
    drop(conn);

    let store = open(&dir);

    assert_eq!(store.job_time("old").unwrap().unwrap().fold, 0, "a row the fold never read, so every fold since is later than it");
}
