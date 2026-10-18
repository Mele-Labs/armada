//! The proposer at a Session's dispatch, and the changes a person makes in a panel reaching the
//! agent. Since 23.91. `docs/concepts/session.md`, *A Session's proposal*.
//!
//! **The model is the fake and the process is the stand-in**: what runs is Fleet's call, its
//! reading of the answer, the store and the order a turn is sent in.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Sessions};
use config::ResolvedWorkflow;
use core_model::WorkflowId;
use ipc::{
    ProposedField, RenameSession, SessionId, SessionMode, SessionRecord, SessionRow, SessionTurn, SessionVoice,
    SetSessionWorkflow, TimelineAsk, TimelineEvent, TuneSession,
};
use testkit::FakeJudge;

use super::daemon::workflow_named;
use super::session_host::{eventually, rig_fitted, Rig};
use crate::judging::JudgeBudget;
use crate::session_proposer::{NamesNoWorkflow, SessionBrief, SessionProposed};

const REQUEST: &str = "the log reader drops the last line of every file";
const ANSWER: &str =
    "workflow: bug\ntitle: Log reader drops a line\nsettings: model=another-model, effort=high";

fn catalogue() -> BTreeMap<WorkflowId, ResolvedWorkflow> {
    ["bug", "feature"]
        .into_iter()
        .map(|id| workflow_named(id))
        .map(|workflow| (workflow.id().clone(), workflow))
        .collect()
}

fn models() -> Vec<String> {
    vec!["a-model".to_string(), "another-model".to_string()]
}

fn read(answer: &str) -> Result<SessionProposed, NamesNoWorkflow> {
    let held = catalogue();
    SessionBrief::about(REQUEST, &held, &models()).read(answer, &held, &models())
}

/// A rig holding two workflows whose proposer answers `proposer`.
fn proposing(proposer: FakeJudge) -> Rig {
    rig_fitted(|fittings| {
        fittings.judge = Arc::new(proposer);
        fittings.starting().workflows = catalogue();
    })
}

async fn record(rig: &Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session
}

async fn landed(rig: &Rig, id: &SessionId) -> SessionRecord {
    eventually(|| async { record(rig, id).await.proposal.is_none_or(|one| !one.reading) }).await;
    record(rig, id).await
}

fn picked(record: &SessionRecord) -> Vec<ProposedField> {
    record.proposal.clone().map(|one| one.picked).unwrap_or_default()
}

fn sent(rig: &Rig, process: usize) -> Vec<String> {
    rig.stand_in.process(process).sent.lock().unwrap().clone()
}

async fn entered(rig: &Rig) -> Vec<TimelineEvent> {
    rig.fleet.timeline().settled().await;
    rig.fleet.fleet_timeline(TimelineAsk::default()).await.expect("a timeline").events
}

fn says_of(events: &[TimelineEvent], kind: &str) -> Vec<String> {
    events
        .iter()
        .filter(|one| one.kind.as_wire() == kind)
        .map(|one| one.says.clone())
        .collect()
}

async fn idle(rig: &Rig, id: &SessionId) {
    eventually(|| async { rig.turn(id).await == SessionTurn::Idle }).await;
}

// ---- Reading the answer, one field at a time ----

#[test]
fn a_full_answer_reads_into_each_field() {
    let read = read(ANSWER).expect("readable");
    assert_eq!(read.workflow.as_ref().map(|id| id.as_str()), Some("bug"));
    assert_eq!(read.title.as_deref(), Some("Log reader drops a line"));
    assert_eq!(read.model.as_deref(), Some("another-model"));
    assert_eq!(read.effort.as_deref(), Some("high"));
}

#[test]
fn a_none_workflow_is_a_custom_session_and_never_a_default_one() {
    let read = read("workflow: none\ntitle: Look around\nsettings: model=a-model, effort=low")
        .expect("readable");
    assert_eq!(read.workflow, None);
    assert_eq!(read.title.as_deref(), Some("Look around"));
    assert_eq!(read.model.as_deref(), Some("a-model"));
}

#[test]
fn a_workflow_the_repository_does_not_hold_is_none_too() {
    let read = read("workflow: epic\ntitle: Whatever").expect("readable");
    assert_eq!(read.workflow, None, "a name nothing holds is not evidence about which was meant");
}

