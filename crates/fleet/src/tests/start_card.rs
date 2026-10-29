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
use super::workflow_off::{back_on, off, saved};
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
    Arc::clone(&rig.fleet).begin_session(BeginSession { session_id: id.clone(), delivery, message: None }).await
}

/// Start pressed with the box as he left it.
async fn begin_edited(rig: &Rig, id: &SessionId, delivery: Option<DeliveryChoice>, message: &str) -> Result<SessionRecord, Refusal> {
    Arc::clone(&rig.fleet)
        .begin_session(BeginSession { session_id: id.clone(), delivery, message: Some(message.to_string()) })
        .await
}

/// The messages on the thread, by their words.
async fn said_on_the_thread(rig: &Rig, id: &SessionId) -> Vec<String> {
    rig.rows(id)
        .await
        .into_iter()
        .filter_map(|row| match row {
            SessionRow::Message { text, .. } => Some(text),
            _ => None,
        })
        .collect()
}

/// The reading the first message began has landed, and with it any default.
async fn read(rig: &Rig, id: &SessionId) {
    eventually(|| async { record(rig, id).await.proposal.is_none_or(|one| !one.reading) }).await;
}

async fn on_gate(rig: &Rig, id: &SessionId) {
    Arc::clone(&rig.fleet)
        .set_session_workflow(SetSessionWorkflow { session_id: id.clone(), workflow: Some("gate".into()) })
        .await
        .expect("set");
}

