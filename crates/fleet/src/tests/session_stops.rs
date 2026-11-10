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
                items: vec![WaitingInput { approves: false, id: "pick".into(), text: "Pick the name".into(), act: None, options: Vec::new(), steps: Vec::new() }],
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

/// His words: "if there is a needs me item, thats not silence. Thats waiting on me to act."
#[tokio::test]
async fn a_question_that_lapsed_is_him_to_act_and_no_stop() {
    let rig = rig_placing(None);
    let id = stopped_a_while_ago(&rig).await;
    assert!(record(&rig, &id).await.stop_told_by.is_some(), "owed before either");

    super::session_waiting::lapsed_clock_question(&rig, &id).await;
    let row = record(&rig, &id).await;
    assert!(row.waiting_for.is_empty(), "the lapsed question is no listed item");
    assert!(row.stop_told_by.is_none(), "yet it waits on him");
    tell_ripe(&rig, &id).await;
    assert_eq!(nudges(&rig.rows(&id).await), 0, "never nudged while it stands");
}

#[tokio::test]
async fn a_shown_walk_is_him_to_act_and_no_stop() {
    let rig = rig_placing(None);
    let id = stopped_a_while_ago(&rig).await;
    assert!(record(&rig, &id).await.stop_told_by.is_some(), "owed before it");
    super::session_waiting::show(&rig, &id, "http://localhost:5187/?walk=anything").await;
    assert!(record(&rig, &id).await.stop_told_by.is_none(), "a shown walk waits on him");
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

/// The repair is held at its start until a test releases it, so the row can be read while it runs.
fn holding(rig: &Rig) {
    std::fs::write(rig.repair_state.join("hold"), "").expect("held");
}

fn released(rig: &Rig) {
    std::fs::write(rig.repair_state.join("release"), "").expect("released");
}

fn fleet_notes(rows: &[SessionRow]) -> Vec<String> {
    rows.iter()
        .filter_map(|row| match row {
            SessionRow::Message { from: ipc::SessionVoice::Session { title, .. }, text, .. } if title == "Fleet" => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn a_dead_job_gets_one_hidden_repair_that_is_no_job_and_no_nudge() {
    let rig = rig_placing(None);
    holding(&rig);
    let id = dead_job(&rig).await;
    let row = record(&rig, &id).await;
    assert_eq!(row.standing, Some(SessionStanding::Stopped));
    assert_eq!(row.stopped.as_ref().map(|one| one.restart), Some(StopRestart::Failed));
    assert!(row.waiting_for.is_empty(), "no item");
    let live = rig.fleet.store().lock().await.live_session_ids().unwrap();

    rig.fleet.stops_told().await;
    let row = record(&rig, &id).await;
    let stopped = row.stopped.as_ref().expect("still stopped");
    assert_eq!((stopped.repair, stopped.gave_up), (Some(StopRepair::Running), false));
    assert_eq!(nudges(&rig.rows(&id).await), 0, "a dead Job hears no nudge");

    let store = rig.fleet.store().lock().await;
    let repair = store.session_stop(id.as_str()).unwrap().repair_job.expect("a repair");
    assert_eq!(store.live_session_ids().unwrap(), live, "no Job was started for it");
    assert!(store.session(&repair).unwrap().is_none(), "it has no session row");
    let kept = store.repair(&repair).unwrap().expect("kept");
    assert_eq!((kept.session_id.as_str(), kept.broke.as_str(), kept.ended_at), (id.as_str(), "died", None));
    drop(store);
    assert!(fleet_notes(&rig.rows(&id).await).contains(&"Repair: its process died twice".to_string()));
    assert!(rig.fleet.repairs().running_for(id.as_str()), "listed as running");
    eventually(|| async { super::repairing::turns(&rig.repair_state).len() == 1 }).await;
    assert!(super::repairing::turns(&rig.repair_state)[0].contains(id.as_str()), "told which Job");
    assert_eq!(rig.stand_in.starts().len(), 2, "no process of the Jobs host");

    rig.fleet.stops_told().await;
    rig.fleet.stops_told().await;
    assert_eq!(rig.fleet.store().lock().await.session_stop(id.as_str()).unwrap().repair_job.as_deref(), Some(repair.as_str()), "one");
    assert_eq!(super::repairing::turns(&rig.repair_state).len(), 1, "and no second");

    // It put things right: Fleet says so in the thread and wakes the Job once.
    released(&rig);
    eventually(|| async { rig.stand_in.starts().len() == 3 }).await;
    let notes = fleet_notes(&rig.rows(&id).await);
    assert!(notes.contains(&"Repaired: it carries on".to_string()), "{notes:?}");
    eventually(|| async { rig.stand_in.process(2).sent.lock().unwrap().iter().any(|line| line.contains("Fleet: a repair fixed what stopped its process. Carry on.")) }).await;
    let kept = rig.fleet.store().lock().await.repair(&repair).unwrap().expect("kept");
    assert_eq!((kept.outcome.as_deref(), kept.said.as_deref()), (Some("repaired"), Some("it carries on")));
    assert!(!rig.fleet.repairs().running_for(id.as_str()));
}

#[tokio::test]
async fn a_repair_that_did_not_repair_leaves_the_row_amber_and_wakes_nobody() {
    let rig = rig_placing(None);
    std::fs::write(rig.repair_state.join("answer"), "NOT REPAIRED: its binary is gone").expect("an answer");
    let id = dead_job(&rig).await;
    rig.fleet.stops_told().await;
    eventually(|| async { fleet_notes(&rig.rows(&id).await).contains(&"Not repaired: its binary is gone".to_string()) }).await;
    let stopped = record(&rig, &id).await.stopped.expect("still stopped");
    assert_eq!((stopped.repair, stopped.gave_up), (Some(StopRepair::Failed), true));
    assert_eq!(rig.stand_in.starts().len(), 2, "the Job was not woken");
    let repair = rig.fleet.store().lock().await.session_stop(id.as_str()).unwrap().repair_job.expect("a repair");
    assert_eq!(rig.fleet.store().lock().await.repair(&repair).unwrap().unwrap().outcome.as_deref(), Some("not_repaired"));
    rig.fleet.stops_told().await;
    assert_eq!(super::repairing::turns(&rig.repair_state).len(), 1, "Fleet has run out: no second repair");
}

#[tokio::test]
async fn a_repair_that_ends_with_neither_marker_is_not_repaired() {
    let rig = rig_placing(None);
    std::fs::write(rig.repair_state.join("answer"), "I looked at it and it is fine.").expect("an answer");
    let id = dead_job(&rig).await;
    rig.fleet.stops_told().await;
    eventually(|| async { fleet_notes(&rig.rows(&id).await).iter().any(|note| note.starts_with("Not repaired: ")) }).await;
    assert_eq!(record(&rig, &id).await.stopped.expect("stopped").repair, Some(StopRepair::Failed));
    assert_eq!(rig.stand_in.starts().len(), 2);
}

#[tokio::test]
async fn a_jobs_next_turn_or_its_end_stops_a_repair_still_running() {
    let rig = rig_placing(None);
    holding(&rig);
    let id = dead_job(&rig).await;
    rig.fleet.stops_told().await;
    let repair = rig.fleet.store().lock().await.session_stop(id.as_str()).unwrap().repair_job.expect("a repair");
    eventually(|| async { super::repairing::turns(&rig.repair_state).len() == 1 }).await;

    rig.fleet.turn_opened(id.as_str(), rig.fleet.now().as_str()).await;
    eventually(|| async { rig.fleet.store().lock().await.repair(&repair).unwrap().unwrap().outcome.is_some() }).await;
    let kept = rig.fleet.store().lock().await.repair(&repair).unwrap().unwrap();
    assert_eq!((kept.outcome.as_deref(), kept.said.as_deref()), (Some("stopped"), Some("the job moved on first")));
    assert!(fleet_notes(&rig.rows(&id).await).contains(&"Not repaired: the job moved on first".to_string()));
    assert_eq!(rig.fleet.store().lock().await.session_stop(id.as_str()).unwrap().repair_job, None, "its next death gets a repair of its own");
}

#[tokio::test]
async fn closing_a_job_stops_the_repair_running_for_it() {
    let rig = rig_placing(None);
    holding(&rig);
    let id = dead_job(&rig).await;
    rig.fleet.stops_told().await;
    let repair = rig.fleet.store().lock().await.session_stop(id.as_str()).unwrap().repair_job.expect("a repair");
    eventually(|| async { super::repairing::turns(&rig.repair_state).len() == 1 }).await;
    super::session_queue::close(&rig, &id).await;
    eventually(|| async { rig.fleet.store().lock().await.repair(&repair).unwrap().unwrap().outcome.as_deref() == Some("stopped") }).await;
}

#[tokio::test]
async fn a_subagent_still_working_excuses_a_stop() {
    let rig = rig_placing(None);
    let id = stopped_a_while_ago(&rig).await;
    assert!(record(&rig, &id).await.stop_told_by.is_some(), "owed before it");

    let at = rig.fleet.now().as_str().to_string();
    let subagent = store::KeptAttachment {
        holder: store::Holder::session(id.as_str()),
        kind: "subagent".into(),
        manifest_id: String::new(),
        target: "x1".into(),
        state: store::AttachmentState::Standing,
        detail: [("type", "explorer"), ("description", "sweep")].into_iter().map(|(key, value)| (key.to_string(), value.to_string())).collect(),
        since: at.clone(),
        changed_at: at,
    };
    rig.fleet.store().lock().await.attach(&subagent, false).expect("attached");
    let row = record(&rig, &id).await;
    assert!(row.stop_told_by.is_none(), "its turn ended on work running, not on nothing");
    assert_eq!(row.running.len(), 1);
    assert_eq!((row.running[0].kind, row.running[0].what.as_str()), (ipc::RunningKind::Subagent, "sweep"));
}

/// **A repair never runs in the repository's own checkout.** A dead Job that holds no slot has one
/// leased for its repair, held under the repair's id while it runs and given back when it ends.
#[tokio::test]
async fn a_dead_job_holding_no_slot_has_its_repair_in_a_leased_slot_and_the_slot_is_given_back() {
    let rig = rig_placing(None);
    holding(&rig);
    let id = dead_job(&rig).await;
    rig.fleet.stops_told().await;
    eventually(|| async { super::repairing::turns(&rig.repair_state).len() == 1 }).await;
    let repair = rig.fleet.store().lock().await.session_stop(id.as_str()).unwrap().repair_job.expect("a repair");

    let ran_in = super::repairing::directories(&rig.repair_state).remove(0);
    let canonical = |path: &str| std::fs::canonicalize(path).expect("exists").to_string_lossy().to_string();
    assert_ne!(ran_in, canonical(&rig.root), "not the repository's own checkout");
    assert_eq!(ran_in, canonical(&adapter_traits::slot_path(&rig.root, 1)), "a slot of the pool");
    assert_eq!(rig.fleet.vcs().slot_holders(&rig.root)[0].as_deref(), Some(repair.as_str()), "held under the repair's id");
    assert_eq!(rig.fleet.store().lock().await.repair(&repair).unwrap().unwrap().slot, Some(1), "kept, for a Fleet that restarts");

    released(&rig);
    eventually(|| async { rig.fleet.store().lock().await.repair(&repair).unwrap().unwrap().outcome.is_some() }).await;
    assert_eq!(rig.fleet.vcs().released_slots(), vec![(1, repair.clone())], "given back once");
    assert_eq!(rig.fleet.vcs().slot_holders(&rig.root)[0], None);
    assert_eq!(rig.fleet.store().lock().await.repair(&repair).unwrap().unwrap().slot, None);
    let gone = rig.fleet.vcs().deleted_branches();
    assert!(gone.len() == 1 && gone[0].starts_with("armada/repair-"), "the branch cut for it goes with it: {gone:?}");
}

/// A repair a Fleet left open when it stopped had its slot leased, and nothing runs it now: the slot is
/// given back at boot, before the repair is ended as abandoned.
#[tokio::test]
async fn a_slot_a_repair_held_when_fleet_stopped_is_given_back_at_boot() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    {
        let mut store = rig.fleet.store().lock().await;
        store.keep_repair_started(id.as_str(), "a-repair", "died", ENDED).unwrap();
        store.keep_repair_slot("a-repair", 1, false).unwrap();
    }
    rig.fleet.vcs().hold_slot(&rig.root, 1, "a-repair");

    rig.fleet.open_turns_started().await;

    assert_eq!(rig.fleet.vcs().released_slots(), vec![(1, "a-repair".to_string())]);
    assert_eq!(rig.fleet.vcs().slot_holders(&rig.root)[0], None);
    let kept = rig.fleet.store().lock().await.repair("a-repair").unwrap().unwrap();
    assert_eq!((kept.outcome.as_deref(), kept.slot), (Some("abandoned"), None));
}