#[test]
fn a_model_this_machine_does_not_run_counts_as_none() {
    let read = read("workflow: bug\ntitle: T\nsettings: model=imaginary, effort=medium")
        .expect("readable");
    assert_eq!(read.model, None);
    assert_eq!(read.effort.as_deref(), Some("medium"), "the effort beside it is read on its own");
}

#[test]
fn an_effort_the_cli_does_not_take_counts_as_none() {
    let read = read("workflow: bug\ntitle: T\nsettings: model=a-model, effort=extreme")
        .expect("readable");
    assert_eq!(read.effort, None);
    assert_eq!(read.model.as_deref(), Some("a-model"));
}

#[test]
fn an_answer_with_no_workflow_line_is_unreadable() {
    assert_eq!(read("this looks like a bug to me"), Err(NamesNoWorkflow));
}

// ---- The call, off the press ----

#[tokio::test]
async fn the_press_does_not_wait_and_the_first_message_is_on_the_thread_before_the_process_starts() {
    let rig = proposing(FakeJudge::saying(ANSWER).taking(Duration::from_millis(500)));
    let started = rig.start_requesting(REQUEST, |_| {}).await;
    let id = started.id.clone();

    assert!(started.proposal.as_ref().is_some_and(|one| one.reading), "reading is marked at once");
    assert!(!started.hosted.expect("hosted").running, "and no process has started");

    rig.send(&id, REQUEST).await;
    assert!(rig.stand_in.starts().is_empty(), "the process waits for the answer");
    assert!(
        rig.rows(&id).await.iter().any(|row| matches!(
            row, SessionRow::Message { text, .. } if text == REQUEST
        )),
        "the first message is on the thread though its turn is held"
    );

    // The answer lands: the process starts on what it settled and the held turn follows.
    eventually(|| async { !rig.stand_in.starts().is_empty() }).await;
    let start = rig.stand_in.starts().remove(0);
    assert_eq!(start.model.as_deref(), Some("another-model"));
    assert_eq!(start.effort.as_deref(), Some("high"));
    eventually(|| async { sent(&rig, 0).iter().any(|line| line.contains(REQUEST)) }).await;
}

#[tokio::test]
async fn turns_sent_while_it_is_read_go_in_the_order_they_came() {
    let rig = proposing(FakeJudge::saying(ANSWER).taking(Duration::from_millis(300)));
    let id = rig.start_requesting(REQUEST, |_| {}).await.id;
    rig.send(&id, "first words").await;
    rig.send(&id, "second words").await;

    eventually(|| async { !rig.stand_in.starts().is_empty() && sent(&rig, 0).len() == 2 }).await;
    let lines = sent(&rig, 0);
    assert!(lines[0].contains("first words") && lines[1].contains("second words"), "{lines:?}");
}

#[tokio::test]
async fn the_answer_is_recorded_per_field_and_the_first_turn_is_told_the_workflow() {
    let rig = proposing(FakeJudge::saying(ANSWER));
    let id = rig.start_requesting(REQUEST, |_| {}).await.id;
    rig.send(&id, REQUEST).await;

    let landed = landed(&rig, &id).await;
    assert_eq!(landed.title.as_deref(), Some("Log reader drops a line"));
    assert_eq!(landed.workflow.as_deref(), Some("bug"));
    let hosted = landed.hosted.clone().expect("hosted");
    assert_eq!(hosted.model.as_deref(), Some("another-model"));
    assert_eq!(hosted.effort.as_deref(), Some("high"));
    assert_eq!(
        picked(&landed),
        [ProposedField::Title, ProposedField::Workflow, ProposedField::Model, ProposedField::Effort]
    );

    eventually(|| async { !sent(&rig, 0).is_empty() }).await;
    let first = sent(&rig, 0).remove(0);
    assert!(first.contains("follows the bug workflow") && first.contains("only_in_bug"), "{first}");
}

#[tokio::test]
async fn what_the_person_set_on_the_start_wins_over_the_answer() {
    let rig = proposing(FakeJudge::saying(ANSWER));
    let id = rig
        .start_requesting(REQUEST, |start| {
            start.title = Some("Mine".to_string());
            start.model = Some("a-model".to_string());
            start.effort = Some("low".to_string());
        })
        .await
        .id;
    rig.send(&id, REQUEST).await;

    let landed = landed(&rig, &id).await;
    assert_eq!(landed.title.as_deref(), Some("Mine"));
    let hosted = landed.hosted.clone().expect("hosted");
    assert_eq!((hosted.model.as_deref(), hosted.effort.as_deref()), (Some("a-model"), Some("low")));
    assert_eq!(landed.workflow.as_deref(), Some("bug"), "what they did not set is still filled");
    assert_eq!(picked(&landed), [ProposedField::Workflow], "and only that is marked");
    eventually(|| async { !rig.stand_in.starts().is_empty() }).await;
    assert_eq!(rig.stand_in.starts()[0].model.as_deref(), Some("a-model"));
}

