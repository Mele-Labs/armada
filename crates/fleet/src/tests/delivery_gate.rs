//! The claim, written before the code: **a bug or feature Job is stopped at its start and asked how
//! the work is delivered, unless its first message already says, and the answer is kept on the Job.**
//! `docs/concepts/session/the-delivery-question.md`.

use std::sync::Arc;

use api::HostedSessions;
use ipc::{AnswerWaiting, DeliveryChoice, DismissWaiting, SessionId, SessionRecord, SetSessionWorkflow, WaitingItem, WaitingSource};
use testkit::FakeJudge;

use super::session_host::{eventually, rig_fitted, Rig};
use crate::session_host::delivering::{delivery_of, named_delivery};

#[test]
fn a_message_names_a_delivery_only_when_it_names_exactly_one() {
    use DeliveryChoice::{AutoLand, KeepLocal, OpenPr};
    for (said, expected) in [
        ("Fix the reader and keep it local.", Some(KeepLocal)),
        ("fix it, don't push anything", Some(KeepLocal)),
        ("Do not push, I will read it first", Some(KeepLocal)),
        ("Add the flag, then open a PR", Some(OpenPr)),
        ("pull request only please", Some(OpenPr)),
        ("Fix it and land it", Some(AutoLand)),
        ("Auto-merge when it is green", Some(AutoLand)),
        ("The reader drops the last line", None),
        ("Open a PR and merge it", None),
        ("Do not merge it", None),
        ("Open a PR, do not push to main", None),
    ] {
        assert_eq!(named_delivery(said), expected, "{said}");
    }
}

const WORKFLOW: &str = "version: 1\nworkflow_id: gate\nname: Gate\nsteps:\n  \
    - id: plan\n    label: \"Plan the change\"\n    instructions: \"Record the plan before editing.\"\n    asks_delivery: true\n    \
    evidence: {submitted: {type: plan}}\n    mechanical_checks:\n      - { type: plan_recorded, min_tasks: 1 }\n    delivers: false\n    advance_gate: auto\n  \
    - id: handoff\n    label: \"Hand off\"\n    delivers: true\n    advance_gate: auto\n";

fn rig() -> Rig {
    let def = config::WorkflowDef::parse(std::path::Path::new("gate.yml"), WORKFLOW, &config::Roster::offering_nothing())
        .unwrap_or_else(|refused| panic!("the fixture did not parse: {refused}"));
    let workflow = config::ResolvedWorkflow::resolve(&def, &super::daemon::manifest())
        .unwrap_or_else(|refused| panic!("the fixture did not resolve: {refused}"));
    rig_fitted(|fittings| {
        fittings.judge = Arc::new(FakeJudge::with_no_objection());
        fittings.starting().workflows = super::daemon::one(workflow);
    })
}

async fn on_the_rail(rig: &Rig, first: &str) -> SessionId {
    crate::timeline::keep_timeline(Arc::clone(&rig.fleet));
    crate::rail_watching::keep_the_rail(Arc::clone(&rig.fleet));
    let id = rig.start().await;
    rig.send(&id, first).await;
    rig.stand_in.init(0);
    Arc::clone(&rig.fleet)
        .set_session_workflow(SetSessionWorkflow { session_id: id.clone(), workflow: Some("gate".into()) })
        .await
        .expect("set");
    id
}

async fn record(rig: &Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session
}

async fn question(rig: &Rig, id: &SessionId) -> Option<WaitingItem> {
    record(rig, id).await.waiting_for.into_iter().find(|one| one.source == WaitingSource::Delivery)
}

fn told(rig: &Rig, containing: &str) -> usize {
    rig.stand_in.process(0).sent.lock().unwrap().iter().filter(|line| line.contains(containing)).count()
}

#[tokio::test]
async fn a_job_whose_first_message_names_no_delivery_is_stopped_and_asked() {
    let rig = rig();
    let id = on_the_rail(&rig, "the log reader drops the last line of every file").await;
    eventually(|| async { told(&rig, "choosing how this work is delivered") == 1 }).await;

    let item = question(&rig, &id).await.expect("the question is on the job's needs");
    assert_eq!(item.text, "How should this work be delivered?");
    let labels: Vec<_> = item.options.iter().map(|one| one.label.as_str()).collect();
    assert_eq!(labels, ["Auto land", "Open PR only", "Keep local"]);
    assert_eq!(told(&rig, "Record the plan before editing."), 0, "the step's instructions wait for the answer");
    assert_eq!(delivery_of(&rig.rows(&id).await), None, "nothing is chosen yet");
}

#[tokio::test]
async fn the_answer_is_kept_on_the_job_and_sent_to_the_agent_with_the_steps_instructions() {
    let rig = rig();
    let id = on_the_rail(&rig, "the log reader drops the last line of every file").await;
    eventually(|| async { question(&rig, &id).await.is_some() }).await;

    let answered = Arc::clone(&rig.fleet)
        .answer_waiting(AnswerWaiting { session_id: id.clone(), item_id: "delivery".into(), choice: Some(1), text: None, mode: None })
        .await
        .expect("answered");
    assert!(answered.waiting_for.iter().all(|one| one.source != WaitingSource::Delivery), "the question is gone");
    assert_eq!(delivery_of(&rig.rows(&id).await), Some(DeliveryChoice::OpenPr));
    eventually(|| async { told(&rig, "the owner chose Open PR only") == 1 }).await;
    assert_eq!(told(&rig, "Record the plan before editing."), 1, "the instructions arrive with the choice");
    assert!(question(&rig, &id).await.is_none(), "and it is not asked again");
}

#[tokio::test]
async fn a_first_message_that_names_a_delivery_is_not_asked() {
    let rig = rig();
    let id = on_the_rail(&rig, "the log reader drops the last line of every file. Keep it local.").await;
    eventually(|| async { told(&rig, "Record the plan before editing.") == 1 }).await;

    assert!(question(&rig, &id).await.is_none());
    assert_eq!(delivery_of(&rig.rows(&id).await), Some(DeliveryChoice::KeepLocal));
    assert_eq!(told(&rig, "choosing how this work is delivered"), 0, "the agent is not stopped");
}

#[tokio::test]
async fn the_question_is_answered_with_one_of_the_three_and_never_dismissed() {
    let rig = rig();
    let id = on_the_rail(&rig, "the log reader drops the last line of every file").await;
    eventually(|| async { question(&rig, &id).await.is_some() }).await;

    let words = Arc::clone(&rig.fleet)
        .answer_waiting(AnswerWaiting { session_id: id.clone(), item_id: "delivery".into(), choice: None, text: Some("whatever".into()), mode: None })
        .await;
    assert!(words.is_err(), "words alone are refused");
    let dismissed = Arc::clone(&rig.fleet)
        .dismiss_waiting(DismissWaiting { session_id: id.clone(), item_id: "delivery".into(), reason: None })
        .await;
    assert!(dismissed.is_err(), "the question is answered, not dismissed");
    assert!(question(&rig, &id).await.is_some(), "it still stands");
}
