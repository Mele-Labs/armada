//! The claim, written before the code: **a gate closing is told to the agent once, `all_done` is held
//! rather than refused while a gate is open and Fleet finishes the Job when the last one closes, and a
//! held gate only the person can close reaches Needs you while nothing in the Job runs.**
//! `docs/concepts/session/the-plan.md`, *Gates*.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use ipc::{
    AllDone, AnswerWaiting, AttachmentNamed, AttachmentReport, PlanGate, PlanGroupInput, SessionFact, SessionId, SessionRecord,
    SessionReport, WaitingItem, WaitingSource,
};

use super::session_host::{eventually, rig_placing, Rig};
use super::session_plan::{group, plan, read, recorded, running, told};
use crate::gate_checks::Ended;

const SHA: &str = "1b500c4e9a7d3f20c8b1a64d5e7f0392a1c4b8de";

fn gated(title: &str, gate: PlanGate) -> PlanGroupInput {
    PlanGroupInput { gate, ..group(None, title, "rust-engineer", &["crates/fleet"], &[(None, &format!("Do {title}"))]) }
}

async fn report(rig: &Rig, id: &SessionId, fact: SessionFact) {
    let harness = read(rig, id).await.harness;
    rig.fleet.report_session(SessionReport { harness, session_id: id.clone(), fact }).await.expect("reported");
}

async fn pull(rig: &Rig, id: &SessionId, number: &str, state: &str) {
    let detail = [("state".to_string(), state.to_string())].into_iter().collect();
    report(rig, id, SessionFact::Attached { attachment: AttachmentReport { kind: "pr".into(), target: number.into(), detail } }).await;
}

async fn subagent(rig: &Rig, id: &SessionId, working: bool) {
    let named = AttachmentNamed { kind: "subagent".into(), target: "x1".into() };
    let fact = match working {
        true => SessionFact::Attached { attachment: AttachmentReport { kind: "subagent".into(), target: "x1".into(), detail: Default::default() } },
        false => SessionFact::Settled { attachment: named, state: ipc::AttachmentState::Spent },
    };
    report(rig, id, fact).await;
}

async fn done(rig: &Rig, id: &SessionId) -> SessionRecord {
    rig.fleet.all_done(None, AllDone { session_id: Some(id.clone()), summary: Some("It shipped.".into()) }).await.expect("held or taken")
}

fn gate_item(record: &SessionRecord) -> Option<&WaitingItem> {
    record.waiting_for.iter().find(|one| one.source == WaitingSource::Gate)
}

async fn pressed(rig: &Rig, id: &SessionId, item: &str, choice: u32) -> SessionRecord {
    let said = AnswerWaiting { session_id: id.clone(), item_id: item.into(), choice: Some(choice), text: None, mode: None, fixed: None };
    Arc::clone(&rig.fleet).answer_waiting(said).await.expect("answered")
}

async fn with_gates(rig: &Rig, gates: &[(&str, PlanGate)]) -> SessionId {
    let id = running(rig).await;
    // The turn's start is heard on a task of its own, and a turn begun clears a call held: let it land.
    eventually(|| async { rig.fleet.hosts().of(id.as_str()).state().resolved_model.is_some() }).await;
    recorded(rig, plan(&id, gates.iter().map(|(title, gate)| gated(title, *gate)).collect())).await;
    id
}

#[tokio::test]
async fn a_gate_closing_tells_the_agent_once() {
    let rig = rig_placing(None);
    let id = with_gates(&rig, &[("Deliver", PlanGate::Merged)]).await;
    pull(&rig, &id, "3360", "merged").await;
    assert_eq!(rig.fleet.gates_kept(id.as_str()).await.len(), 1);
    eventually(|| async { told(&rig, "Pull request #3360 merged.") == 1 }).await;

    assert!(rig.fleet.gates_kept(id.as_str()).await.is_empty());
    pull(&rig, &id, "3360", "merged").await;
    rig.fleet.gates_kept(id.as_str()).await;
    assert_eq!(told(&rig, "Gate closed on"), 1, "a row written once is told once");
}

#[tokio::test]
async fn red_checks_tell_the_agent_what_failed_and_what_was_red_on_main() {
    let rig = rig_placing(None);
    let id = with_gates(&rig, &[("Build it", PlanGate::ChecksGreen)]).await;
    let names = |of: &[&str]| of.iter().map(|name| name.to_string()).collect::<Vec<_>>();
    let ended = Ended::Failed { failed: names(&["tests::a", "tests::b", "tests::c"]), red_at_base: names(&["tests::a"]) };
    rig.fleet.gate_checked(id.as_str(), SHA, ended).await;
    eventually(|| async { told(&rig, "Checks failed at 1b500c4: tests::b, tests::c. Already red on main: tests::a.") == 1 }).await;
}

#[tokio::test]
async fn all_done_is_held_while_a_gate_is_open_and_the_last_close_finishes_the_job() {
    let rig = rig_placing(None);
    let id = with_gates(&rig, &[("Find the cause", PlanGate::DocumentApproved), ("Deliver", PlanGate::Merged)]).await;
    let held = done(&rig, &id).await;
    assert!(held.done_since.is_none(), "a held call stamps nothing");
    let plan = held.plan.expect("a plan");
    let on = plan.held.expect("held");
    assert_eq!(on.group, "g1", "the first group whose gate is open");

    // A second call keeps the first instant.
    let again = done(&rig, &id).await.plan.and_then(|plan| plan.held).expect("held");
    assert_eq!(again.since, on.since);

    // The last gate closing is not the first: one still open holds it.
    pull(&rig, &id, "3360", "merged").await;
    rig.fleet.gates_kept(id.as_str()).await;
    let after = read(&rig, &id).await;
    assert!(after.done_since.is_none());
    assert_eq!(after.plan.as_ref().and_then(|plan| plan.held.clone()).map(|held| held.group), Some("g1".into()));

    // Skip the first through Needs you, which closes the last.
    let item = gate_item(&after).expect("the person can close a page's approval").id.clone();
    let record = pressed(&rig, &id, &item, 1).await;
    assert!(record.done_since.is_some(), "Fleet finished the Job");
    assert_eq!(record.done_summary.as_deref(), Some("It shipped."), "with the summary the agent gave");
    assert!(record.plan.expect("a plan").held.is_none());
}

