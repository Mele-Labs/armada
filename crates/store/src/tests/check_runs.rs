//! The history of every Check run, kept beside the last-five timings.

use std::time::Duration;

use core_model::{JobId, ManifestId, Timestamp, Ulid};

use crate::tests::{open, TempDir};
use crate::{CheckOutcome, CheckRun};

fn repository(id: &str) -> ManifestId {
    ManifestId::carried(Ulid::carried(id))
}

fn run(check: &str, started_at: &str) -> CheckRun {
    CheckRun {
        check: check.to_string(),
        started_at: Timestamp::from_rfc3339(started_at),
        outcome: CheckOutcome::Passed,
        narrowed_to: None,
        queue_wait: Duration::from_millis(40),
        took: Duration::from_millis(1_500),
        tests_run: None,
        tests_failed: None,
        load_avg_at_start: Some(2.5),
        cores: 10,
        width: 4,
        places_held: 2,
    }
}

type Row = (
    String,
    Option<String>,
    String,
    bool,
    Option<String>,
    i64,
    i64,
    Option<i64>,
    f64,
);

fn rows(store: &crate::Store) -> Vec<Row> {
    store
        .conn
        .prepare(
            "SELECT check_name, job_id, outcome, narrowed, narrowed_to, queue_wait_ms, took_ms,
                    tests_run, load_avg_at_start
             FROM check_runs ORDER BY started_at, rowid",
        )
        .expect("prepares")
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
            ))
        })
        .expect("reads")
        .collect::<Result<_, _>>()
        .expect("rows")
}

/// Every run is a row, whatever its outcome, with what was known filled and
/// the rest left NULL, and it survives a reopen.
#[test]
fn every_run_is_appended_with_what_was_known() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let mine = repository("01REPO");
    let job = JobId::carried(Ulid::carried("01JOB"));
    store
        .append_check_run(&mine, Some(&job), &run("test", "2026-09-14T00:00:01Z"))
        .expect("kept");
    let mut narrowed = run("test", "2026-09-14T00:00:02Z");
    narrowed.outcome = CheckOutcome::TimedOut;
    narrowed.narrowed_to = Some("cargo test one".to_string());
    store
        .append_check_run(&mine, None, &narrowed)
        .expect("kept");
    drop(store);

    let store = open(&dir);
    let kept = rows(&store);
    assert_eq!(kept.len(), 2);
    assert_eq!(
        kept[0],
        (
            "test".into(),
            Some("01JOB".into()),
            "passed".into(),
            false,
            None,
            40,
            1_500,
            None,
            2.5
        )
    );
    assert_eq!(kept[1].1, None);
    assert_eq!(kept[1].2, "timed_out");
    assert!(kept[1].3, "narrowed is 1 where a narrowed command ran");
    assert_eq!(kept[1].4.as_deref(), Some("cargo test one"));
}

/// Past 90 days drops on the next write; inside it stays. `check_timings`
/// keeps its own last five untouched.
#[test]
fn a_row_older_than_ninety_days_is_pruned_on_the_next_write() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let mine = repository("01REPO");
    store
        .append_check_run(&mine, None, &run("old", "2026-06-01T00:00:00Z"))
        .expect("kept");
    store
        .append_check_run(&mine, None, &run("edge", "2026-06-16T00:00:01Z"))
        .expect("kept");
    store
        .append_check_run(&mine, None, &run("new", "2026-09-14T00:00:00Z"))
        .expect("kept");
    let names: Vec<String> = rows(&store).into_iter().map(|row| row.0).collect();
    assert_eq!(names, ["edge", "new"], "90 days before 14 Sep is 16 Jun");
}
