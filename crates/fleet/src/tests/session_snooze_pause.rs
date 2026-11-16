//! A snooze pauses a hosted Job as `pause_session` does, and when it ends Fleet resumes the Job if
//! the snooze was what paused it. `docs/concepts/session/snoozing-a-job.md`.

use std::sync::Arc;

use api::HostedSessions;
use ipc::{AttachmentState, SessionId, SessionRecord, SessionRow, SessionTurn, SnoozeSession};

use super::planted::Held;
use super::session_host::{eventually, rig_fitted, Rig};
use super::session_pause::{holding_slot_one, pause, resume};
use crate::clock::Clock;

/// 03:00 on the held clock's day.
const UNTIL: &str = "2026-08-26T03:00:00.000Z";

fn rig_on(clock: &Arc<Held>) -> Rig {
    let clock = Arc::clone(clock);
    rig_fitted(move |fittings| fittings.clock = clock as Arc<dyn Clock>)
}

fn snooze(id: &SessionId, until: Option<&str>) -> SnoozeSession {
    SnoozeSession { session_id: id.clone(), until: until.map(ipc::Instant::carried) }
}

async fn row_of(rig: &Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("a session").session
}

fn paused(record: &SessionRecord) -> bool {
    record.hosted.as_ref().expect("hosted").paused.is_some()
}

fn holds_a_slot(record: &SessionRecord) -> bool {
    record.attachments.iter().any(|one| one.kind == "slot" && one.state == AttachmentState::Standing)
}

async fn told_the_snooze_ended(rig: &Rig, id: &SessionId) -> bool {
    rig.rows(id).await.iter().any(|row| matches!(row, SessionRow::Message { text, .. } if text.contains("the snooze has ended")))
}

#[tokio::test]
async fn a_snooze_pauses_a_working_job_as_pause_does() {
    let clock = Arc::new(Held::started());
    let rig = rig_on(&clock);
    let id = holding_slot_one(&rig).await;

    let snoozed = Arc::clone(&rig.fleet).snooze_session(snooze(&id, Some(UNTIL))).await.expect("snoozed");

    assert_eq!(snoozed.snoozed_until.as_ref().map(|at| at.as_str()), Some(UNTIL));
    assert!(paused(&snoozed), "paused");
    assert_eq!(rig.turn(&id).await, SessionTurn::Idle, "the turn was ended");
    assert!(!holds_a_slot(&snoozed), "the slot is given back");
}

#[tokio::test]
async fn the_snooze_running_out_resumes_the_job() {
    let clock = Arc::new(Held::started());
    let rig = rig_on(&clock);
    let id = holding_slot_one(&rig).await;
    Arc::clone(&rig.fleet).snooze_session(snooze(&id, Some(UNTIL))).await.expect("snoozed");

    rig.fleet.snoozes_ended().await;
    assert!(paused(&row_of(&rig, &id).await), "still paused before the time");

    clock.on(3 * 3_600);
    rig.fleet.snoozes_ended().await;

    let row = row_of(&rig, &id).await;
    assert_eq!(row.snoozed_until, None);
    assert!(!paused(&row), "resumed");
    assert!(holds_a_slot(&row), "a slot again");
    eventually(|| told_the_snooze_ended(&rig, &id)).await;
}

#[tokio::test]
async fn waking_a_snoozed_job_resumes_it_now() {
    let clock = Arc::new(Held::started());
    let rig = rig_on(&clock);
    let id = holding_slot_one(&rig).await;
    Arc::clone(&rig.fleet).snooze_session(snooze(&id, Some(UNTIL))).await.expect("snoozed");

    let woken = Arc::clone(&rig.fleet).snooze_session(snooze(&id, None)).await.expect("woken");

    assert_eq!(woken.snoozed_until, None);
    assert!(!paused(&woken), "resumed");
    assert!(holds_a_slot(&woken));
}

#[tokio::test]
async fn moving_the_time_keeps_the_resume_owed() {
    let clock = Arc::new(Held::started());
    let rig = rig_on(&clock);
    let id = holding_slot_one(&rig).await;
    Arc::clone(&rig.fleet).snooze_session(snooze(&id, Some("2026-08-26T01:00:00.000Z"))).await.expect("snoozed");
    Arc::clone(&rig.fleet).snooze_session(snooze(&id, Some(UNTIL))).await.expect("moved");

    clock.on(3 * 3_600);
    rig.fleet.snoozes_ended().await;

    assert!(!paused(&row_of(&rig, &id).await), "resumed");
}

#[tokio::test]
async fn a_job_he_paused_before_the_snooze_stays_paused_after_it() {
    let clock = Arc::new(Held::started());
    let rig = rig_on(&clock);
    let id = holding_slot_one(&rig).await;
    let before = rig.fleet.pause_session(pause(&id)).await.expect("paused");

    let snoozed = Arc::clone(&rig.fleet).snooze_session(snooze(&id, Some(UNTIL))).await.expect("snoozed");
    assert_eq!(snoozed.hosted.as_ref().unwrap().paused, before.hosted.as_ref().unwrap().paused, "his pause, as it was");

    clock.on(3 * 3_600);
    rig.fleet.snoozes_ended().await;

    let row = row_of(&rig, &id).await;
    assert_eq!(row.snoozed_until, None);
    assert!(paused(&row), "still his pause");
}

#[tokio::test]
async fn a_job_he_resumed_and_paused_again_during_the_snooze_is_left_as_he_left_it() {
    let clock = Arc::new(Held::started());
    let rig = rig_on(&clock);
    let id = holding_slot_one(&rig).await;
    Arc::clone(&rig.fleet).snooze_session(snooze(&id, Some(UNTIL))).await.expect("snoozed");
    Arc::clone(&rig.fleet).resume_session(resume(&id)).await.expect("resumed by hand");
    rig.fleet.pause_session(pause(&id)).await.expect("paused by hand");

    let woken = Arc::clone(&rig.fleet).snooze_session(snooze(&id, None)).await.expect("woken");

    assert_eq!(woken.snoozed_until, None);
    assert!(paused(&woken), "his own pause stands");
}

#[tokio::test]
async fn a_fleet_restart_during_the_snooze_still_resumes_the_job_at_its_time() {
    let clock = Arc::new(Held::started());
    let rig = rig_on(&clock);
    let id = holding_slot_one(&rig).await;
    Arc::clone(&rig.fleet).snooze_session(snooze(&id, Some(UNTIL))).await.expect("snoozed");

    let later = Arc::new(Held::started());
    later.on(3 * 3_600);
    let after = Arc::clone(&later);
    let rig = rig.restarted(move |fittings| fittings.clock = after as Arc<dyn Clock>);
    assert!(paused(&row_of(&rig, &id).await), "paused across the restart");

    rig.fleet.snoozes_ended().await;

    let row = row_of(&rig, &id).await;
    assert_eq!(row.snoozed_until, None);
    assert!(!paused(&row), "resumed at its time");
}
