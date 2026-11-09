//! The claim, written before the code: **a gate closes once, on a fact Fleet saw, and stays closed
//! whatever a later reading says; each gate is watched on its own, and one fact closes one gate.**
//! `docs/concepts/session/the-plan.md`, *Gates*.

use std::sync::Arc;

use api::Sessions;
use ipc::{
    ApprovePlan, AttachmentReport, PlanGate, PlanGroupInput, SessionFact, SessionId, SessionRecord, SessionReport,
};
use store::{KeptGate, KeptPlan};

use super::session_host::{rig_placing, Rig};
use super::session_plan::{group, plan, read, recorded, running};
use crate::gate_keeping::{closing, Seen};
use crate::plan_record;

fn gated(id: Option<&str>, title: &str, gate: PlanGate) -> PlanGroupInput {
    PlanGroupInput { gate, ..group(id, title, "rust-engineer", &["crates/fleet"], &[(None, &format!("Do {title}"))]) }
}

fn closes(record: &SessionRecord) -> Vec<(String, Option<String>)> {
    let plan = record.plan.as_ref().expect("a plan");
    plan.groups.iter().map(|group| (group.id.clone(), group.gate_closed.as_ref().map(|one| one.fact.clone()))).collect()
}

async fn merged(rig: &Rig, id: &SessionId, number: &str, state: &str) {
    let harness = read(rig, id).await.harness;
    let detail = [("state".to_string(), state.to_string())].into_iter().collect();
    let fact = SessionFact::Attached { attachment: AttachmentReport { kind: "pr".into(), target: number.into(), detail } };
    rig.fleet.report_session(SessionReport { harness, session_id: id.clone(), fact }).await.expect("attached");
}

#[tokio::test]
async fn the_plans_approval_closes_its_gate_for_that_version_and_a_new_version_asks_it_again() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    let first = [gated(None, "Find the cause", PlanGate::PlanApproved), gated(None, "Deliver", PlanGate::Merged)];
    recorded(&rig, plan(&id, first.to_vec())).await;
    assert!(rig.fleet.gates_kept(id.as_str()).await.is_empty(), "nothing has happened yet");

    Arc::clone(&rig.fleet).approve_plan(ApprovePlan { session_id: id.clone(), version: 1 }).await.unwrap();
    let closed = rig.fleet.gates_kept(id.as_str()).await;
    assert_eq!(closed.iter().map(|one| (one.group_id.as_str(), one.version)).collect::<Vec<_>>(), [("g1", 1)]);
    assert_eq!(closes(&read(&rig, &id).await), [("g1".into(), Some("You approved plan v1".into())), ("g2".into(), None)]);
    assert!(rig.fleet.gates_kept(id.as_str()).await.is_empty(), "a close is kept once");

    let again = [gated(Some("g1"), "Find the cause", PlanGate::PlanApproved), gated(Some("g2"), "Deliver", PlanGate::Merged)];
    let record = recorded(&rig, plan(&id, again.to_vec())).await;
    assert_eq!(closes(&record), [("g1".into(), None), ("g2".into(), None)], "v2 is his to approve again");
    Arc::clone(&rig.fleet).approve_plan(ApprovePlan { session_id: id.clone(), version: 2 }).await.unwrap();
    rig.fleet.gates_kept(id.as_str()).await;
    assert_eq!(closes(&read(&rig, &id).await)[0].1.as_deref(), Some("You approved plan v2"));
}

#[tokio::test]
async fn a_merge_closes_its_gate_while_an_earlier_gate_is_open_and_stays_closed_when_the_ledger_reads_otherwise() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    let groups = [gated(None, "Find the cause", PlanGate::DocumentApproved), gated(None, "Deliver", PlanGate::Merged)];
    recorded(&rig, plan(&id, groups.to_vec())).await;

    merged(&rig, &id, "3360", "merged").await;
    rig.fleet.gates_kept(id.as_str()).await;
    let after = closes(&read(&rig, &id).await);
    assert_eq!(after, [("g1".into(), None), ("g2".into(), Some("Pull request #3360 merged".into()))]);

    // The rail fell back when a fact dropped out of its reading; a row does not.
    merged(&rig, &id, "3360", "open").await;
    rig.fleet.gates_kept(id.as_str()).await;
    assert_eq!(closes(&read(&rig, &id).await), after);
}

