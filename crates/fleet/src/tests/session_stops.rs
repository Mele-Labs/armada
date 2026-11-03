//! The claim: **a Job that ends its turn with nothing running and nothing asked is told once to carry on or name
//! what it waits on, and if it stops again its row says Stopped.** Read from what Fleet saw, one nudge per stop,
//! so a Job can never loop with Fleet. `docs/concepts/session/what-a-job-is-doing.md`.

use std::sync::Arc;

use api::{HostedSessions, Watches};
use ipc::{SessionId, SessionRecord, SessionRow, SessionStanding, SetWaitingFor, StartWatch, StopReason, StopSession, WaitingInput, WatchStarts};
use store::KeptStop;

use crate::session_host::stopping::{self, read, Read};

use super::session_host::{eventually, rig_placing, Rig};

const ENDED: &str = "2026-11-03T10:00:00.000Z";
const LATER: &str = "2026-11-03T10:03:00.000Z";
const TOLD: &str = "2026-11-03T10:01:00.000Z";

fn millis(at: &str) -> i64 {
    core_model::Timestamp::from_rfc3339(at).epoch_millis().unwrap()
}

fn told(stop: &str, unheard: bool) -> KeptStop {
    KeptStop { told: Some((stop.to_string(), TOLD.to_string())), unheard, passed_over: None }
}

#[test]
fn a_stop_never_told_about_is_owed_a_word() {
    assert_eq!(read(ENDED, &KeptStop::default(), None, millis(LATER)), Read::Owed);
}

#[test]
fn the_stop_he_made_or_that_stood_when_this_landed_is_passed_over() {
    let kept = KeptStop { passed_over: Some(ENDED.to_string()), ..KeptStop::default() };
    assert_eq!(read(ENDED, &kept, None, millis(LATER)), Read::Quiet);
    assert_eq!(read(LATER, &kept, None, millis(LATER)), Read::Owed, "only that stop");
}

#[test]
fn the_stop_after_a_nudge_is_stopped_and_draws_no_second_nudge() {
    assert_eq!(read(LATER, &told(ENDED, false), None, millis(LATER)), Read::Stopped);
}

#[test]
fn a_nudge_has_its_minute_to_start_a_turn_and_then_the_job_is_stopped() {
    let within = millis(TOLD) + stopping::STOP_AFTER.as_millis() as i64 - 1;
    assert_eq!(read(ENDED, &told(ENDED, false), None, within), Read::Quiet);
    assert_eq!(read(ENDED, &told(ENDED, false), None, within + 2), Read::Stopped);
}

#[test]
fn a_nudge_that_reached_nobody_goes_to_stopped_at_once() {
    assert_eq!(read(ENDED, &told(ENDED, true), None, millis(TOLD)), Read::Stopped);
}

#[test]
fn his_message_after_the_nudge_gives_the_next_stop_a_nudge_of_its_own() {
    let spoke = "2026-11-03T10:02:00.000Z";
    assert_eq!(read(LATER, &told(ENDED, false), Some(spoke), millis(LATER)), Read::Owed);
    assert_eq!(read(LATER, &told(ENDED, false), Some(ENDED), millis(LATER)), Read::Stopped, "written before it: no");
}

/// A background command or a Check reads working between turns; an ask in its reply still raises its item.
#[test]
fn a_command_beside_the_turn_reads_working_only_where_it_would_rest() {
    use crate::session_host::standing::beside_the_turn;
    assert_eq!(beside_the_turn(SessionStanding::Idle, true), SessionStanding::Working);
    assert_eq!(beside_the_turn(SessionStanding::Replied, true), SessionStanding::Working);
    assert_eq!(beside_the_turn(SessionStanding::Done, true), SessionStanding::Done);
    assert_eq!(beside_the_turn(SessionStanding::Idle, false), SessionStanding::Idle);
}

async fn record(rig: &Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session
}

/// A Job whose turn ended on a reply, far enough back that its minute is up.
async fn stopped_a_while_ago(rig: &Rig) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, "go").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "It is written.");
    eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;
    aged(rig, &id).await;
    id
}

/// Its last turn ended, and the person last wrote, long ago: the row is moved by hand as `session_slot_choice` does.
async fn aged(rig: &Rig, id: &SessionId) {
    let long_ago = crate::clock::rfc3339_utc(rig.fleet.now().epoch_millis().unwrap() - 120_000);
    {
        let mut store = rig.fleet.store().lock().await;
        let mut row = store.session(id.as_str()).unwrap().expect("a row");
        row.last_turn_at = Some(long_ago);
        store.keep_session(&row).unwrap();
    }
}