#[tokio::test]
async fn a_name_given_while_it_is_read_wins_too() {
    let rig = proposing(FakeJudge::saying(ANSWER).taking(Duration::from_millis(300)));
    let id = rig.start_requesting(REQUEST, |_| {}).await.id;
    Arc::clone(&rig.fleet)
        .rename_session(RenameSession { session_id: id.clone(), title: "Mine".to_string() })
        .await
        .expect("renamed");

    let landed = landed(&rig, &id).await;
    assert_eq!(landed.title.as_deref(), Some("Mine"));
    assert!(!picked(&landed).contains(&ProposedField::Title));
}

#[tokio::test]
async fn a_change_in_a_panel_clears_that_fields_mark_and_no_other() {
    let rig = proposing(FakeJudge::saying(ANSWER));
    let id = rig.start_requesting(REQUEST, |_| {}).await.id;
    let landed = landed(&rig, &id).await;
    assert_eq!(picked(&landed).len(), 4);

    // A model chosen, the effort left as it was.
    Arc::clone(&rig.fleet)
        .tune_session(TuneSession {
            session_id: id.clone(),
            model: Some("a-model".to_string()),
            effort: Some("high".to_string()),
            mode: SessionMode::Auto,
        })
        .await
        .expect("tuned");
    assert_eq!(
        picked(&record(&rig, &id).await),
        [ProposedField::Title, ProposedField::Workflow, ProposedField::Effort]
    );

    Arc::clone(&rig.fleet)
        .rename_session(RenameSession { session_id: id.clone(), title: "Mine".to_string() })
        .await
        .expect("renamed");
    assert_eq!(picked(&record(&rig, &id).await), [ProposedField::Workflow, ProposedField::Effort]);

    Arc::clone(&rig.fleet)
        .set_session_workflow(SetSessionWorkflow { session_id: id.clone(), workflow: None })
        .await
        .expect("cleared");
    let after = record(&rig, &id).await;
    assert_eq!(after.workflow, None);
    assert_eq!(picked(&after), [ProposedField::Effort]);
}

#[tokio::test]
async fn a_call_that_fails_starts_the_session_on_the_machines_defaults() {
    let rig = proposing(FakeJudge::that_fails("the model is down"));
    let id = rig.start_requesting(REQUEST, |_| {}).await.id;
    rig.send(&id, REQUEST).await;

    let landed = landed(&rig, &id).await;
    assert_eq!(landed.workflow, None, "no workflow is made up");
    assert!(landed.proposal.is_none(), "and nothing is marked as picked");
    assert_eq!(landed.title.as_deref(), Some(REQUEST), "the title is the words, as it is without a proposer");
    assert_eq!(landed.hosted.expect("hosted").model, None);
    eventually(|| async { !rig.stand_in.starts().is_empty() }).await;
    assert_eq!(rig.stand_in.starts()[0].model.as_deref(), Some("a-model"), "the machine's own default");
    eventually(|| async { sent(&rig, 0).iter().any(|line| line.contains(REQUEST)) }).await;
}

#[tokio::test]
async fn an_answer_that_cannot_be_read_starts_on_the_defaults_too() {
    let rig = proposing(FakeJudge::saying("I would call this a bug."));
    let id = rig.start_requesting(REQUEST, |_| {}).await.id;
    rig.send(&id, REQUEST).await;

    let landed = landed(&rig, &id).await;
    assert!(landed.proposal.is_none() && landed.workflow.is_none());
    eventually(|| async { sent(&rig, 0).iter().any(|line| line.contains(REQUEST)) }).await;
}

#[tokio::test]
async fn a_call_that_outlasts_its_budget_starts_on_the_defaults() {
    let rig = rig_fitted(|fittings| {
        fittings.judge = Arc::new(FakeJudge::saying(ANSWER).taking(Duration::from_secs(30)));
        fittings.starting().workflows = catalogue();
        fittings.proposer_budget = JudgeBudget::of(Duration::from_millis(300));
    });
    let id = rig.start_requesting(REQUEST, |_| {}).await.id;
    rig.send(&id, REQUEST).await;

    let landed = landed(&rig, &id).await;
    assert!(landed.proposal.is_none() && landed.workflow.is_none());
    eventually(|| async { sent(&rig, 0).iter().any(|line| line.contains(REQUEST)) }).await;
}

