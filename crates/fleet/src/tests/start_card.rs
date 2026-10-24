//! The claim, written before the code: **a Job started `held` waits on its start card, where the
//! person settles how the work lands, and nothing runs until Start; the landing he settles is the
//! answer to the delivery question, and what he usually picks is offered next time.**

use std::sync::Arc;

use adapter_traits::{CallDetail, DroneEvent};
use api::{HostedSessions, Refusal};
use ipc::{
    BeginSession, DeliveryChoice, DeliveryPicked, DeliveryPickedFrom, SessionId, SessionRecord, SessionRow, SetSessionWorkflow, StartHeld,
    WaitingItem, WaitingSource,
};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct};

use super::delivery_gate::workflow;
use super::session_host::{eventually, rig_fitted, Rig};
use crate::daemon::Fittings;
use crate::session_host::address_of;
use crate::session_host::delivering::delivery_of;

const FIRST: &str = "the log reader drops the last line of every file";

fn fitted(fittings: &mut Fittings<FakeHarness, FakeVcs, FakeWorkProduct>) {
    fittings.judge = Arc::new(FakeJudge::with_no_objection());
    fittings.starting().workflows = super::daemon::one(workflow());
}

fn rig() -> Rig {
    rig_fitted(fitted)
}

async fn record(rig: &Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session
}

async fn card(rig: &Rig, id: &SessionId) -> Option<StartHeld> {
    record(rig, id).await.hosted.and_then(|hosted| hosted.start)
}

async fn question(rig: &Rig, id: &SessionId) -> Option<WaitingItem> {
    record(rig, id).await.waiting_for.into_iter().find(|one| one.source == WaitingSource::Delivery)
}

fn sent(rig: &Rig) -> Vec<String> {
    if rig.stand_in.starts().is_empty() {
        return Vec::new();
    }
    rig.stand_in.process(0).sent.lock().unwrap().clone()
}

fn told(rig: &Rig, containing: &str) -> usize {
    sent(rig).iter().filter(|line| line.contains(containing)).count()
}

async fn begin(rig: &Rig, id: &SessionId, delivery: Option<DeliveryChoice>) -> Result<SessionRecord, Refusal> {
    Arc::clone(&rig.fleet).begin_session(BeginSession { session_id: id.clone(), delivery }).await
}

async fn on_gate(rig: &Rig, id: &SessionId) {
    Arc::clone(&rig.fleet)
        .set_session_workflow(SetSessionWorkflow { session_id: id.clone(), workflow: Some("gate".into()) })
        .await
        .expect("set");
}

/// A held Job on the gate workflow, with the rail watching, whose first message is waiting.
async fn held_with(rig: &Rig, first: &str) -> SessionId {
    crate::timeline::keep_timeline(Arc::clone(&rig.fleet));
    crate::rail_following::keep_the_rail(Arc::clone(&rig.fleet));
    let id = rig.start_held().await;
    on_gate(rig, &id).await;
    rig.send_unsettled(&id, first).await;
    id
}

fn manifest(rig: &Rig) -> String {
    rig.fleet.repositories().first().expect("served").manifest().id().as_str().to_string()
}

async fn picks(rig: &Rig, choices: &[DeliveryChoice]) {
    for choice in choices {
        let word = match choice {
            DeliveryChoice::AutoLand => "auto_land",
            DeliveryChoice::OpenPr => "open_pr",
            DeliveryChoice::KeepLocal => "keep_local",
        };
        rig.fleet.store().lock().await.keep_delivery_pick(&manifest(rig), "gate", word, "2026-10-23T09:00:00Z").unwrap();
    }
}

/// A fresh held Job's card, for what the picks so far make of it.
async fn offered(rig: &Rig) -> Option<DeliveryPicked> {
    let id = rig.start_held().await;
    on_gate(rig, &id).await;
    card(rig, &id).await.expect("held").picked
}