fn nudges(rows: &[SessionRow]) -> usize {
    rows.iter().filter(|row| matches!(row, SessionRow::Message { notice: Some(word), .. } if word == "stop_nudge")).count()
}

/// The spoken message is younger than the minute, so the first read owes nothing until the minute is up from it too.
async fn tell_ripe(rig: &Rig, id: &SessionId) {
    let _ = record(rig, id).await;
    let stop = rig.fleet.store().lock().await.session(id.as_str()).unwrap().unwrap().last_turn_at.unwrap();
    rig.fleet.hosts().stops_owed().noted(id.as_str(), Some((&stop, 0)));
    rig.fleet.stops_told().await;
}

#[tokio::test]
async fn a_job_that_stops_is_told_once_and_stopping_again_reads_stopped_with_no_item() {
    let rig = rig_placing(None);
    let id = stopped_a_while_ago(&rig).await;

    tell_ripe(&rig, &id).await;
    assert_eq!(nudges(&rig.rows(&id).await), 1, "told once");
    rig.fleet.stops_told().await;
    assert_eq!(nudges(&rig.rows(&id).await), 1, "and not again for the same stop");
    let said = rig.stand_in.process(0).sent.lock().unwrap().iter().find(|line| line.contains("your turn ended")).cloned().unwrap();
    for part in ["Carry on now", "waiting_for", "finish_up", "A second stop"] {
        assert!(said.contains(part), "{part}: {said}");
    }

    // The word started a turn, and that turn ends with the agent saying nothing new.
    rig.stand_in.finishes(0, "Still written.");
    eventually(|| async { record(&rig, &id).await.standing == Some(SessionStanding::Stopped) }).await;
    let row = record(&rig, &id).await;
    let stopped = row.stopped.expect("why it stopped");
    assert_eq!(stopped.reason, StopReason::TurnEnded);
    assert!(stopped.gave_up, "a live agent: a repair Job has nothing to fix");
    assert!(row.waiting_for.is_empty(), "no item: the mark and the call reach him");

    rig.fleet.stops_told().await;
    assert_eq!(nudges(&rig.rows(&id).await), 1, "never a second nudge");
}

#[tokio::test]
async fn landing_passes_over_a_stop_and_his_stop_press_leaves_none() {
    let rig = rig_placing(None);
    let id = stopped_a_while_ago(&rig).await;
    Arc::clone(&rig.fleet).stop_session(StopSession { session_id: id.clone() }).await.expect("stopped");
    rig.fleet.stops_told().await;
    let row = record(&rig, &id).await;
    assert_eq!(nudges(&rig.rows(&id).await), 0);
    assert_ne!(row.standing, Some(SessionStanding::Stopped));
    assert!(row.stop_told_by.is_none(), "nothing coming");
}

#[tokio::test]
async fn an_item_for_him_or_a_live_watch_excuses_a_stop() {
    let rig = rig_placing(None);
    let id = stopped_a_while_ago(&rig).await;
    assert!(record(&rig, &id).await.stop_told_by.is_some(), "owed before either");

    Arc::clone(&rig.fleet)
        .set_waiting(
            None,
            SetWaitingFor {
                session_id: Some(id.clone()),
                items: vec![WaitingInput { id: "pick".into(), text: "Pick the name".into(), act: None, options: Vec::new(), steps: Vec::new() }],
            },
        )
        .await
        .expect("kept");
    assert!(record(&rig, &id).await.stop_told_by.is_none(), "it named what it waits on");
    Arc::clone(&rig.fleet)
        .set_waiting(None, SetWaitingFor { session_id: Some(id.clone()), items: Vec::new() })
        .await
        .expect("kept");
    aged(&rig, &id).await;
    assert!(record(&rig, &id).await.stop_told_by.is_some(), "owed again");

    let watch = StartWatch {
        session_id: Some(id.clone()),
        name: "QA deploy".to_string(),
        starts: WatchStarts::Command { command: vec!["scripts/deploy-state".to_string()], every: "60s".to_string(), gives_up_after: None },
        follows: None,
        set_aside: Vec::new(),
    };
    rig.fleet.start_watch(None, watch).await.expect("started");
    assert!(record(&rig, &id).await.stop_told_by.is_none(), "a live watch is a wait");
}

#[tokio::test]
async fn a_job_that_has_not_ended_a_turn_has_no_stop() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let row = record(&rig, &id).await;
    assert!(row.stop_told_by.is_none());
    assert!(row.stopped.is_none());
}
