//! The claim, written before the code: **a `you_approved` gate closes on his Approve of an item the
//! agent raised with `approves: true`, and on nothing else.** Ask for changes, words alone, a dismiss,
//! an Approve given before the plan, and an Approve on an item that did not ask leave it open.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{AnswerWaiting, DismissWaiting, PlanGate, PlanGroupInput, SessionId, SessionRecord, SetWaitingFor, WaitingInput, WaitingOption};

use super::session_host::{rig_placing, Rig};
use super::session_plan::{group, plan, read, recorded, running};

fn gated(title: &str, gate: PlanGate) -> PlanGroupInput {
    PlanGroupInput { gate, ..group(None, title, "rust-engineer", &["crates/fleet"], &[(None, &format!("Do {title}"))]) }
}

fn asking(id: &str, approves: bool, options: &[&str]) -> WaitingInput {
    WaitingInput {
        id: id.into(),
        text: format!("Approve {id}?"),
        act: None,
        options: options.iter().map(|label| WaitingOption { label: (*label).into(), description: None }).collect(),
        approves,
        steps: Vec::new(),
    }
}

async fn waiting(rig: &Rig, id: &SessionId, items: Vec<WaitingInput>) -> SessionRecord {
    rig.fleet.waiting_for(None, SetWaitingFor { items, session_id: Some(id.clone()) }).await.expect("kept")
}

async fn answered(rig: &Rig, id: &SessionId, item: &str, choice: Option<u32>, text: Option<&str>) {
    // The clock reads milliseconds: let the answer land after the rows before it.
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let said = AnswerWaiting { session_id: id.clone(), item_id: item.into(), choice, text: text.map(str::to_string), mode: None, fixed: None };
    Arc::clone(&rig.fleet).answer_waiting(said).await.expect("answered");
}

fn closes(record: &SessionRecord) -> Vec<Option<String>> {
    record.plan.as_ref().expect("a plan").groups.iter().map(|group| group.gate_closed.as_ref().map(|one| one.fact.clone())).collect()
}

fn labels(record: &SessionRecord, item: &str) -> Vec<String> {
    let found = record.waiting_for.iter().find(|one| one.id == item).unwrap_or_else(|| panic!("{item} waits: {:?}", record.waiting_for));
    found.options.iter().map(|one| one.label.clone()).collect()
}

#[tokio::test]
async fn an_item_that_asks_for_his_approval_offers_approve_then_ask_for_changes_and_keeps_them() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let record = waiting(&rig, &id, vec![asking("design", true, &["Yes", "No", "Later"]), asking("plain", false, &["Yes", "No"])]).await;
    assert_eq!(labels(&record, "design"), ["Approve", "Ask for changes"], "the agent's own options are replaced");
    assert_eq!(labels(&record, "plain"), ["Yes", "No"], "an item that does not ask keeps its options");
    let none = waiting(&rig, &id, vec![asking("design", true, &[]), asking("plain", false, &[])]).await;
    assert_eq!(labels(&none, "design"), ["Approve", "Ask for changes"], "and gets them where it passed none");

    let kept = read(&rig, &id).await;
    let design = kept.waiting_for.iter().find(|one| one.id == "design").expect("kept");
    assert!(design.approves, "the mark is kept in the store, not only echoed back");
    assert_eq!(labels(&kept, "design"), ["Approve", "Ask for changes"]);
    let wire = ipc::encode(design).expect("encodes");
    assert!(wire.contains(r#""approves":true"#), "{wire}");
}

#[tokio::test]
async fn his_approve_closes_the_first_open_you_approved_gate_once_and_the_next_approve_the_next() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    let groups = vec![gated("Design", PlanGate::YouApproved), gated("Build", PlanGate::None), gated("Sign off", PlanGate::YouApproved)];
    recorded(&rig, plan(&id, groups)).await;
    waiting(&rig, &id, vec![asking("design", true, &[]), asking("sign-off", true, &[])]).await;
    assert!(rig.fleet.gates_kept(id.as_str()).await.is_empty(), "nothing is approved yet");

    answered(&rig, &id, "design", Some(0), None).await;
    let closed = rig.fleet.gates_kept(id.as_str()).await;
    assert_eq!(closed.iter().map(|one| one.group_id.as_str()).collect::<Vec<_>>(), ["g1"]);
    assert_eq!(closes(&read(&rig, &id).await), [Some("You approved \"Approve design?\"".into()), None, None]);
    assert!(rig.fleet.gates_kept(id.as_str()).await.is_empty(), "one Approve closes one gate, once");

    answered(&rig, &id, "sign-off", Some(0), Some("ship it")).await;
    let closed = rig.fleet.gates_kept(id.as_str()).await;
    assert_eq!(closed.iter().map(|one| one.group_id.as_str()).collect::<Vec<_>>(), ["g3"], "Approve with words is still Approve");
}

#[tokio::test]
async fn ask_for_changes_words_alone_and_a_dismiss_leave_the_gate_open() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    recorded(&rig, plan(&id, vec![gated("Design", PlanGate::YouApproved)])).await;
    waiting(&rig, &id, vec![asking("changes", true, &[]), asking("words", true, &[]), asking("dismissed", true, &[])]).await;

    answered(&rig, &id, "changes", Some(1), Some("rename the field")).await;
    answered(&rig, &id, "words", None, Some("Approve, looks good")).await;
    let dismiss = DismissWaiting { session_id: id.clone(), item_id: "dismissed".into(), reason: None, misread: false };
    Arc::clone(&rig.fleet).dismiss_waiting(dismiss).await.expect("dismissed");

    assert!(rig.fleet.gates_kept(id.as_str()).await.is_empty());
    assert_eq!(closes(&read(&rig, &id).await), [None]);
}

#[tokio::test]
async fn an_answer_in_his_place_is_refused_on_an_item_that_asks_for_his_approval() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    recorded(&rig, plan(&id, vec![gated("Design", PlanGate::YouApproved)])).await;
    waiting(&rig, &id, vec![asking("design", true, &[]), asking("plain", false, &["Yes", "No"])]).await;
    let said = |item: &str| AnswerWaiting { session_id: id.clone(), item_id: item.into(), choice: Some(0), text: None, mode: None, fixed: None };

    assert!(Arc::clone(&rig.fleet).settle_waiting(said("design")).await.is_err(), "the Operator cannot approve for him");
    assert_eq!(labels(&read(&rig, &id).await, "design"), ["Approve", "Ask for changes"], "and the item still waits on him");
    Arc::clone(&rig.fleet).settle_waiting(said("plain")).await.expect("an item that does not ask is still the Operator's to answer");
    assert!(rig.fleet.gates_kept(id.as_str()).await.is_empty());
}

#[tokio::test]
async fn an_approve_on_an_item_that_did_not_ask_or_before_the_plan_closes_nothing() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    waiting(&rig, &id, vec![asking("early", true, &[]), asking("plain", false, &["Approve", "Not yet"])]).await;
    answered(&rig, &id, "early", Some(0), None).await;
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    recorded(&rig, plan(&id, vec![gated("Design", PlanGate::YouApproved)])).await;
    answered(&rig, &id, "plain", Some(0), None).await;

    assert!(rig.fleet.gates_kept(id.as_str()).await.is_empty(), "one Approve came before the plan, one on an item that did not ask");
    assert_eq!(closes(&read(&rig, &id).await), [None]);
}
