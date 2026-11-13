//! The claim, written before the code: **a project's `delivery` answers the delivery question, the first answer in a
//! project that sets none becomes its value, and the person is told so once.** `ask` turns the learning off.
//! `docs/concepts/session/the-delivery-question.md`.

use std::sync::Arc;

use api::{HostedSessions, Projects};
use ipc::{
    AnswerWaiting, BeginSession, DeliveryChoice, NewProject, ProjectMember, SessionId, SessionRecord, SessionRow, SetSessionWorkflow,
    ValueLayer, ValuesSet, WaitingItem, WaitingSource,
};
use testkit::FakeJudge;

use super::delivery_gate::workflow;
use super::session_host::{eventually, rig_fitted, Rig};
use crate::session_host::delivering::delivery_of;

const FIRST: &str = "the log reader drops the last line of every file";
const WHERE: &str = "Change it in Settings, in the project's settings.";

fn rig() -> Rig {
    let workflow = workflow();
    rig_fitted(|fittings| {
        fittings.judge = Arc::new(FakeJudge::with_no_objection());
        fittings.starting().workflows = super::daemon::one(workflow);
    })
}

/// A project of the served repository, setting `delivery` where a word is given.
async fn project(rig: &Rig, delivery: Option<&str>) -> String {
    let values = ValuesSet { delivery: delivery.map(str::to_string), pr_mode: Some(core_model::PrMode::Draft.into()), ..ValuesSet::default() };
    let members = Some(vec![ProjectMember { manifest_id: rig.manifest.clone(), folder: None }]);
    let new = NewProject { name: "Shop".to_string(), members, place: None, values: Some(values) };
    rig.fleet.add_project(new).await.expect("added").projects.last().expect("the project").id.clone()
}

fn watching(rig: &Rig) {
    crate::timeline::keep_timeline(Arc::clone(&rig.fleet));
    crate::rail_following::keep_the_rail(Arc::clone(&rig.fleet));
}

async fn on_gate(rig: &Rig, id: &SessionId) {
    Arc::clone(&rig.fleet)
        .set_session_workflow(SetSessionWorkflow { session_id: id.clone(), workflow: Some("gate".into()) })
        .await
        .expect("set");
}

/// A Job whose first message arrives after the workflow is set, so the first turn carries what the step says.
async fn first_turn_on_the_rail(rig: &Rig, project: Option<&str>, first: &str) -> SessionId {
    watching(rig);
    let id = rig.start_for(project, false).await;
    on_gate(rig, &id).await;
    rig.send(&id, first).await;
    id
}

/// A Job that meets the step after its first message, so the step's entry says what it says.
async fn entering_the_rail(rig: &Rig, project: Option<&str>, first: &str) -> SessionId {
    watching(rig);
    let id = rig.start_for(project, false).await;
    rig.send(&id, first).await;
    rig.stand_in.init(0);
    on_gate(rig, &id).await;
    id
}

async fn record(rig: &Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session
}

async fn question(rig: &Rig, id: &SessionId) -> Option<WaitingItem> {
    record(rig, id).await.waiting_for.into_iter().find(|one| one.source == WaitingSource::Delivery)
}

fn told(rig: &Rig, containing: &str) -> usize {
    if rig.stand_in.starts().is_empty() {
        return 0;
    }
    rig.stand_in.process(0).sent.lock().unwrap().iter().filter(|line| line.contains(containing)).count()
}

/// What the person is told on the `delivery` row.
async fn said(rig: &Rig, id: &SessionId) -> Option<String> {
    rig.rows(id).await.into_iter().find_map(|row| match row {
        SessionRow::Delivery { said, .. } => said,
        _ => None,
    })
}

async fn kept(rig: &Rig, id: &SessionId) -> Option<DeliveryChoice> {
    eventually(|| async { delivery_of(&rig.rows(id).await).is_some() }).await;
    delivery_of(&rig.rows(id).await)
}