#[tokio::test]
async fn a_call_held_stops_standing_when_he_writes() {
    let rig = rig_placing(None);
    let id = with_gates(&rig, &[("Deliver", PlanGate::Merged)]).await;
    done(&rig, &id).await;
    rig.send(&id, "one more thing").await;
    assert!(read(&rig, &id).await.plan.expect("a plan").held.is_none());
}

#[tokio::test]
async fn the_item_waits_for_nothing_to_run_and_goes_with_the_close() {
    let rig = rig_placing(None);
    let id = with_gates(&rig, &[("Deliver", PlanGate::Merged)]).await;
    subagent(&rig, &id, true).await;
    pull(&rig, &id, "3360", "open").await;
    done(&rig, &id).await;
    assert!(gate_item(&read(&rig, &id).await).is_none(), "a subagent runs, and a pull request is open");

    subagent(&rig, &id, false).await;
    assert!(gate_item(&read(&rig, &id).await).is_none(), "nothing runs, but an open pull request is on the way to merging");

    pull(&rig, &id, "3360", "merged").await;
    rig.fleet.gates_kept(id.as_str()).await;
    let record = read(&rig, &id).await;
    assert!(gate_item(&record).is_none(), "gone after the close");
    assert!(record.done_since.is_some());
}

#[tokio::test]
async fn a_gate_only_he_can_close_is_asked_when_nothing_runs() {
    let rig = rig_placing(None);
    let id = with_gates(&rig, &[("Draw it", PlanGate::SketchApproved)]).await;
    subagent(&rig, &id, true).await;
    done(&rig, &id).await;
    assert!(gate_item(&read(&rig, &id).await).is_none(), "a subagent runs");

    subagent(&rig, &id, false).await;
    let record = read(&rig, &id).await;
    let item = gate_item(&record).expect("nothing runs");
    assert_eq!(item.id, "gate:g1");
    assert_eq!(item.text, "\"Draw it\" waits on your approval of a sketch");
    assert_eq!(item.options.iter().map(|one| one.label.as_str()).collect::<Vec<_>>(), ["Approve", "Skip this group"]);
    assert_eq!(item.act.as_ref().map(|act| act.target.as_str()), Some(id.as_str()));
}

#[tokio::test]
async fn checks_nobody_has_started_are_asked_and_a_run_that_has_begun_is_not() {
    let rig = rig_placing(None);
    let id = with_gates(&rig, &[("Build it", PlanGate::ChecksGreen)]).await;
    done(&rig, &id).await;
    assert_eq!(gate_item(&read(&rig, &id).await).map(|one| one.text.as_str()), Some("\"Build it\" waits on Checks at a pushed commit"));
    assert!(rig.fleet.gate_check_began(id.as_str(), SHA).await);
    assert!(gate_item(&read(&rig, &id).await).is_none(), "its own Checks are running");
}

#[tokio::test]
async fn approve_closes_the_gate_on_his_word_and_skip_closes_it_skipped() {
    let rig = rig_placing(None);
    let id = with_gates(&rig, &[("Draw it", PlanGate::SketchApproved)]).await;
    done(&rig, &id).await;
    let record = pressed(&rig, &id, "gate:g1", 0).await;
    let close = record.plan.as_ref().expect("a plan").groups[0].gate_closed.clone().expect("closed");
    assert_eq!((close.fact.as_str(), close.skipped), ("You approved it in Needs you", false));
    assert!(record.done_since.is_some());
    eventually(|| async { told(&rig, "Gate closed on") == 1 && told(&rig, "You approved it in Needs you.") == 1 }).await;

    let rig = rig_placing(None);
    let id = with_gates(&rig, &[("Draw it", PlanGate::SketchApproved)]).await;
    done(&rig, &id).await;
    let record = pressed(&rig, &id, "gate:g1", 1).await;
    let close = record.plan.as_ref().expect("a plan").groups[0].gate_closed.clone().expect("closed");
    assert_eq!((close.fact.as_str(), close.skipped), ("You skipped this group", true));
    assert!(record.done_since.is_some());
}

#[tokio::test]
async fn approve_on_the_plan_gate_approves_the_plan() {
    let rig = rig_placing(None);
    let id = with_gates(&rig, &[("Find the cause", PlanGate::PlanApproved)]).await;
    done(&rig, &id).await;
    let record = pressed(&rig, &id, "gate:g1", 0).await;
    let plan = record.plan.expect("a plan");
    assert!(plan.approved, "the plan was approved");
    assert_eq!(plan.groups[0].gate_closed.as_ref().map(|one| one.fact.as_str()), Some("You approved plan v1"));
    assert!(record.done_since.is_some());
    eventually(|| async { told(&rig, "approved plan v1") == 2 }).await;
    assert_eq!(told(&rig, "Gate closed on"), 1, "the plan's approval and the gate's close are two lines");
}
