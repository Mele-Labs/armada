//! A hosted Job whose process dies with a turn open is started again once, by Fleet: told the word,
//! never in a loop, and not at all when the person stopped, paused or closed it or a limit holds it.
//! At boot the same path starts a Job whose keeper is gone. The stand-in process host is
//! `session_host`'s, and the store's open turn is read directly.

use std::sync::Arc;
use std::time::Duration;

use api::HostedSessions;
use ipc::{SessionId, SessionRow, StopSession};
use store::KeptOpenTurn;

use super::session_host::{eventually, rig_fitted, rig_placing, Rig};
use super::session_pause::{holding_slot_one, pause};
use super::session_queue::close;
use fleet::session_host::Heard;

const WORD: &str = "Fleet started it again";

fn rig() -> Rig {
    rig_fitted(|_| {})
}

async fn open_turn(rig: &Rig, id: &SessionId) -> Option<KeptOpenTurn> {
    rig.fleet.store().lock().await.open_turn(id.as_str()).expect("reads")
}

/// A Job whose first process has said its first line, so a turn is open.
async fn working(rig: &Rig) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, "go").await;
    eventually(|| async { rig.stand_in.starts().len() == 1 }).await;
    rig.stand_in.init(0);
    eventually(|| async { open_turn(rig, &id).await.is_some() }).await;
    id
}

async fn told(rig: &Rig, index: usize) -> bool {
    eventually(|| async { rig.stand_in.starts().len() > index }).await;
    rig.stand_in.process(index).sent.lock().unwrap().iter().any(|line| line.contains(WORD))
}

/// Long enough for a start that was going to happen to have happened.
async fn settle() {
    tokio::time::sleep(Duration::from_millis(150)).await;
}

#[tokio::test]
async fn a_process_that_ends_in_a_turn_is_started_again_once_and_told_the_word() {
    let rig = rig();
    let id = working(&rig).await;

    rig.stand_in.hears(0, Heard::Gone);

    eventually(|| async { rig.stand_in.starts().len() == 2 }).await;
    eventually(|| async { told(&rig, 1).await }).await;
    let kept = open_turn(&rig, &id).await.expect("the turn is still kept");
    assert_eq!(kept.started_again, 1);
    assert!(kept.ended_at.is_some());
}

#[tokio::test]
async fn a_second_death_before_the_turn_finishes_starts_nothing() {
    let rig = rig();
    let id = working(&rig).await;
    rig.stand_in.hears(0, Heard::Gone);
    eventually(|| async { rig.stand_in.starts().len() == 2 }).await;
    rig.stand_in.init(1);

    rig.stand_in.hears(1, Heard::Gone);
    settle().await;

    assert_eq!(rig.stand_in.starts().len(), 2, "no third process");
    let kept = open_turn(&rig, &id).await.expect("the turn is still kept");
    assert_eq!((kept.started_again, kept.ended_at.is_some()), (1, true));
}

#[tokio::test]
async fn a_finished_turn_deletes_the_row_so_a_later_death_starts_again() {
    let rig = rig();
    let id = working(&rig).await;
    rig.stand_in.hears(0, Heard::Gone);
    eventually(|| async { rig.stand_in.starts().len() == 2 }).await;
    rig.stand_in.init(1);
    rig.stand_in.finishes(1, "done");
    eventually(|| async { open_turn(&rig, &id).await.is_none() }).await;

    rig.send(&id, "more").await;
    rig.stand_in.init(1);
    eventually(|| async { open_turn(&rig, &id).await.is_some() }).await;
    rig.stand_in.hears(1, Heard::Gone);

    eventually(|| async { rig.stand_in.starts().len() == 3 }).await;
    assert_eq!(open_turn(&rig, &id).await.expect("kept").started_again, 1, "counted anew");
}

#[tokio::test]
async fn a_job_the_person_stopped_is_not_started() {
    let rig = rig();
    let id = working(&rig).await;

    rig.fleet.stop_session(StopSession { session_id: id.clone() }).await.expect("stopped");
    assert!(open_turn(&rig, &id).await.is_none(), "the turn is closed");
    rig.stand_in.hears(0, Heard::Gone);
    settle().await;

    assert_eq!(rig.stand_in.starts().len(), 1);
}

#[tokio::test]
async fn a_job_the_person_paused_is_not_started() {
    let rig = rig_placing(None);
    let id = holding_slot_one(&rig).await;

    rig.fleet.pause_session(pause(&id)).await.expect("paused");
    assert!(open_turn(&rig, &id).await.is_none(), "the turn is closed");
    rig.stand_in.hears(0, Heard::Gone);
    settle().await;

    assert_eq!(rig.stand_in.starts().len(), 1);
}

#[tokio::test]
async fn a_job_the_person_closed_is_not_started() {
    let rig = rig_placing(None);
    let id = working(&rig).await;

    close(&rig, &id).await;
    assert!(open_turn(&rig, &id).await.is_none(), "the turn is closed");
    rig.stand_in.hears(0, Heard::Gone);
    settle().await;

    assert_eq!(rig.stand_in.starts().len(), 1);
}

#[tokio::test]
async fn a_job_held_for_a_limit_is_not_started_and_its_turn_is_closed() {
    let rig = rig();
    let id = working(&rig).await;
    rig.fleet.store().lock().await.keep_session_limit(id.as_str(), "2026-11-04T00:00:00Z").expect("kept");

    rig.stand_in.hears(0, Heard::Gone);
    eventually(|| async { open_turn(&rig, &id).await.is_none() }).await;
    settle().await;

    assert_eq!(rig.stand_in.starts().len(), 1);
}

#[tokio::test]
async fn after_a_restart_a_job_with_an_open_turn_and_no_keeper_is_started_and_noted() {
    let (rig, id) = working_then_restarted().await;

    Arc::clone(&rig.fleet).open_turns_started().await;

    eventually(|| async { rig.stand_in.starts().len() == 1 }).await;
    eventually(|| async { told(&rig, 0).await }).await;
    assert!(
        rig.rows(&id).await.iter().any(|row| matches!(row, SessionRow::Message { text, .. } if text.contains("while Fleet was down"))),
        "the thread says so"
    );
}

#[tokio::test]
async fn after_a_restart_a_paused_job_is_not_started_and_gets_no_note() {
    let rig = rig_placing(None);
    let id = holding_slot_one(&rig).await;
    rig.fleet.pause_session(pause(&id)).await.expect("paused");
    // The row a pause raced past: open, with the Job paused.
    rig.fleet.store().lock().await.turn_opened(id.as_str(), "2026-11-03T10:00:00Z").expect("opened");
    let rig = rig.restarted(|_| {});

    Arc::clone(&rig.fleet).open_turns_started().await;
    settle().await;

    assert!(rig.stand_in.starts().is_empty());
    assert!(open_turn(&rig, &id).await.is_none(), "the turn is closed");
    assert!(
        !rig.rows(&id).await.iter().any(|row| matches!(row, SessionRow::Message { text, .. } if text.contains("while Fleet was down"))),
        "no note"
    );
}

async fn working_then_restarted() -> (Rig, SessionId) {
    let rig = rig();
    let id = working(&rig).await;
    (rig.restarted(|_| {}), id)
}