#[tokio::test]
async fn a_held_jobs_first_message_starts_no_agent_until_begin() {
    let rig = rig();
    let id = rig.start_held().await;
    assert!(card(&rig, &id).await.is_some(), "the row says it waits on its card");

    rig.send_unsettled(&id, FIRST).await;
    assert!(rig.stand_in.starts().is_empty(), "no agent runs before Start");
    assert!(
        rig.rows(&id).await.iter().any(|row| matches!(row, SessionRow::Message { text, .. } if text == FIRST)),
        "the message is on the thread"
    );

    let begun = begin(&rig, &id, None).await.expect("begun");
    assert!(begun.hosted.expect("hosted").start.is_none(), "the card is gone once it has begun");
    eventually(|| async { rig.stand_in.starts().len() == 1 && told(&rig, FIRST) == 1 }).await;
}

#[tokio::test]
async fn messages_sent_while_held_go_in_the_order_they_came() {
    let rig = rig();
    let id = rig.start_held().await;
    rig.send_unsettled(&id, "first words").await;
    rig.send_unsettled(&id, "second words").await;
    begin(&rig, &id, None).await.expect("begun");
    eventually(|| async { sent(&rig).len() == 2 }).await;
    let lines = sent(&rig);
    assert!(lines[0].contains("first words") && lines[1].contains("second words"), "{lines:?}");
}

#[tokio::test]
async fn a_message_sent_once_the_first_has_been_read_is_kept_too() {
    let rig = rig();
    let id = rig.start_held().await;
    rig.send_unsettled(&id, "first words").await;
    let runtime = rig.fleet.hosts().of(id.as_str());
    eventually(|| async { runtime.state().proposal.reading.is_none() }).await;
    rig.send_unsettled(&id, "second words").await;
    assert!(rig.stand_in.starts().is_empty(), "nothing is reading, and the Job still waits for Start");
    begin(&rig, &id, None).await.expect("begun");
    eventually(|| async { sent(&rig).len() == 2 }).await;
    let lines = sent(&rig);
    assert!(lines[0].contains("first words") && lines[1].contains("second words"), "{lines:?}");
}

#[tokio::test]
async fn a_job_that_is_not_held_starts_on_its_first_message_as_before() {
    let rig = rig();
    let id = rig.start().await;
    rig.send(&id, FIRST).await;
    assert_eq!(rig.stand_in.starts().len(), 1);
    assert!(card(&rig, &id).await.is_none());
}

#[tokio::test]
async fn after_begin_with_a_choice_entering_the_plan_step_asks_nothing_and_auto_land_follows_the_choice() {
    for (choice, auto_land) in [(DeliveryChoice::AutoLand, true), (DeliveryChoice::OpenPr, false), (DeliveryChoice::KeepLocal, false)] {
        let rig = rig();
        let id = held_with(&rig, FIRST).await;
        assert_eq!(card(&rig, &id).await.map(|one| one.asks_delivery), Some(true));
        begin(&rig, &id, Some(choice)).await.expect("begun");

        eventually(|| async { told(&rig, "Record the plan before editing.") == 1 }).await;
        assert!(question(&rig, &id).await.is_none(), "{choice:?}: no delivery item waits");
        assert_eq!(told(&rig, "choosing how this work is delivered"), 0, "{choice:?}: the agent is not stopped");
        assert!(!rig.fleet.delivery_entered(id.as_str()).await, "{choice:?}: entering the step asks nothing");
        assert_eq!(delivery_of(&rig.rows(&id).await), Some(choice));
        assert_eq!(record(&rig, &id).await.auto_land, auto_land, "{choice:?}");
        assert_eq!(told(&rig, &format!("the owner chose {}", choice.label())), 1, "the agent hears the choice with its first turn");
    }
}

#[tokio::test]
async fn a_landing_the_first_message_names_is_the_cards_and_is_not_said_again_to_the_agent() {
    let rig = rig();
    let id = held_with(&rig, "the log reader drops the last line of every file. Keep it local.").await;
    assert_eq!(
        card(&rig, &id).await.and_then(|one| one.picked),
        Some(DeliveryPicked { choice: DeliveryChoice::KeepLocal, from: DeliveryPickedFrom::Named })
    );
    begin(&rig, &id, Some(DeliveryChoice::KeepLocal)).await.expect("begun");
    eventually(|| async { told(&rig, "Record the plan before editing.") == 1 }).await;
    assert_eq!(told(&rig, "the owner chose"), 0, "the message already said it");
    assert!(rig.rows(&id).await.iter().any(|row| matches!(row, SessionRow::Delivery { named: true, .. })));
}