fn kept(groups: &[PlanGroupInput]) -> KeptPlan {
    let mut plan = plan_record::recorded(None, groups, None, None, false, "You get it.".into(), "2026-11-08T10:00:00.000Z");
    plan.approved_at = None;
    plan
}

fn seen(gate: PlanGate, source: &str, at: &str) -> Seen {
    Seen { gate, source: source.into(), at: at.into(), fact: format!("{source} at {at}"), commit: None }
}

#[test]
fn one_fact_closes_the_first_open_gate_of_its_kind_and_no_other() {
    let plan = kept(&[
        gated(None, "Design", PlanGate::DocumentApproved),
        gated(None, "Build", PlanGate::None),
        gated(None, "Write up", PlanGate::DocumentApproved),
    ]);
    let first = seen(PlanGate::DocumentApproved, "document:design.html@1", "2026-11-08T11:00:00.000Z");
    let closed = closing(&plan, &[], std::slice::from_ref(&first));
    assert_eq!(closed.iter().map(|one| one.group_id.as_str()).collect::<Vec<_>>(), ["g1"]);

    assert!(closing(&plan, &closed, &[first.clone()]).is_empty(), "a fact a row names closes nothing more");
    let second = seen(PlanGate::DocumentApproved, "document:report.html@2", "2026-11-08T12:00:00.000Z");
    let later = closing(&plan, &closed, &[first, second]);
    assert_eq!(later.iter().map(|one| one.group_id.as_str()).collect::<Vec<_>>(), ["g3"]);
}

#[test]
fn a_gate_kept_closed_names_its_group_its_kind_and_for_a_plan_its_version() {
    let row = |gate: &str, version: u32| KeptGate {
        group_id: "g1".into(),
        gate: gate.into(),
        version,
        closed_at: "2026-11-08T11:00:00.000Z".into(),
        fact: "f".into(),
        source: "s".into(),
        commit: None,
        already_red: Vec::new(),
        skipped: false,
    };
    assert!(row("merged", 0).closes("g1", "merged", 4), "a merge holds for every version");
    assert!(!row("merged", 0).closes("g1", "checks_green", 4), "a group whose gate changed is open");
    assert!(!row("merged", 0).closes("g2", "merged", 4));
    assert!(row("plan_approved", 3).closes("g1", "plan_approved", 3));
    assert!(!row("plan_approved", 3).closes("g1", "plan_approved", 4), "a new version is asked again");
}

#[test]
fn a_sketch_he_approved_before_there_was_a_plan_closes_nothing_and_one_after_does() {
    use ipc::{Instant, SessionRow, SessionVoice};
    let plan = kept(&[gated(None, "Sketch it", PlanGate::SketchApproved)]);
    let approved = |id: &str, at: &str| SessionRow::Message {
        id: id.into(),
        at: Instant::carried(at),
        from: SessionVoice::You,
        text: "Approved sketch: The flow\nGo on from it as drawn.".into(),
        files: Vec::new(),
        tags: Vec::new(),
        notice: None,
        queued: false,
    };
    let recorded = SessionRow::Plan { id: "p1".into(), at: Instant::carried("2026-11-08T10:00:00.000Z"), version: 1 };
    let before = [approved("m1", "2026-11-08T09:00:00.000Z"), recorded.clone()];
    assert!(crate::gate_keeping::facts_seen(&plan, &before, &[], &[]).is_empty());
    let after = [recorded, approved("m2", "2026-11-08T11:00:00.000Z")];
    let facts = crate::gate_keeping::facts_seen(&plan, &after, &[], &[]);
    assert_eq!(facts.iter().map(|one| (one.source.as_str(), one.fact.as_str())).collect::<Vec<_>>(), [("sketch:m2", "You approved the sketch The flow")]);
    assert_eq!(closing(&plan, &[], &facts).len(), 1);
}