async fn answered(rig: &Rig, id: &SessionId, choice: u32) {
    eventually(|| async { question(rig, id).await.is_some() }).await;
    Arc::clone(&rig.fleet)
        .answer_waiting(AnswerWaiting { session_id: id.clone(), item_id: "delivery".into(), choice: Some(choice), text: None, mode: None, fixed: None })
        .await
        .expect("answered");
}

async fn delivery_of_project(rig: &Rig, project: Option<&str>) -> ipc::ResolvedValue {
    rig.fleet.values_asked(project.map(str::to_string), Some(rig.manifest.clone())).await.expect("read").delivery.expect("the key")
}

#[tokio::test]
async fn a_project_set_to_auto_land_answers_the_question_and_the_first_turn_carries_it() {
    let rig = rig();
    let project = project(&rig, Some("auto_land")).await;
    let id = first_turn_on_the_rail(&rig, Some(&project), FIRST).await;

    assert_eq!(kept(&rig, &id).await, Some(DeliveryChoice::AutoLand));
    assert!(question(&rig, &id).await.is_none(), "nothing is asked");
    assert!(rig.rows(&id).await.iter().any(|row| matches!(row, SessionRow::Delivery { named: false, .. })), "the setting answered, not the message");
    eventually(|| async { told(&rig, "this project's setting is Auto land. Open a pull request, watch ci, merge it.") == 1 }).await;
    assert_eq!(told(&rig, "choosing how this work is delivered"), 0, "the agent is not stopped");
    assert!(record(&rig, &id).await.auto_land, "the flag is on, as a named auto land turns it");
    assert_eq!(said(&rig, &id).await.as_deref(), Some(&*format!("Auto land, as Shop is set. {WHERE}")));
}

#[tokio::test]
async fn keep_local_and_open_pr_set_on_the_project_answer_without_asking() {
    for (word, choice, label) in [("keep_local", DeliveryChoice::KeepLocal, "Keep local"), ("open_pr", DeliveryChoice::OpenPr, "Open PR only")] {
        let rig = rig();
        let project = project(&rig, Some(word)).await;
        let id = entering_the_rail(&rig, Some(&project), FIRST).await;

        assert_eq!(kept(&rig, &id).await, Some(choice), "{word}");
        assert!(question(&rig, &id).await.is_none(), "{word}: nothing is asked");
        assert!(!record(&rig, &id).await.auto_land, "{word}: the flag stays off");
        eventually(|| async { told(&rig, &format!("this project's setting is {label}.")) == 1 }).await;
        assert_eq!(told(&rig, "choosing how this work is delivered"), 0, "{word}");
    }
}

#[tokio::test]
async fn kits_delivery_answers_for_a_job_in_no_project() {
    let rig = rig();
    rig.kit_values("version: 1\nvalues:\n  delivery: open_pr\n");
    let id = entering_the_rail(&rig, None, FIRST).await;

    assert_eq!(kept(&rig, &id).await, Some(DeliveryChoice::OpenPr));
    assert!(question(&rig, &id).await.is_none());
    assert_eq!(said(&rig, &id).await.as_deref(), Some(&*format!("Open PR only, as set in Settings. {WHERE}")));
    eventually(|| async { told(&rig, "the delivery setting is Open PR only.") == 1 }).await;
}

#[tokio::test]
async fn ask_on_the_project_asks_and_learns_nothing() {
    let rig = rig();
    let project = project(&rig, Some("ask")).await;
    let id = entering_the_rail(&rig, Some(&project), FIRST).await;
    eventually(|| async { question(&rig, &id).await.is_some() }).await;

    answered(&rig, &id, 2).await;
    assert_eq!(delivery_of(&rig.rows(&id).await), Some(DeliveryChoice::KeepLocal));
    let held = delivery_of_project(&rig, Some(&project)).await;
    assert_eq!((held.value.as_deref(), held.from), (Some("ask"), ValueLayer::Project), "the project still says ask");
    assert_eq!(said(&rig, &id).await, None, "nothing was kept, so nothing is said");
}