#[tokio::test]
async fn begin_without_a_landing_where_the_kind_asks_is_refused_and_leaves_the_job_held() {
    let rig = rig();
    let id = held_with(&rig, FIRST).await;
    let refused = begin(&rig, &id, None).await.expect_err("refused");
    assert_eq!(refused.error().code, "fleet.session_waiting_empty");
    assert!(card(&rig, &id).await.is_some(), "still held");
    assert!(rig.stand_in.starts().is_empty());
    assert!(rig.rows(&id).await.iter().all(|row| !matches!(row, SessionRow::Delivery { .. })));
}

#[tokio::test]
async fn begin_where_the_kind_asks_nothing_writes_no_delivery_row_and_ignores_a_landing() {
    let rig = rig();
    let id = rig.start_held().await;
    assert_eq!(card(&rig, &id).await.map(|one| one.asks_delivery), Some(false), "a custom Job asks nothing");
    rig.send_unsettled(&id, FIRST).await;
    begin(&rig, &id, Some(DeliveryChoice::AutoLand)).await.expect("begun");
    assert!(rig.rows(&id).await.iter().all(|row| !matches!(row, SessionRow::Delivery { .. })));
    assert!(!record(&rig, &id).await.auto_land, "a landing nobody asked for turns nothing on");
}

#[tokio::test]
async fn begin_on_a_job_that_is_not_held_is_refused_as_settled_already() {
    let rig = rig();
    let plain = rig.start().await;
    let refused = begin(&rig, &plain, None).await.expect_err("refused");
    assert!(matches!(refused, Refusal::IllegalMove(_)));
    assert_eq!(refused.error().code, "fleet.session_waiting_unheld");

    let held = rig.start_held().await;
    begin(&rig, &held, None).await.expect("begun");
    let again = begin(&rig, &held, None).await.expect_err("refused");
    assert_eq!(again.error().code, "fleet.session_waiting_unheld", "a second press finds it begun");
}

#[tokio::test]
async fn the_card_follows_a_workflow_change() {
    let rig = rig();
    let id = rig.start_held().await;
    assert_eq!(card(&rig, &id).await.map(|one| one.asks_delivery), Some(false));
    on_gate(&rig, &id).await;
    assert_eq!(card(&rig, &id).await.map(|one| one.asks_delivery), Some(true));
    Arc::clone(&rig.fleet)
        .set_session_workflow(SetSessionWorkflow { session_id: id.clone(), workflow: None })
        .await
        .expect("cleared");
    assert_eq!(card(&rig, &id).await.map(|one| one.asks_delivery), Some(false));
}

#[tokio::test]
async fn a_held_job_survives_a_fleet_restart_held_and_begin_then_delivers_the_first_message() {
    let rig = rig();
    let id = held_with(&rig, FIRST).await;
    let rig = rig.restarted(fitted);

    assert_eq!(card(&rig, &id).await.map(|one| one.asks_delivery), Some(true), "still on its card");
    assert!(rig.stand_in.starts().is_empty());
    rig.send_unsettled(&id, "and the second").await;
    assert!(rig.stand_in.starts().is_empty(), "a message after the restart is kept as well");

    begin(&rig, &id, Some(DeliveryChoice::OpenPr)).await.expect("begun");
    eventually(|| async { rig.stand_in.starts().len() == 1 && told(&rig, "and the second") == 1 }).await;
    assert_eq!(told(&rig, FIRST), 1, "the message the old Fleet held is delivered once");
    let lines = sent(&rig);
    assert!(lines[0].contains(FIRST) && lines[1].contains("and the second"), "in the order they came: {lines:?}");
}

#[tokio::test]
async fn begin_right_after_a_restart_delivers_the_first_message() {
    let rig = rig();
    let id = held_with(&rig, FIRST).await;
    let rig = rig.restarted(fitted);
    begin(&rig, &id, Some(DeliveryChoice::KeepLocal)).await.expect("begun");
    eventually(|| async { rig.stand_in.starts().len() == 1 && told(&rig, FIRST) == 1 }).await;
}

