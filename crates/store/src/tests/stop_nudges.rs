//! The stop a Job was told about and the stop passed over, and when Fleet was away.

use crate::tests::{open, TempDir};
use crate::KeptStop;

const STOP: &str = "2026-11-03T10:00:00.000Z";
const TOLD: &str = "2026-11-03T10:01:00.000Z";

#[test]
fn a_job_with_no_row_has_been_told_nothing() {
    let dir = TempDir::new();
    assert_eq!(open(&dir).session_stop("s1").expect("reads"), KeptStop::default());
}

#[test]
fn what_it_was_told_is_kept_and_cleared_and_what_was_passed_over_stays() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.pass_over_stop("s1", STOP).expect("keeps");
    store.keep_stop_told("s1", TOLD, TOLD, true).expect("keeps");
    let kept = store.session_stop("s1").expect("reads");
    assert_eq!(kept.told, Some((TOLD.to_string(), TOLD.to_string())));
    assert!(kept.unheard);
    assert_eq!(kept.passed_over.as_deref(), Some(STOP));

    store.clear_stop_told("s1").expect("clears");
    let kept = store.session_stop("s1").expect("reads");
    assert_eq!((kept.told, kept.unheard, kept.passed_over.as_deref()), (None, false, Some(STOP)));
}

#[test]
fn a_repair_is_kept_on_both_rows_and_cleared_when_its_job_runs_again() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    store.keep_repair_held("s1", true).expect("keeps");
    assert!(store.session_stop("s1").expect("reads").repair_held);
    store.keep_repair_started("s1", "r1").expect("keeps");
    let kept = store.session_stop("s1").expect("reads");
    assert_eq!((kept.repair_job.as_deref(), kept.repair_held), (Some("r1"), false));
    assert_eq!(store.session_stop("r1").expect("reads").repairs.as_deref(), Some("s1"));

    store.clear_repair("s1").expect("clears");
    assert_eq!(store.session_stop("s1").expect("reads").repair_job, None);
    assert_eq!(store.session_stop("r1").expect("reads").repairs.as_deref(), Some("s1"), "it stays a repair Job");
}

#[test]
fn a_gap_in_the_beat_is_kept_as_an_outage_and_a_short_one_is_not() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    assert_eq!(store.fleet_started(STOP, 90).expect("keeps"), None, "never seen up");
    store.keep_fleet_alive(STOP).expect("keeps");
    assert_eq!(store.fleet_started("2026-11-03T10:00:30.000Z", 90).expect("keeps"), None, "a restart, not an outage");
    let back = "2026-11-03T11:00:00.000Z";
    let away = store.fleet_started(back, 90).expect("keeps").expect("an outage");
    assert_eq!(away, ("2026-11-03T10:00:30.000Z".to_string(), back.to_string()));

    let overlaps = store.fleet_outages_between("2026-11-03T10:30:00.000Z", "2026-11-03T12:00:00.000Z").expect("reads");
    assert_eq!(overlaps, vec![away]);
    assert!(store.fleet_outages_between("2026-11-03T11:30:00.000Z", "2026-11-03T12:00:00.000Z").expect("reads").is_empty());
}

#[test]
fn landing_passes_over_every_live_jobs_stop_and_leaves_the_rest() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    let job = |id: &str, state: crate::SessionState, last: Option<&str>| crate::KeptSession {
        state,
        last_turn_at: last.map(str::to_string),
        ..super::session_ledger::session(id)
    };
    store.keep_session(&job("live", crate::SessionState::Live, Some(STOP))).expect("keeps");
    store.keep_session(&job("fresh", crate::SessionState::Live, None)).expect("keeps");
    store.keep_session(&job("over", crate::SessionState::Ended, Some(STOP))).expect("keeps");
    store
        .conn
        .execute_batch(concat!("DROP TABLE stop_nudges;", include_str!("../../migrations/20261103T1300Z-stop_nudges.table.sql")))
        .expect("the migration runs");
    assert_eq!(store.session_stop("live").expect("reads").passed_over.as_deref(), Some(STOP));
    assert_eq!(store.session_stop("fresh").expect("reads"), KeptStop::default());
    assert_eq!(store.session_stop("over").expect("reads"), KeptStop::default());
}
