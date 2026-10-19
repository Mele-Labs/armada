//! The claim, written before the code: **a recorded plan is put to the person when he can answer
//! it.** Not while a page the Job showed is being changed, and, where it names a walk, not before he
//! has approved that walk. `docs/concepts/session/the-plan.md`.

use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Sessions};
use ipc::{
    PlanGroupInput, PlanTaskInput, RecordPlan, SessionFact, SessionId, SessionRecord, SessionReport, ShowWindow,
    WaitingItem, WaitingSource, WindowTake,
};

use super::session_host::{rig_placing, Rig};

const ONE: &str = "http://localhost:5173/one";
const TWO: &str = "http://localhost:5173/two";

fn plan(id: &SessionId, walk: Option<&str>) -> RecordPlan {
    RecordPlan {
        groups: vec![PlanGroupInput {
            id: None,
            title: "Store".into(),
            agent: "rust-engineer".into(),
            paths: vec!["crates/store".into()],
            tasks: vec![PlanTaskInput { id: None, title: "Add the table".into() }],
        }],
        sketch: None,
        walk: walk.map(String::from),
        session_id: Some(id.clone()),
    }
}

async fn record(rig: &Rig, id: &SessionId, walk: Option<&str>) {
    rig.fleet.record_plan(None, plan(id, walk)).await.expect("recorded");
}

async fn show(rig: &Rig, id: &SessionId, shown: ShowWindow) {
    rig.fleet.show_window(None, ShowWindow { session_id: Some(id.clone()), ..shown }).await.expect("shown");
}

async fn page(rig: &Rig, id: &SessionId, url: &str) {
    show(rig, id, ShowWindow { url: url.into(), ..Default::default() }).await;
}

async fn told(rig: &Rig, id: &SessionId, fact: SessionFact) {
    let report = SessionReport { harness: adapters::HOSTED_HARNESS.into(), session_id: id.clone(), fact };
    rig.fleet.report_session(report).await.expect("kept");
}

async fn read(rig: &Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session
}

async fn call(rig: &Rig, id: &SessionId) -> Option<WaitingItem> {
    read(rig, id).await.waiting_for.into_iter().find(|one| one.source == WaitingSource::Plan)
}

/// The clock reads milliseconds; a pause makes the next instant a later one.
async fn later() {
    tokio::time::sleep(Duration::from_millis(5)).await;
}

#[tokio::test]
async fn a_plan_recorded_while_a_page_changes_is_asked_once_the_page_is_ready() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    page(&rig, &id, ONE).await;
    told(&rig, &id, SessionFact::Wrote).await;

    record(&rig, &id, None).await;
    assert!(call(&rig, &id).await.is_none(), "the page the plan shows is being changed");
    assert!(read(&rig, &id).await.plan.is_some_and(|plan| !plan.approved), "and the plan is kept, pending");

    later().await;
    told(&rig, &id, SessionFact::TurnCompleted { said: None }).await;
    let record = read(&rig, &id).await;
    let ready = record.attachments.iter().find(|one| one.kind == "artifact").expect("the window").changed_at.clone();
    let item = call(&rig, &id).await.expect("ready, so it is asked");
    assert_eq!(item.id, "plan:1");
    assert_eq!(item.since.as_str(), ready.as_str(), "the call begins when the page became ready");
    assert!(item.since > record.plan.expect("a plan").recorded_at, "later than the recording");

    told(&rig, &id, SessionFact::Wrote).await;
    assert!(call(&rig, &id).await.is_none(), "changing the page again takes it back");
}

#[tokio::test]
async fn a_plan_with_no_page_shown_is_asked_at_its_recording() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    record(&rig, &id, None).await;
    let record = read(&rig, &id).await;
    let item = call(&rig, &id).await.expect("asked");
    assert_eq!(item.since.as_str(), record.plan.expect("a plan").recorded_at.as_str());
}

#[tokio::test]
async fn a_plan_naming_a_walk_is_asked_only_after_his_approve_on_it() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    record(&rig, &id, Some(ONE)).await;
    assert!(call(&rig, &id).await.is_none(), "the walk was never shown");

    rig.send(&id, &format!("Approved: {ONE}")).await;
    assert!(call(&rig, &id).await.is_none(), "an Approve before the page was shown approves nothing");

    later().await;
    page(&rig, &id, ONE).await;
    assert!(call(&rig, &id).await.is_none(), "shown and not approved");
    rig.send(&id, "Looks close, but the header is off").await;
    rig.send(&id, &format!("Approved: {TWO}")).await;
    assert!(call(&rig, &id).await.is_none(), "an Approve of another page is not this walk's");

    later().await;
    rig.send(&id, &format!("Approved: {ONE}")).await;
    let item = call(&rig, &id).await.expect("approved, so it is asked");
    assert!(item.since.as_str() > read(&rig, &id).await.plan.expect("a plan").recorded_at.as_str());

    later().await;
    page(&rig, &id, ONE).await;
    assert!(call(&rig, &id).await.is_none(), "shown again, it is to be approved again");
}

#[tokio::test]
async fn an_approve_of_a_take_of_the_window_is_the_walks() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let takes = [("Every state", ONE), ("Needs you first", TWO)]
        .map(|(name, url)| WindowTake { name: name.into(), url: url.into() })
        .into();
    show(&rig, &id, ShowWindow { url: ONE.into(), variations: takes, ..Default::default() }).await;
    record(&rig, &id, Some(ONE)).await;
    assert!(call(&rig, &id).await.is_none(), "shown and not approved");

    rig.send(&id, &format!("Approved B, Needs you first: {TWO}")).await;
    assert!(call(&rig, &id).await.is_some(), "the second take's Approve approves the window");
}

#[tokio::test]
async fn recording_again_without_a_walk_clears_it() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    record(&rig, &id, Some(ONE)).await;
    assert!(call(&rig, &id).await.is_none());

    record(&rig, &id, None).await;
    assert_eq!(call(&rig, &id).await.map(|item| item.id), Some("plan:2".to_string()));
}
