//! A bug report is a row the owner keeps or drops, read newest first, with its lines back as it was given.

use crate::bug_reports::{KeptBugFix, KeptBugReport};
use crate::tests::{open, TempDir};

fn report(id: &str, at: &str) -> KeptBugReport {
    KeptBugReport {
        id: id.into(),
        session_id: "s1".into(),
        job_title: "Build the report call".into(),
        title: "Preview restart drops the lease".into(),
        what: "Refresh preview left the slot unleased.".into(),
        steps: vec!["Press Refresh preview".into(), "Read the slot".into()],
        errors: vec!["fleet.slot_unleased: no lease".into()],
        logs: vec!["/tmp/fleet.log".into()],
        at: at.into(),
        kept: None,
        fixed: None,
    }
}

fn fix(number: u64) -> KeptBugFix {
    KeptBugFix {
        at: "2026-10-31T09:00:00.000Z".into(),
        pull_request: number,
        title: "Keep the lease across a refresh".into(),
        url: format!("https://forge.invalid/o/r/pull/{number}"),
    }
}

#[test]
fn a_kept_bug_report_reads_back_with_its_lines() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let sent = report("b1", "2026-10-29T09:00:00.000Z");
    store.keep_bug_report(&sent).unwrap();

    assert_eq!(store.bug_reports().unwrap(), vec![sent]);
}

#[test]
fn bug_reports_are_listed_newest_first() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_bug_report(&report("b1", "2026-10-29T09:00:00.000Z")).unwrap();
    store.keep_bug_report(&report("b3", "2026-10-29T09:10:00.000Z")).unwrap();
    store.keep_bug_report(&report("b2", "2026-10-29T09:05:00.000Z")).unwrap();

    let ids: Vec<String> = store.bug_reports().unwrap().into_iter().map(|one| one.id).collect();
    assert_eq!(ids, ["b3", "b2", "b1"]);
}

#[test]
fn keeping_a_bug_report_twice_keeps_the_first_time() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_bug_report(&report("b1", "2026-10-29T09:00:00.000Z")).unwrap();

    let first = store.mark_bug_report_kept("b1", "2026-10-29T10:00:00.000Z").unwrap().expect("kept");
    assert_eq!(first.kept.as_deref(), Some("2026-10-29T10:00:00.000Z"));
    let second = store.mark_bug_report_kept("b1", "2026-10-29T11:00:00.000Z").unwrap().expect("kept");
    assert_eq!(second.kept.as_deref(), Some("2026-10-29T10:00:00.000Z"));
    assert_eq!(store.bug_reports().unwrap()[0].kept.as_deref(), Some("2026-10-29T10:00:00.000Z"));
}

#[test]
fn keeping_a_missing_bug_report_answers_none() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.mark_bug_report_kept("nope", "2026-10-29T10:00:00.000Z").unwrap(), None);
    assert!(store.bug_reports().unwrap().is_empty());
}

#[test]
fn dropping_a_bug_report_deletes_it_and_a_second_drop_answers_false() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_bug_report(&report("b1", "2026-10-29T09:00:00.000Z")).unwrap();
    store.keep_bug_report(&report("b2", "2026-10-29T09:05:00.000Z")).unwrap();
    store.mark_bug_report_kept("b1", "2026-10-29T10:00:00.000Z").unwrap();

    assert!(store.drop_bug_report("b1").unwrap(), "a kept report is dropped too");
    assert!(!store.drop_bug_report("b1").unwrap());
    let left: Vec<String> = store.bug_reports().unwrap().into_iter().map(|one| one.id).collect();
    assert_eq!(left, ["b2"]);
}

#[test]
fn a_report_fixed_by_a_merge_reads_back_with_the_pull_request_and_one_still_open_reads_none() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_bug_report(&report("b1", "2026-10-29T09:00:00.000Z")).unwrap();
    store.keep_bug_report(&report("b2", "2026-10-29T09:05:00.000Z")).unwrap();

    assert!(store.mark_bug_report_fixed("b1", &fix(41)).unwrap());

    assert_eq!(store.bug_report("b1").unwrap().expect("kept").fixed, Some(fix(41)));
    assert_eq!(store.bug_report("b2").unwrap().expect("kept").fixed, None);
}

#[test]
fn a_report_keeps_the_first_merge_that_fixed_it() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_bug_report(&report("b1", "2026-10-29T09:00:00.000Z")).unwrap();

    assert!(store.mark_bug_report_fixed("b1", &fix(41)).unwrap());
    assert!(!store.mark_bug_report_fixed("b1", &fix(57)).unwrap(), "a second merge changes nothing");

    assert_eq!(store.bug_reports().unwrap()[0].fixed, Some(fix(41)));
}

#[test]
fn fixing_a_missing_bug_report_changes_nothing() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert!(!store.mark_bug_report_fixed("nope", &fix(41)).unwrap());
    assert_eq!(store.bug_report("nope").unwrap(), None);
}
