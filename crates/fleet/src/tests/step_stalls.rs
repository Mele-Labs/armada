//! The claim: **a Job that stands on a workflow step past the time 9 in 10 Jobs take is flagged on its row, and told
//! once for each visit to the step what the step waits on.** The usual time is measured from the visits Fleet kept, and
//! is absent below five. The word is the only one Fleet sends a Job about its step, and the owner asked for it
//! (20 Nov 2026).

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use core_model::Timestamp;
use ipc::{SessionId, SessionRecord, SessionRow, SessionStanding};
use store::{KeptHosting, KeptRail};

use super::planted::Held;
use super::session_host::{eventually, rig_fitted, Rig};
use super::session_rail::{approved_plan, on_the_rail, plan, workflow};
use crate::rail::{Facts, RailStep};
use crate::step_stalls::{cause_of, nine_in_ten, nudge_told, spoken, stalled_since, QUIET_SECS};

#[test]
fn the_usual_time_is_the_ninetieth_percentile_and_absent_below_five_visits() {
    assert_eq!(nine_in_ten(vec![10, 20, 30, 40]), None, "four visits say nothing");
    assert_eq!(nine_in_ten(vec![50, 10, 40, 20, 30]), Some(50), "five: the nearest rank of nine in ten is the last");
    let ten: Vec<u64> = (1..=10).map(|n| n * 60).collect();
    assert_eq!(nine_in_ten(ten), Some(540), "ten: the ninth, so one slow visit does not move it");
    assert_eq!(nine_in_ten((1..=100).collect()), Some(90));
}

#[test]
fn a_step_is_past_its_time_once_the_usual_after_it_began_has_gone_by() {
    let at = |text: &str| Timestamp::from_rfc3339(text);
    let began = "2026-11-20T10:00:00.000Z";
    assert_eq!(stalled_since(began, 600, None, &at("2026-11-20T10:10:00.000Z")), None, "on the minute is not past it");
    assert_eq!(stalled_since(began, 600, None, &at("2026-11-20T10:10:01.000Z")).as_deref(), Some("2026-11-20T10:10:00.000Z"));
}

#[test]
fn a_step_he_wrote_on_stays_plain_until_he_has_been_quiet_half_an_hour() {
    let at = |text: &str| Timestamp::from_rfc3339(text);
    let began = "2026-11-20T10:00:00.000Z";
    let wrote = Some("2026-11-20T10:50:00.000Z");
    assert_eq!(QUIET_SECS, 1800);
    assert_eq!(stalled_since(began, 600, wrote, &at("2026-11-20T11:20:00.000Z")), None, "half an hour after his message is not past it");
    assert_eq!(stalled_since(began, 600, wrote, &at("2026-11-20T11:20:01.000Z")).as_deref(), Some("2026-11-20T11:20:00.000Z"));
    let before = Some("2026-11-20T09:59:00.000Z");
    assert_eq!(stalled_since(began, 600, before, &at("2026-11-20T10:10:01.000Z")).as_deref(), Some("2026-11-20T10:10:00.000Z"), "a message before the step began is not on it");
}

#[test]
fn the_cause_says_what_the_step_waits_on_and_whose_move_it_is() {
    let waiting = [crate::tests::session_rail::plan_step()];
    let recorded = Facts { plan_recorded: true, ..Facts::default() };
    assert_eq!(cause_of(&waiting, 0, &recorded, true), "Waiting on you. The step ends when you approve the plan.");
    assert_eq!(cause_of(&waiting, 0, &recorded, false), "Waiting on the Job. The step ends when you approve the plan.", "the Job is still working");
    assert_eq!(
        cause_of(&waiting, 0, &Facts::default(), true),
        "Waiting on the Job. It has recorded no plan for you to approve. The step ends when you approve the plan.",
        "nothing to press yet"
    );
    let nothing = [crate::tests::session_rail::delivers("Ship"), crate::tests::session_rail::step("Retro")];
    assert_eq!(cause_of(&nothing, 1, &Facts::default(), true), "Waiting on the Job. The step ends when the Job ends.");
}

#[test]
fn the_nudge_names_the_step_the_time_and_what_it_waits_on_and_says_how_to_ask_for_the_press() {
    let step = RailStep { label: "Sign off".into(), ..crate::tests::session_rail::step("Sign off") };
    let plain = nudge_told("feature", &step, 3 * 3600 + 120, 1320, "Waiting on you. The step ends when you approve.", false);
    assert_eq!(
        plain,
        "Fleet: this Job has been on the \"Sign off\" step of its feature workflow for 3h 2m, longer than the 22m that 9 in 10 Jobs take. Waiting on you. The step ends when you approve."
    );
    let asking = nudge_told("feature", &step, 3600, 1320, "Waiting on you.", true);
    assert!(asking.contains("waiting_for item with approves: true") && asking.contains("only his press ends the step"), "{asking}");
    assert_eq!((spoken(45), spoken(120), spoken(7200), spoken(19800)), ("45s".into(), "2m".into(), "2h".into(), "5h 30m".into()));
}