#[tokio::test]
async fn a_held_start_with_a_request_is_read_but_sends_nothing_until_begin() {
    let rig = rig_fitted(|fittings| {
        fitted(fittings);
        fittings.judge = Arc::new(FakeJudge::saying("workflow: gate\ntitle: Log reader drops a line"));
    });
    let id = rig.start_requesting(FIRST, |start| start.held = true).await.id;
    eventually(|| async { record(&rig, &id).await.proposal.is_none_or(|one| !one.reading) }).await;
    let read = record(&rig, &id).await;
    assert_eq!(read.workflow.as_deref(), Some("gate"));
    assert_eq!(read.title.as_deref(), Some("Log reader drops a line"));
    assert_eq!(card(&rig, &id).await.map(|one| one.asks_delivery), Some(true), "the card shows what was read");

    rig.send_unsettled(&id, FIRST).await;
    assert!(rig.stand_in.starts().is_empty(), "the reading ended and sent nothing");
    begin(&rig, &id, Some(DeliveryChoice::OpenPr)).await.expect("begun");
    eventually(|| async { rig.stand_in.starts().len() == 1 && told(&rig, FIRST) == 1 }).await;
}

#[tokio::test]
async fn three_auto_land_picks_make_auto_land_the_usual_and_two_make_none() {
    let rig = rig();
    for _ in 0..2 {
        let id = held_with(&rig, FIRST).await;
        begin(&rig, &id, Some(DeliveryChoice::AutoLand)).await.expect("begun");
    }
    assert_eq!(offered(&rig).await, None, "two picks are not yet a habit");
    let id = held_with(&rig, FIRST).await;
    begin(&rig, &id, Some(DeliveryChoice::AutoLand)).await.expect("begun");
    assert_eq!(offered(&rig).await, Some(DeliveryPicked { choice: DeliveryChoice::AutoLand, from: DeliveryPickedFrom::Usual }));
}

#[tokio::test]
async fn a_tie_goes_to_the_most_recent_pick() {
    use DeliveryChoice::{AutoLand, KeepLocal, OpenPr};
    let rig = rig();
    picks(&rig, &[AutoLand, AutoLand, OpenPr, OpenPr]).await;
    assert_eq!(offered(&rig).await.map(|one| one.choice), Some(OpenPr));
    picks(&rig, &[KeepLocal]).await;
    assert_eq!(offered(&rig).await.map(|one| one.choice), Some(OpenPr), "the latest pick, Keep local, trails the two that tie");
    picks(&rig, &[AutoLand]).await;
    assert_eq!(offered(&rig).await.map(|one| one.choice), Some(AutoLand), "three to two to one: the most frequent");
}

#[tokio::test]
async fn only_the_last_ten_picks_count() {
    use DeliveryChoice::{AutoLand, KeepLocal};
    let rig = rig();
    picks(&rig, &[KeepLocal; 8]).await;
    picks(&rig, &[AutoLand; 7]).await;
    assert_eq!(offered(&rig).await.map(|one| one.choice), Some(AutoLand), "seven to three in the last ten, though eight to seven in all");
}

#[tokio::test]
async fn the_usual_is_for_the_kind_of_work_and_the_repository_it_was_picked_in() {
    let rig = rig();
    picks(&rig, &[DeliveryChoice::AutoLand; 3]).await;
    let other = rig.start_held().await;
    assert_eq!(card(&rig, &other).await.and_then(|one| one.picked), None, "a custom Job has no kind, so no usual");
    rig.fleet
        .store()
        .lock()
        .await
        .keep_delivery_pick("another-repository", "gate", "keep_local", "2026-10-23T09:00:00Z")
        .unwrap();
    assert_eq!(offered(&rig).await.map(|one| one.choice), Some(DeliveryChoice::AutoLand));
}

#[tokio::test]
async fn what_the_first_message_names_beats_the_usual() {
    let rig = rig();
    picks(&rig, &[DeliveryChoice::AutoLand; 3]).await;
    let id = held_with(&rig, "the log reader drops the last line of every file. Open a PR.").await;
    assert_eq!(
        card(&rig, &id).await.and_then(|one| one.picked),
        Some(DeliveryPicked { choice: DeliveryChoice::OpenPr, from: DeliveryPickedFrom::Named })
    );
}