// ---- Every start is read at its first message ----

/// A rig whose proposer is `judge`, which the test keeps to count the calls it was put.
fn counting(judge: &Arc<FakeJudge>) -> Rig {
    let judge = Arc::clone(judge);
    rig_fitted(move |fittings| {
        fittings.judge = judge;
        fittings.starting().workflows = catalogue();
    })
}

#[tokio::test]
async fn a_start_without_a_request_is_read_at_its_first_message() {
    let judge = Arc::new(FakeJudge::saying(ANSWER).taking(Duration::from_millis(400)));
    let rig = counting(&judge);
    // An annotation's start: a title and a root, no request; the notes follow as a message.
    let id = rig.start_titled(Some("Notes")).await;
    assert!(record(&rig, &id).await.proposal.is_none(), "nothing is read before there are words");

    rig.send_unsettled(&id, REQUEST).await;
    assert!(record(&rig, &id).await.proposal.is_some_and(|one| one.reading), "the first message is being read");
    assert!(rig.stand_in.starts().is_empty(), "and its turn is held");

    let landed = landed(&rig, &id).await;
    assert_eq!(landed.title.as_deref(), Some("Notes"), "the title the start carried stands");
    assert_eq!(landed.workflow.as_deref(), Some("bug"));
    assert_eq!(landed.hosted.expect("hosted").model.as_deref(), Some("another-model"));
    assert_eq!(picked(&record(&rig, &id).await), [ProposedField::Workflow, ProposedField::Model, ProposedField::Effort]);
    eventually(|| async { sent(&rig, 0).iter().any(|line| line.contains(REQUEST)) }).await;
    assert_eq!(rig.stand_in.starts()[0].model.as_deref(), Some("another-model"));
}

#[tokio::test]
async fn a_session_already_read_is_not_read_again() {
    let judge = Arc::new(FakeJudge::saying(ANSWER));
    let rig = counting(&judge);

    // Read at its dispatch: its first message is not read a second time.
    let id = rig.start_requesting(REQUEST, |_| {}).await.id;
    rig.send(&id, REQUEST).await;
    landed(&rig, &id).await;
    eventually(|| async { !sent(&rig, 0).is_empty() }).await;
    assert_eq!(judge.asked().len(), 1);

    // Read at its first message: the second is not read either.
    let other = rig.start_titled(None).await;
    rig.send(&other, REQUEST).await;
    landed(&rig, &other).await;
    rig.send(&other, "and the footer").await;
    eventually(|| async { rig.stand_in.starts().len() == 2 }).await;
    assert_eq!(judge.asked().len(), 2, "one call for each Session, and none for its later messages");
}

#[tokio::test]
async fn words_that_are_not_a_person_s_first_message_are_not_read() {
    let judge = Arc::new(FakeJudge::saying(ANSWER));
    let rig = counting(&judge);
    let id = rig.start_titled(None).await;
    // Fleet's own word, and the brief of a Session another started, are not a person's request.
    Arc::clone(&rig.fleet).told_by_fleet(id.as_str(), "main moved").await.expect("told");
    rig.fleet
        .first_message_from(id.as_str(), SessionVoice::Session { id: "rescue".into(), title: "Rescue".into() }, REQUEST)
        .await
        .expect("sent");
    eventually(|| async { !rig.stand_in.starts().is_empty() }).await;
    assert!(judge.asked().is_empty());
    assert!(record(&rig, &id).await.proposal.is_none());
}

// ---- A change made in a panel reaches the agent ----