// ---- A Fleet ----

fn nudges(rows: &[SessionRow]) -> usize {
    rows.iter().filter(|row| matches!(row, SessionRow::Message { notice: Some(word), .. } if word == "rail_nudge")).count()
}

async fn record(rig: &Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session
}

/// Five other Jobs on this Job's workflow, each of which stayed `seconds` on its first step.
async fn others_stayed(rig: &Rig, like: &SessionId, seconds: i64) {
    let mut store = rig.fleet.store().lock().await;
    let base = store.hosting(like.as_str()).unwrap().expect("hosted");
    let steps: Vec<String> = ["plan", "implement", "handoff"].iter().map(|one| one.to_string()).collect();
    let stamp = |secs: i64| crate::clock::rfc3339_utc(1_780_000_000_000 + secs * 1000);
    for n in 0..5 {
        let id = format!("other-{n}");
        store.keep_hosting(&KeptHosting { session_id: id.clone(), ..base.clone() }).unwrap();
        let visits = vec![(0, stamp(0)), (1, stamp(seconds))];
        store.keep_session_rail(&id, &KeptRail { step: Some(1), visits, ..KeptRail::default() }, &steps).unwrap();
    }
}

fn clocked() -> (Rig, Arc<Held>) {
    let held = Arc::new(Held::started());
    let clock = Arc::clone(&held);
    let rig = rig_fitted(move |fittings| {
        fittings.clock = clock;
        fittings.starting().workflows = super::daemon::one(workflow());
    });
    (rig, held)
}

#[tokio::test]
async fn the_strip_carries_the_steps_tabs_usual_time_and_end_and_goes_amber_past_it_once_told() {
    let (rig, held) = clocked();
    let id = on_the_rail(&rig).await;
    rig.stand_in.finishes(0, "Reading the code.");
    eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;

    let first = record(&rig, &id).await.workflow_strip.expect("a strip for a Job on a step");
    assert_eq!(first.tabs, ["plan", "qa", "now"], "the step's tabs, front first");
    assert_eq!(first.usual_secs, None, "no Job has left the step yet: below five visits it says nothing");
    assert_eq!(first.ends_when, ["you approve the plan"]);
    assert!(first.stalled.is_none() && first.passed_unapproved.is_empty());

    others_stayed(&rig, &id, 3600).await;
    held.on(700);
    let measured = record(&rig, &id).await.workflow_strip.expect("a strip");
    assert_eq!(measured.usual_secs, Some(3600), "read again after ten minutes");
    assert!(measured.stalled.is_none(), "700 seconds in is inside the hour");

    held.on(4000);
    let late = record(&rig, &id).await.workflow_strip.expect("a strip");
    let stalled = late.stalled.expect("past the hour with no move");
    assert_eq!(stalled.cause, "Waiting on the Job. It has recorded no plan for you to approve. The step ends when you approve the plan.");
    let visit = record(&rig, &id).await.workflow_visits.last().cloned().expect("a visit");
    assert_eq!(Timestamp::from_rfc3339(stalled.since.as_str()).epoch_millis(), Some(Timestamp::from_rfc3339(visit.since.as_str()).epoch_millis().unwrap() + 3_600_000));

    assert_eq!(record(&rig, &id).await.standing, Some(SessionStanding::Replied), "the Drone finished its turn, so it can be told");
    rig.fleet.steps_stalled_told().await;
    assert_eq!(nudges(&rig.rows(&id).await), 1, "told once");
    rig.fleet.steps_stalled_told().await;
    assert_eq!(nudges(&rig.rows(&id).await), 1, "never again for the same visit");
    let said = rig.stand_in.process(0).sent.lock().unwrap().iter().find(|line| line.contains("9 in 10 Jobs")).cloned().expect("the word");
    assert!(said.contains("Plan the change") && said.contains("Waiting on the Job"), "{said}");
}

#[tokio::test]
async fn the_move_is_his_once_a_plan_is_recorded_and_a_step_he_writes_on_stays_plain() {
    let (rig, held) = clocked();
    let id = on_the_rail(&rig).await;
    rig.stand_in.finishes(0, "Reading the code.");
    eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;
    others_stayed(&rig, &id, 3600).await;
    held.on(4000);
    rig.fleet.record_plan(None, plan(&id), false).await.expect("recorded");
    let stalled = record(&rig, &id).await.workflow_strip.and_then(|strip| strip.stalled).expect("past the hour");
    assert_eq!(stalled.cause, "Waiting on you. The step ends when you approve the plan.");

    rig.send_unsettled(&id, "Try the other layout.").await;
    let strip = record(&rig, &id).await.workflow_strip.expect("a strip");
    assert!(strip.stalled.is_none(), "he wrote on the step just now: {strip:?}");
    held.on(QUIET_SECS as u64 + 60);
    assert!(record(&rig, &id).await.workflow_strip.and_then(|strip| strip.stalled).is_some(), "quiet for half an hour: amber again");
}