#[tokio::test]
async fn every_way_he_chooses_is_recorded_as_a_pick() {
    let rig = rig();
    crate::timeline::keep_timeline(Arc::clone(&rig.fleet));
    crate::rail_following::keep_the_rail(Arc::clone(&rig.fleet));

    // The card.
    let held = held_with(&rig, FIRST).await;
    begin(&rig, &held, Some(DeliveryChoice::OpenPr)).await.expect("begun");
    eventually(|| async { !rig.stand_in.starts().is_empty() }).await;

    // The question the plan step asks of a Job that was never held.
    let asked = rig.start().await;
    rig.send(&asked, FIRST).await;
    rig.stand_in.init(1);
    on_gate(&rig, &asked).await;
    eventually(|| async { question(&rig, &asked).await.is_some() }).await;
    Arc::clone(&rig.fleet)
        .answer_waiting(ipc::AnswerWaiting { session_id: asked.clone(), item_id: "delivery".into(), choice: Some(2), text: None, mode: None })
        .await
        .expect("answered");

    // A delivery the first message names.
    let named = rig.start().await;
    rig.send(&named, "the log reader drops the last line of every file. Land it.").await;
    rig.stand_in.init(2);
    on_gate(&rig, &named).await;
    eventually(|| async { delivery_of(&rig.rows(&named).await) == Some(DeliveryChoice::AutoLand) }).await;

    let kept = rig.fleet.store().lock().await.delivery_picks(&manifest(&rig), "gate", 10).unwrap();
    assert_eq!(kept, ["auto_land", "keep_local", "open_pr"], "newest first");
}

/// Job `a` tells `to`, with `SendMessage`, to "please review". `a` is the first process the stand-in starts.
async fn a_job_writes_to(rig: &Rig, to: &SessionId) -> SessionId {
    let a = rig.start().await;
    rig.send(&a, "hello a").await;
    rig.stand_in.init(0);
    rig.stand_in.says(
        0,
        vec![DroneEvent::Called {
            tool: "SendMessage".into(),
            call: "toolu_1".into(),
            detail: CallDetail::of(&format!("to {}: please review", address_of(to.as_str()))),
        }],
    );
    eventually(|| async { rig.rows(to).await.iter().filter(|row| matches!(row, SessionRow::Message { .. })).count() == 2 }).await;
    a
}

fn lines_of(rig: &Rig, process: usize) -> Vec<String> {
    rig.stand_in.process(process).sent.lock().unwrap().clone()
}

#[tokio::test]
async fn another_jobs_message_to_a_held_job_reaches_the_agent_once_after_begin_and_after_the_first_message() {
    let rig = rig();
    let held = rig.start_held().await;
    rig.send_unsettled(&held, FIRST).await;
    a_job_writes_to(&rig, &held).await;
    assert_eq!(rig.stand_in.starts().len(), 1, "only the Job that wrote has an agent; the held one is not woken");

    begin(&rig, &held, None).await.expect("begun");
    eventually(|| async { rig.stand_in.starts().len() == 2 && lines_of(&rig, 1).len() == 2 }).await;
    let lines = lines_of(&rig, 1);
    assert!(lines[0].contains(FIRST), "{lines:?}");
    assert!(lines[1].contains("please review") && lines[1].contains("A message from job"), "{lines:?}");
    assert_eq!(lines.iter().filter(|line| line.contains("please review")).count(), 1, "once");
}

#[tokio::test]
async fn another_jobs_message_to_a_held_job_survives_a_fleet_restart() {
    let rig = rig();
    let held = rig.start_held().await;
    rig.send_unsettled(&held, FIRST).await;
    a_job_writes_to(&rig, &held).await;
    let rig = rig.restarted(fitted);

    begin(&rig, &held, None).await.expect("begun");
    eventually(|| async { rig.stand_in.starts().len() == 1 && lines_of(&rig, 0).len() == 2 }).await;
    let lines = lines_of(&rig, 0);
    assert!(lines[0].contains(FIRST) && lines[1].contains("please review"), "in the order the rows were written: {lines:?}");
    assert_eq!(lines.iter().filter(|line| line.contains("please review")).count(), 1, "once");
}