#[tokio::test]
async fn a_start_card_choice_beats_the_projects_value() {
    let rig = rig();
    let project = project(&rig, Some("auto_land")).await;
    watching(&rig);
    let id = rig.start_for(Some(&project), true).await;
    on_gate(&rig, &id).await;
    rig.send_unsettled(&id, FIRST).await;
    eventually(|| async { record(&rig, &id).await.proposal.is_none_or(|one| !one.reading) }).await;
    Arc::clone(&rig.fleet)
        .begin_session(BeginSession { session_id: id.clone(), delivery: Some(DeliveryChoice::KeepLocal), message: None })
        .await
        .expect("begun");

    eventually(|| async { told(&rig, "the owner chose Keep local") == 1 }).await;
    assert_eq!(delivery_of(&rig.rows(&id).await), Some(DeliveryChoice::KeepLocal));
    assert_eq!(told(&rig, "this project's setting"), 0, "the setting is not said over the card");
    assert!(!record(&rig, &id).await.auto_land);
    assert_eq!(said(&rig, &id).await, None);
}

#[tokio::test]
async fn a_first_message_naming_a_delivery_beats_the_projects_value() {
    let rig = rig();
    let project = project(&rig, Some("auto_land")).await;
    let id = entering_the_rail(&rig, Some(&project), "the log reader drops the last line of every file. Keep it local.").await;

    assert_eq!(kept(&rig, &id).await, Some(DeliveryChoice::KeepLocal));
    assert!(rig.rows(&id).await.iter().any(|row| matches!(row, SessionRow::Delivery { named: true, .. })));
    assert!(!record(&rig, &id).await.auto_land, "the message said keep it local");
    assert_eq!(told(&rig, "this project's setting"), 0);
    assert_eq!(said(&rig, &id).await, None);
}

#[tokio::test]
async fn the_first_answer_in_a_project_with_no_value_is_kept_as_its_value_and_said_once() {
    let rig = rig();
    let project = project(&rig, None).await;
    let id = entering_the_rail(&rig, Some(&project), FIRST).await;
    answered(&rig, &id, 1).await;

    let held = delivery_of_project(&rig, Some(&project)).await;
    assert_eq!((held.value.as_deref(), held.from), (Some("open_pr"), ValueLayer::Project));
    let listed = rig.fleet.list_projects().await.expect("listed");
    let values = listed.projects.last().and_then(|one| one.values.clone()).expect("its values");
    assert_eq!(values.pr_mode, Some(core_model::PrMode::Draft.into()), "what the project already set is kept");
    assert_eq!(said(&rig, &id).await.as_deref(), Some(&*format!("Kept for Shop: its Jobs land this way from now on. {WHERE}")));
    let rows = rig.rows(&id).await;
    assert_eq!(rows.iter().filter(|row| matches!(row, SessionRow::Delivery { said: Some(_), .. })).count(), 1, "said once");
}

#[tokio::test]
async fn the_next_job_in_that_project_is_not_asked() {
    let rig = rig();
    let project = project(&rig, None).await;
    let first = entering_the_rail(&rig, Some(&project), FIRST).await;
    answered(&rig, &first, 0).await;

    let next = entering_the_rail(&rig, Some(&project), FIRST).await;
    assert_eq!(kept(&rig, &next).await, Some(DeliveryChoice::AutoLand));
    assert!(question(&rig, &next).await.is_none());
    eventually(|| async { record(&rig, &next).await.auto_land }).await;
    assert_eq!(said(&rig, &next).await.as_deref(), Some(&*format!("Auto land, as Shop is set. {WHERE}")));
}

#[tokio::test]
async fn an_answer_in_no_project_learns_nothing() {
    let rig = rig();
    let project = project(&rig, None).await;
    let id = entering_the_rail(&rig, None, FIRST).await;
    answered(&rig, &id, 1).await;

    assert_eq!(delivery_of(&rig.rows(&id).await), Some(DeliveryChoice::OpenPr));
    assert_eq!(said(&rig, &id).await, None);
    assert_eq!(delivery_of_project(&rig, Some(&project)).await.value, None, "no project took it");
    assert_eq!(delivery_of_project(&rig, None).await.value, None);
}