#[tokio::test]
async fn a_workflow_change_tells_the_agent_at_once_and_enters_its_event() {
    let rig = proposing(FakeJudge::that_fails("not asked"));
    crate::timeline::keep_timeline(Arc::clone(&rig.fleet));
    let id = rig.start().await;
    rig.send(&id, "hello").await;
    assert!(matches!(rig.turn(&id).await, SessionTurn::Working { .. }), "mid-turn");

    let set = Arc::clone(&rig.fleet)
        .set_session_workflow(SetSessionWorkflow {
            session_id: id.clone(),
            workflow: Some("bug".to_string()),
        })
        .await
        .expect("set");
    assert_eq!(set.workflow.as_deref(), Some("bug"));
    let told = sent(&rig, 0).pop().expect("a line");
    assert!(
        told.contains("now follows the bug workflow") && told.contains("only_in_bug"),
        "the agent is told the workflow and its steps without waiting for the turn: {told}"
    );

    // Idle, clearing it wakes the Session.
    rig.stand_in.finishes(0, "done");
    idle(&rig, &id).await;
    Arc::clone(&rig.fleet)
        .set_session_workflow(SetSessionWorkflow { session_id: id.clone(), workflow: None })
        .await
        .expect("cleared");
    assert!(sent(&rig, 0).pop().expect("a line").contains("custom Job"));
    assert!(matches!(rig.turn(&id).await, SessionTurn::Working { .. }), "it woke");

    assert_eq!(
        says_of(&entered(&rig).await, "session.workflow"),
        ["workflow: custom → bug", "workflow: bug → custom"]
    );
}

#[tokio::test]
async fn a_workflow_the_repository_does_not_hold_is_refused() {
    let rig = proposing(FakeJudge::that_fails("not asked"));
    let id = rig.start().await;
    let refused = Arc::clone(&rig.fleet)
        .set_session_workflow(SetSessionWorkflow {
            session_id: id,
            workflow: Some("epic".to_string()),
        })
        .await;
    assert!(refused.is_err());
}

#[tokio::test]
async fn a_model_change_tells_the_turn_that_starts_on_it_and_enters_its_event() {
    let rig = proposing(FakeJudge::that_fails("not asked"));
    crate::timeline::keep_timeline(Arc::clone(&rig.fleet));
    let id = rig.start().await;
    rig.send(&id, "one").await;
    rig.stand_in.finishes(0, "done");
    idle(&rig, &id).await;

    Arc::clone(&rig.fleet)
        .tune_session(TuneSession {
            session_id: id.clone(),
            model: Some("another-model".to_string()),
            effort: None,
            mode: SessionMode::Auto,
        })
        .await
        .expect("tuned");
    assert_eq!(rig.stand_in.starts().len(), 1, "tuning wakes nothing and tells nothing");
    rig.send(&id, "two").await;

    eventually(|| async { rig.stand_in.starts().len() == 2 }).await;
    let turn = sent(&rig, 1).remove(0);
    assert!(
        turn.contains("your model is another-model, not a-model") && turn.contains("two"),
        "{turn}"
    );
    let events = entered(&rig).await;
    assert_eq!(says_of(&events, "session.model"), ["model: a-model → another-model"]);
    assert!(says_of(&events, "session.effort").is_empty(), "the effort did not change");
}

#[tokio::test]
async fn an_effort_change_is_told_and_entered_the_same_way() {
    let rig = proposing(FakeJudge::that_fails("not asked"));
    crate::timeline::keep_timeline(Arc::clone(&rig.fleet));
    let id = rig.start().await;
    rig.send(&id, "one").await;
    rig.stand_in.finishes(0, "done");
    idle(&rig, &id).await;

    Arc::clone(&rig.fleet)
        .tune_session(TuneSession {
            session_id: id.clone(),
            model: None,
            effort: Some("high".to_string()),
            mode: SessionMode::Auto,
        })
        .await
        .expect("tuned");
    rig.send(&id, "two").await;

    eventually(|| async { rig.stand_in.starts().len() == 2 }).await;
    assert!(sent(&rig, 1).remove(0).contains("your effort is high, not the default"));
    assert_eq!(says_of(&entered(&rig).await, "session.effort"), ["effort: the default → high"]);
}

#[tokio::test]
async fn an_unchanged_model_tells_nothing() {
    let rig = proposing(FakeJudge::that_fails("not asked"));
    crate::timeline::keep_timeline(Arc::clone(&rig.fleet));
    let id = rig.start().await;
    rig.send(&id, "one").await;
    rig.stand_in.finishes(0, "done");
    idle(&rig, &id).await;
    rig.send(&id, "two").await;

    eventually(|| async { sent(&rig, 0).len() == 2 }).await;
    for line in sent(&rig, 0) {
        assert!(!line.contains("from this turn"), "nothing changed, so nothing is said: {line}");
    }
    let events = entered(&rig).await;
    assert!(says_of(&events, "session.model").is_empty() && says_of(&events, "session.effort").is_empty());
}
