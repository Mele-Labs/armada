//! Where a Job's time went, kept: a Job is waiting to be split until a row of the current reading
//! of the fold stands for its end, so a fix to the fold reaches the rows already kept.

use crate::{KeptJobTime, KeptSession, SessionFigures, SessionState};
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