/// A held Job on the gate workflow, with the rail watching, whose first message is waiting and read:
/// Start pressed during the reading is queued (`held_children.rs`), which these tests do not ask about.
async fn held_with(rig: &Rig, first: &str) -> SessionId {
    crate::timeline::keep_timeline(Arc::clone(&rig.fleet));
    crate::rail_following::keep_the_rail(Arc::clone(&rig.fleet));
    let id = rig.start_held().await;
    on_gate(rig, &id).await;
    rig.send_unsettled(&id, first).await;
    read(rig, &id).await;
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
async fn a_held_jobs_first_message_waits_off_the_thread_in_his_box_until_begin() {
    let rig = rig();
    let id = rig.start_held().await;
    assert!(card(&rig, &id).await.is_some(), "the row says it waits on its card");

    rig.send_unsettled(&id, FIRST).await;
    assert!(rig.stand_in.starts().is_empty(), "no agent runs before Start");
    assert!(said_on_the_thread(&rig, &id).await.is_empty(), "the first message is not on the thread");
    assert_eq!(card(&rig, &id).await.and_then(|one| one.message).as_deref(), Some(FIRST), "it is the card's");

    read(&rig, &id).await;
    let begun = begin(&rig, &id, Some(DeliveryChoice::OpenPr)).await.expect("begun");
    assert!(begun.hosted.expect("hosted").start.is_none(), "the card is gone once it has begun");
    eventually(|| async { rig.stand_in.starts().len() == 1 && told(&rig, FIRST) == 1 }).await;
    assert_eq!(said_on_the_thread(&rig, &id).await, [FIRST], "Start writes it to the thread");
}

#[tokio::test]
async fn begin_with_his_edit_sends_his_words_and_keeps_the_picture() {
    let rig = rig();
    let id = rig.start_held().await;
    Arc::clone(&rig.fleet)
        .send_session_message(ipc::SendSessionMessage {
            session_id: id.clone(),
            text: FIRST.into(),
            attachments: vec![ipc::SessionUpload { name: "shot.png".into(), media_type: "image/png".into(), data: "aGVsbG8=".into() }],
            mentions: Vec::new(),
        })
        .await
        .expect("taken");

    begin_edited(&rig, &id, Some(DeliveryChoice::OpenPr), "  the reader drops blank lines too  ").await.expect("begun");
    eventually(|| async { told(&rig, "the reader drops blank lines too") == 1 }).await;
    assert_eq!(told(&rig, FIRST), 0, "what he replaced is not sent");
    assert_eq!(told(&rig, "aGVsbG8="), 1, "the picture goes with his words");
    let rows = rig.rows(&id).await;
    let kept = rows.iter().find_map(|row| match row {
        SessionRow::Message { from: ipc::SessionVoice::You, text, files, .. } => Some((text.clone(), files.len())),
        _ => None,
    });
    assert_eq!(kept, Some(("the reader drops blank lines too".to_string(), 1)), "the thread keeps his words and the file");
}

#[tokio::test]
async fn begin_with_words_on_a_job_that_held_none_makes_them_its_first_turn() {
    let rig = rig();
    let id = rig.start_held().await;
    assert_eq!(card(&rig, &id).await.and_then(|one| one.message), None, "nothing is kept yet");

    begin_edited(&rig, &id, None, "the reader drops blank lines").await.expect("begun");
    eventually(|| async { rig.stand_in.starts().len() == 1 && told(&rig, "the reader drops blank lines") == 1 }).await;
    assert_eq!(said_on_the_thread(&rig, &id).await, ["the reader drops blank lines"], "written to the thread as his");
}

#[tokio::test]
async fn a_held_notes_composed_turn_is_the_cards_message_and_survives_a_restart_whole() {
    let rig = rig();
    let id = rig.start_held().await;
    Arc::clone(&rig.fleet).send_session_note(super::session_notes::note(&id, "Other commands", None)).await.expect("taken");
    let message = card(&rig, &id).await.and_then(|one| one.message).expect("the note is the first message");
    assert!(message.starts_with(super::session_notes::SAID) && message.contains("Pointed at: li \"Other commands\""), "{message}");
    assert!(said_on_the_thread(&rig, &id).await.is_empty());

    let rig = rig.restarted(fitted);
    assert_eq!(card(&rig, &id).await.and_then(|one| one.message), Some(message.clone()), "kept whole across the restart");
    begin(&rig, &id, Some(DeliveryChoice::OpenPr)).await.expect("begun");
    eventually(|| async { told(&rig, "Pointed at: li") == 1 }).await;
    assert_eq!(told(&rig, "aGVsbG8="), 1, "with the note's picture");
    assert_eq!(said_on_the_thread(&rig, &id).await, [super::session_notes::SAID], "the thread keeps his words alone");
}

#[tokio::test]
async fn another_jobs_word_to_a_held_job_with_nothing_kept_is_its_first_turn() {
    let rig = rig();
    let held = rig.start_held().await;
    a_job_writes_to(&rig, &held, 0).await;
    let message = card(&rig, &held).await.and_then(|one| one.message).expect("its first message");
    assert!(message.starts_with("A message from job") && message.ends_with("please review"), "{message}");
    assert!(said_on_the_thread(&rig, &held).await.is_empty());

    begin(&rig, &held, Some(DeliveryChoice::OpenPr)).await.expect("begun");
    eventually(|| async { rig.stand_in.starts().len() == 2 && lines_of(&rig, 1).len() == 1 }).await;
    assert!(lines_of(&rig, 1)[0].contains("A message from job"));
    assert_eq!(said_on_the_thread(&rig, &held).await, ["please review"]);
}

#[tokio::test]
async fn a_held_job_waits_in_needs_you_until_begin() {
    let rig = rig();
    let id = rig.start_held().await;
    let item = record(&rig, &id).await.waiting_for.into_iter().find(|one| one.source == WaitingSource::FirstMessage).expect("it waits");
    assert_eq!((item.id.clone(), item.text.as_str()), (format!("unsent:{}", id.as_str()), "First message"));
    let act = item.act.expect("an act");
    assert_eq!((act.kind, act.target.as_str()), (ipc::WaitingActKind::Message, id.as_str()));

    rig.send_unsettled(&id, FIRST).await;
    read(&rig, &id).await;
    begin(&rig, &id, Some(DeliveryChoice::OpenPr)).await.expect("begun");
    assert!(record(&rig, &id).await.waiting_for.iter().all(|one| one.source != WaitingSource::FirstMessage), "Start settles it");

    let ended = rig.start_held().await;
    let closed = Arc::clone(&rig.fleet).close_session(ipc::CloseSession { session_id: ended }).await.expect("closed");
    assert!(closed.waiting_for.is_empty(), "ending it settles it too");
}

#[tokio::test]
async fn a_held_job_with_no_workflow_takes_one_once_read_and_its_card_offers_auto_land() {
    let rig = rig();
    let id = rig.start_held().await;
    rig.send_unsettled(&id, FIRST).await;
    read(&rig, &id).await;
    assert_eq!(record(&rig, &id).await.workflow.as_deref(), Some("gate"), "the repository's first, where it holds no feature");
    assert_eq!(
        card(&rig, &id).await.and_then(|one| one.picked),
        Some(DeliveryPicked { choice: DeliveryChoice::AutoLand, from: DeliveryPickedFrom::Default })
    );

    // His custom pick stands: nothing defaults over it.
    let custom = rig.start_held().await;
    Arc::clone(&rig.fleet).set_session_workflow(SetSessionWorkflow { session_id: custom.clone(), workflow: None }).await.expect("set");
    rig.send_unsettled(&custom, FIRST).await;
    read(&rig, &custom).await;
    assert_eq!(record(&rig, &custom).await.workflow, None);
    assert_eq!(card(&rig, &custom).await.and_then(|one| one.picked), None, "a card that asks nothing offers nothing");
}

#[tokio::test]
async fn a_held_job_takes_the_feature_workflow_where_the_repository_holds_one() {
    let rig = rig_fitted(|fittings| {
        fitted(fittings);
        fittings.starting().workflows =
            ["bug", "feature"].into_iter().map(super::daemon::workflow_named).map(|one| (one.id().clone(), one)).collect();
    });
    let id = rig.start_held().await;
    rig.send_unsettled(&id, FIRST).await;
    read(&rig, &id).await;
    assert_eq!(record(&rig, &id).await.workflow.as_deref(), Some("feature"), "not bug, which sorts first");
}

/// The repository's workflows `bug`, `chore` and `feature`, which `held_defaulted` chooses among.
fn three(fittings: &mut Fittings<FakeHarness, FakeVcs, FakeWorkProduct>) {
    fitted(fittings);
    fittings.starting().workflows =
        ["bug", "chore", "feature"].into_iter().map(super::daemon::workflow_named).map(|one| (one.id().clone(), one)).collect();
}

/// A held Job whose first message has been read: the default has been weighed.
async fn defaulted(rig: &Rig) -> Option<String> {
    let id = rig.start_held().await;
    rig.send_unsettled(&id, FIRST).await;
    read(rig, &id).await;
    record(rig, &id).await.workflow
}

#[tokio::test]
async fn a_held_job_takes_feature_if_it_is_on_else_the_first_that_is_on_else_nothing() {
    let rig = rig_fitted(three);
    let here = manifest(&rig);
    assert_eq!(defaulted(&rig).await.as_deref(), Some("feature"), "feature, with all on");

    saved(&rig, off(&[], &[(&here, &["feature"])])).await;
    assert_eq!(defaulted(&rig).await.as_deref(), Some("bug"), "feature off: the first that is on");
    saved(&rig, off(&["bug", "feature"], &[])).await;
    assert_eq!(defaulted(&rig).await.as_deref(), Some("chore"), "and the next where that is off too, for the machine");
    saved(&rig, off(&["bug", "feature"], &[(&here, &["chore"])])).await;
    assert_eq!(defaulted(&rig).await, None, "none on: none taken");
}

#[tokio::test]
async fn a_default_that_found_nothing_on_is_not_weighed_so_a_workflow_switched_on_later_is_taken() {
    let rig = rig_fitted(three);
    saved(&rig, off(&["bug", "chore", "feature"], &[])).await;
    let id = rig.start_held().await;
    rig.send_unsettled(&id, FIRST).await;
    read(&rig, &id).await;
    assert_eq!(record(&rig, &id).await.workflow, None);
    assert!(!rig.fleet.hosts().of(id.as_str()).state().proposal.defaulted, "nothing was weighed");

    saved(&rig, back_on()).await;
    rig.fleet.held_defaulted(id.as_str()).await;
    assert_eq!(record(&rig, &id).await.workflow.as_deref(), Some("feature"), "the next weighing takes it");
}

#[tokio::test]
async fn a_none_picked_on_the_card_survives_a_fleet_restart_and_no_default_goes_over_it() {
    let rig = rig_fitted(three);
    let id = rig.start_held().await;
    Arc::clone(&rig.fleet).set_session_workflow(SetSessionWorkflow { session_id: id.clone(), workflow: None }).await.expect("set");
    let rig = rig.restarted(three);

    rig.send_unsettled(&id, FIRST).await;
    read(&rig, &id).await;
    rig.fleet.held_defaulted(id.as_str()).await;
    assert_eq!(record(&rig, &id).await.workflow, None, "his None stands");
    assert_eq!(defaulted(&rig).await.as_deref(), Some("feature"), "where a Job he did not pick for takes its default");
}

#[tokio::test]
async fn picking_the_workflow_a_job_already_holds_is_still_a_choice_the_default_does_not_go_over() {
    let rig = rig_fitted(three);
    let id = rig.start_held().await;
    rig.send_unsettled(&id, FIRST).await;
    read(&rig, &id).await;
    let chosen = |rig: &Rig| {
        let fleet = Arc::clone(&rig.fleet);
        let id = id.clone();
        async move { fleet.store().lock().await.hosting(id.as_str()).expect("reads").expect("kept").workflow_chosen }
    };
    assert_eq!(record(&rig, &id).await.workflow.as_deref(), Some("feature"), "taken as its default");
    assert!(!chosen(&rig).await, "a default is not a choice");

    Arc::clone(&rig.fleet)
        .set_session_workflow(SetSessionWorkflow { session_id: id.clone(), workflow: Some("feature".into()) })
        .await
        .expect("set");
    assert!(chosen(&rig).await, "picking what it holds is a choice, and kept");
}

#[tokio::test]
async fn messages_sent_while_held_go_in_the_order_they_came() {
    let rig = rig();
    let id = rig.start_held().await;
    rig.send_unsettled(&id, "first words").await;
    rig.send_unsettled(&id, "second words").await;
    begin(&rig, &id, Some(DeliveryChoice::OpenPr)).await.expect("begun");
    eventually(|| async { sent(&rig).len() == 2 }).await;
    let lines = sent(&rig);
    assert!(lines[0].contains("first words") && lines[1].contains("second words"), "{lines:?}");
}

/// The thread's messages by their words, and the agent's lines, are in the order he sent them.
async fn thread_and_agent_read_in_order(rig: &Rig, id: &SessionId, words: &[&str]) {
    eventually(|| async { sent(rig).len() == words.len() }).await;
    let lines = sent(rig);
    for (line, word) in lines.iter().zip(words) {
        assert!(line.contains(word), "the agent got {lines:?}");
    }
    assert_eq!(said_on_the_thread(rig, id).await, words, "the thread reads as the agent did");
}

#[tokio::test]
async fn later_messages_sent_while_held_are_written_to_the_thread_on_start_after_the_first() {
    let rig = rig();
    let id = rig.start_held().await;
    rig.send_unsettled(&id, "first words").await;
    rig.send_unsettled(&id, "second words").await;
    rig.send_unsettled(&id, "third words").await;
    assert!(said_on_the_thread(&rig, &id).await.is_empty(), "nothing reaches the thread while he can still edit the first");

    begin(&rig, &id, Some(DeliveryChoice::OpenPr)).await.expect("begun");
    thread_and_agent_read_in_order(&rig, &id, &["first words", "second words", "third words"]).await;
}

#[tokio::test]
async fn later_messages_sent_while_held_survive_a_fleet_restart_and_are_written_in_order_on_start() {
    let rig = rig();
    let id = rig.start_held().await;
    rig.send_unsettled(&id, "first words").await;
    rig.send_unsettled(&id, "second words").await;
    let rig = rig.restarted(fitted);
    rig.send_unsettled(&id, "third words").await;
    assert!(said_on_the_thread(&rig, &id).await.is_empty(), "still nothing on the thread");
    assert!(rig.stand_in.starts().is_empty(), "no agent runs before Start");

    begin(&rig, &id, Some(DeliveryChoice::OpenPr)).await.expect("begun");
    thread_and_agent_read_in_order(&rig, &id, &["first words", "second words", "third words"]).await;
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
    begin(&rig, &id, Some(DeliveryChoice::OpenPr)).await.expect("begun");
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
    // He chose custom, so no default workflow goes over it once the message is read.
    Arc::clone(&rig.fleet).set_session_workflow(SetSessionWorkflow { session_id: id.clone(), workflow: None }).await.expect("set");
    rig.send_unsettled(&id, FIRST).await;
    read(&rig, &id).await;
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
    assert_eq!(
        offered(&rig).await,
        Some(DeliveryPicked { choice: DeliveryChoice::AutoLand, from: DeliveryPickedFrom::Default }),
        "two picks are not yet a habit, so the card offers Fleet's own"
    );
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
        .answer_waiting(ipc::AnswerWaiting { session_id: asked.clone(), item_id: "delivery".into(), choice: Some(2), text: None, mode: None, fixed: None })
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
/// `rows` is how many messages `to`'s thread holds before it: a held Job's turns are not on it.
async fn a_job_writes_to(rig: &Rig, to: &SessionId, rows: usize) -> SessionId {
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
    eventually(|| async {
        let reached = rig.rows(to).await.iter().filter(|row| matches!(row, SessionRow::Message { .. })).count() == rows + 1;
        let kept = !rig.fleet.store().lock().await.held_later_turns(to.as_str()).unwrap().is_empty();
        reached || kept || card(rig, to).await.and_then(|one| one.message).is_some_and(|said| said.contains("please review"))
    })
    .await;
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
    a_job_writes_to(&rig, &held, 0).await;
    assert_eq!(rig.stand_in.starts().len(), 1, "only the Job that wrote has an agent; the held one is not woken");

    begin(&rig, &held, Some(DeliveryChoice::OpenPr)).await.expect("begun");
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
    a_job_writes_to(&rig, &held, 0).await;
    let rig = rig.restarted(fitted);

    begin(&rig, &held, Some(DeliveryChoice::OpenPr)).await.expect("begun");
    eventually(|| async { rig.stand_in.starts().len() == 1 && lines_of(&rig, 0).len() == 2 }).await;
    let lines = lines_of(&rig, 0);
    assert!(lines[0].contains(FIRST) && lines[1].contains("please review"), "in the order the rows were written: {lines:?}");
    assert_eq!(lines.iter().filter(|line| line.contains("please review")).count(), 1, "once");
}