#[tokio::test]
async fn a_step_inside_its_time_is_told_nothing_and_leaving_it_keeps_nothing_told() {
    let (rig, held) = clocked();
    let id = on_the_rail(&rig).await;
    rig.stand_in.finishes(0, "Reading the code.");
    eventually(|| async { rig.turn(&id).await == ipc::SessionTurn::Idle }).await;
    others_stayed(&rig, &id, 3600).await;
    held.on(700);
    let _ = record(&rig, &id).await;
    rig.fleet.steps_stalled_told().await;
    assert_eq!(nudges(&rig.rows(&id).await), 0, "inside its time: no word");

    approved_plan(&rig, &id).await;
    assert_eq!(record(&rig, &id).await.workflow_step, Some(1), "he approved: the next step");
    let kept = rig.fleet.store().lock().await.session_rail(id.as_str(), &["plan".to_string(), "implement".to_string(), "handoff".to_string()]).unwrap();
    assert!(kept.stalled.is_empty(), "nothing was told for this step: {kept:?}");
}

// ---- A Job that waits on another has not begun ----

/// A Job on the rail workflow made to wait for `on` to land its pull request.
async fn waits_on(rig: &Rig, on: &SessionId) -> SessionId {
    let start = ipc::StartSession {
        project: None,
        manifest_id: rig.manifest.clone(),
        title: None,
        model: None,
        effort: None,
        mode: None,
        fork: None,
        root: None,
        request: Some("Build the reader once the other lands.".into()),
        after: Some(ipc::StartAfter { session: on.clone(), condition: ipc::StartCondition::PrLands, base: ipc::StartBase::Main, needs: String::new() }),
        held: false,
        project_rows: Vec::new(),
        on_your_words: None,
        workflow: Some("rail".into()),
    };
    Arc::clone(&rig.fleet).start_session_reading(start).await.expect("made to wait").id
}

#[tokio::test]
async fn a_job_waiting_on_another_is_never_stalled_or_told_and_its_step_begins_at_its_first_turn() {
    let (rig, held) = clocked();
    let on = on_the_rail(&rig).await;
    rig.stand_in.finishes(0, "Reading the code.");
    eventually(|| async { rig.turn(&on).await == ipc::SessionTurn::Idle }).await;
    let waiting = waits_on(&rig, &on).await;
    // A step kept while it waited, as Fleet kept one before a waiting Job kept none.
    let steps: Vec<String> = ["plan", "implement", "handoff"].iter().map(|one| one.to_string()).collect();
    let dispatched = rig.fleet.now().as_str().to_string();
    let planted = KeptRail { step: Some(0), visits: vec![(0, dispatched.clone())], ..KeptRail::default() };
    rig.fleet.store().lock().await.keep_session_rail(waiting.as_str(), &planted, &steps).unwrap();
    others_stayed(&rig, &on, 3600).await;
    held.on(4000);

    let still = record(&rig, &waiting).await;
    assert!(still.waits.is_some(), "it still waits");
    assert!(still.workflow_strip.and_then(|strip| strip.stalled).is_none(), "a Job that has not begun is not late");
    rig.fleet.steps_stalled_told().await;
    let told = rig.fleet.told_by_fleet(waiting.as_str(), "Fleet: a word for it.", ipc::Notice::RailNudge).await;
    assert!(matches!(told, Ok(false)), "Fleet's word does not reach it: {told:?}");
    assert_eq!(nudges(&rig.rows(&waiting).await), 0, "no nudge");
    assert_eq!(rig.stand_in.starts().len(), 1, "and no process started for it");

    Arc::clone(&rig.fleet).start_waiting_now(ipc::StartWaitingNow { session_id: waiting.clone() }).await.expect("started");
    eventually(|| async { rig.stand_in.starts().len() == 2 }).await;
    let began = record(&rig, &waiting).await.workflow_visits.last().cloned().expect("its step began");
    let ms = |at: &str| Timestamp::from_rfc3339(at).epoch_millis().unwrap();
    assert!(ms(began.since.as_str()) >= ms(&dispatched) + 4_000_000, "the step began at its first turn, not when it was made: {began:?}");
    assert!(record(&rig, &waiting).await.workflow_strip.and_then(|strip| strip.stalled).is_none(), "so it starts inside its time");
}
