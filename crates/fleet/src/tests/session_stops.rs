//! The claim: **a Job that ends its turn with nothing running and nothing asked is told once to carry on or name
//! what it waits on, and if it stops again its row says Stopped.** Read from what Fleet saw, one nudge per stop,
//! so a Job can never loop with Fleet. `docs/concepts/session/what-a-job-is-doing.md`.

use std::sync::Arc;

use api::{HostedSessions, Watches};
use ipc::{SessionId, SessionRecord, SessionRow, SessionStanding, SetWaitingFor, StartWatch, StopReason, StopRepair, StopRestart, StopSession, WaitingInput, WatchStarts};
use store::KeptStop;

use crate::session_host::stopping::{self, read, Read};
use crate::session_host::Heard;

use super::session_host::{eventually, rig_placing, Rig};

const ENDED: &str = "2026-11-03T10:00:00.000Z";
const LATER: &str = "2026-11-03T10:03:00.000Z";
const TOLD: &str = "2026-11-03T10:01:00.000Z";

fn millis(at: &str) -> i64 {
    core_model::Timestamp::from_rfc3339(at).epoch_millis().unwrap()
}

fn told(stop: &str, unheard: bool) -> KeptStop {
    KeptStop { told: Some((stop.to_string(), TOLD.to_string())), unheard, ..KeptStop::default() }
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
pub(super) async fn stopped_a_while_ago(rig: &Rig) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, "go").await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "It is written.");
    eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;
    aged(rig, &id).await;
    id
}

/// Its last turn ended, and the person last wrote, long ago: the row is moved by hand as `session_slot_choice` does.
pub(super) async fn aged(rig: &Rig, id: &SessionId) {
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
pub(super) async fn tell_ripe(rig: &Rig, id: &SessionId) {
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
    for part in ["Carry on now", "waiting_for", "all_done", "A second stop"] {
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

fn spent() -> store::KeptOpenTurn {
    store::KeptOpenTurn { session_id: "a".into(), since: ENDED.into(), ended_at: Some(LATER.into()), started_again: 1 }
}

#[test]
fn a_turn_is_dead_only_once_its_one_restart_is_spent() {
    assert!(stopping::restart_spent(&spent()));
    assert!(!stopping::restart_spent(&store::KeptOpenTurn { started_again: 0, ..spent() }), "Fleet is on it");
    assert!(!stopping::restart_spent(&store::KeptOpenTurn { ended_at: None, ..spent() }), "its process spoke again");
}

#[test]
fn a_dead_job_says_where_fleet_stands_and_amber_only_once_it_has_run_out() {
    let dead = |kept: &KeptStop, over| stopping::dead(&spent(), kept, over, &[], None);
    let waiting = dead(&KeptStop::default(), false);
    assert_eq!((waiting.reason, waiting.restart, waiting.repair, waiting.gave_up), (StopReason::ProcessGone, StopRestart::Failed, None, false));
    let running = dead(&KeptStop { repair_job: Some("r".into()), ..KeptStop::default() }, false);
    assert_eq!((running.repair, running.gave_up), (Some(StopRepair::Running), false));
    let failed = dead(&KeptStop { repair_job: Some("r".into()), ..KeptStop::default() }, true);
    assert_eq!((failed.repair, failed.gave_up), (Some(StopRepair::Failed), true));
    let held = dead(&KeptStop { repair_held: true, ..KeptStop::default() }, false);
    assert_eq!((held.repair, held.gave_up), (Some(StopRepair::Held), true));
    let itself = dead(&KeptStop { repairs: Some("a".into()), ..KeptStop::default() }, false);
    assert_eq!((itself.repair, itself.gave_up), (None, true), "a repair Job gets no repair of its own");
}

#[test]
fn a_process_lost_while_fleet_was_away_says_so() {
    let away = [(ENDED.to_string(), LATER.to_string())];
    let dead = stopping::dead(&spent(), &KeptStop::default(), false, &away, Some("gone".into()));
    assert_eq!(dead.reason, StopReason::FleetDown);
    assert_eq!(dead.said.as_deref(), Some("gone"));
    assert!(dead.down.is_some());
}

/// A Job dead with its restart spent: two processes, each gone in the open turn.
async fn dead_job(rig: &Rig) -> SessionId {
    let id = rig.start().await;
    rig.send(&id, "go").await;
    eventually(|| async { rig.stand_in.starts().len() == 1 }).await;
    rig.stand_in.init(0);
    eventually(|| async { rig.fleet.store().lock().await.open_turn(id.as_str()).unwrap().is_some() }).await;
    rig.stand_in.hears(0, Heard::Gone);
    eventually(|| async { rig.stand_in.starts().len() == 2 }).await;
    rig.stand_in.init(1);
    rig.stand_in.hears(1, Heard::Gone);
    eventually(|| async { record(rig, &id).await.stopped.is_some() }).await;
    id
}

#[tokio::test]
async fn a_dead_job_reads_stopped_gets_one_repair_job_nested_under_it_and_no_nudge() {
    let rig = rig_placing(None);
    let id = dead_job(&rig).await;
    let row = record(&rig, &id).await;
    assert_eq!(row.standing, Some(SessionStanding::Stopped));
    assert_eq!(row.stopped.as_ref().map(|one| one.restart), Some(StopRestart::Failed));
    assert!(row.waiting_for.is_empty(), "no item");

    rig.fleet.stops_told().await;
    let row = record(&rig, &id).await;
    let stopped = row.stopped.as_ref().expect("still stopped");
    assert_eq!((stopped.repair, stopped.gave_up), (Some(StopRepair::Running), false));
    assert_eq!(nudges(&rig.rows(&id).await), 0, "a dead Job hears no nudge");

    let kept = rig.fleet.store().lock().await.session_stop(id.as_str()).unwrap();
    let repair = kept.repair_job.expect("a repair Job");
    let child = record(&rig, &SessionId::carried(repair.as_str())).await;
    assert!(child.title.as_deref().is_some_and(|title| title.starts_with("Repair: ")), "{:?}", child.title);
    assert!(child.attachments.iter().any(|one| one.kind == "started_by" && one.target == id.as_str()), "nested under it");
    assert!(!rig.fleet.store().lock().await.hosting(&repair).unwrap().unwrap().start_held, "no start card: he presses nothing");
    eventually(|| async { rig.stand_in.starts().len() == 3 }).await;

    rig.fleet.stops_told().await;
    assert_eq!(rig.fleet.store().lock().await.session_stop(id.as_str()).unwrap().repair_job.as_deref(), Some(repair.as_str()), "one");
    let started = rig.stand_in.starts().len();
    rig.fleet.stops_told().await;
    assert_eq!(rig.stand_in.starts().len(), started, "and no second");

    // It ended and the Job is still dead: Fleet has run out, and the row reaches him.
    super::session_queue::close(&rig, &SessionId::carried(repair.as_str())).await;
    let stopped = record(&rig, &id).await.stopped.expect("still stopped");
    assert_eq!((stopped.repair, stopped.gave_up), (Some(StopRepair::Failed), true));
}
