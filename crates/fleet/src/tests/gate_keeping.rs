//! The claim, written before the code: **a gate closes once, on a fact Fleet saw, and stays closed
//! whatever a later reading says; each gate is watched on its own, and one fact closes one gate.**
//! `docs/concepts/session/the-plan.md`, *Gates*.

use std::sync::Arc;

use api::Sessions;
use ipc::{
    ApprovePlan, AttachmentReport, PlanGate, PlanGroupInput, RecordPlan, SessionFact, SessionId, SessionRecord, SessionReport,
    ShowWindow, WaitingSource,
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
async fn a_version_after_his_approval_keeps_the_gate_closed_and_asks_again_only_when_the_agent_asks() {
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

    // No `ask_again`: v2 stands on his approval, so the gate stays closed and nothing waits on him.
    let again = [gated(Some("g1"), "Find the cause", PlanGate::PlanApproved), gated(Some("g2"), "Deliver", PlanGate::Merged)];
    let record = recorded(&rig, plan(&id, again.to_vec())).await;
    assert_eq!(closes(&record), [("g1".into(), Some("You approved plan v1".into())), ("g2".into(), None)], "v2 reopens nothing");
    let held = record.plan.as_ref().expect("a plan");
    assert!(held.approved, "a version that stands reads as approved, so Bridge draws no Approve");
    assert!(!record.waiting_for.iter().any(|one| one.source == WaitingSource::Plan), "v2 is not waiting on him");
    assert!(rig.fleet.gates_kept(id.as_str()).await.is_empty(), "and writes no second row");

    // `ask_again`: v3 is exactly what a new version was before: pending, its gate open, a new row when approved.
    let ask = RecordPlan { ask_again: true, ..plan(&id, again.to_vec()) };
    let record = recorded(&rig, ask).await;
    assert_eq!(closes(&record), [("g1".into(), None), ("g2".into(), None)], "v3 is his to approve again");
    assert!(!record.plan.as_ref().expect("a plan").approved);
    assert!(record.waiting_for.iter().any(|one| one.source == WaitingSource::Plan), "and it waits on him");
    Arc::clone(&rig.fleet).approve_plan(ApprovePlan { session_id: id.clone(), version: 3 }).await.unwrap();
    rig.fleet.gates_kept(id.as_str()).await;
    assert_eq!(closes(&read(&rig, &id).await)[0].1.as_deref(), Some("You approved plan v3"));

    // And v4, recorded without asking, stands on v3's approval.
    let record = recorded(&rig, plan(&id, again.to_vec())).await;
    assert_eq!(closes(&record)[0].1.as_deref(), Some("You approved plan v3"));
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
    Seen { gate, source: source.into(), group: None, at: at.into(), fact: format!("{source} at {at}"), commit: None }
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
    assert!(crate::gate_keeping::facts_seen(&plan, &before, &[], &[], &[]).is_empty());
    let after = [recorded, approved("m2", "2026-11-08T11:00:00.000Z")];
    let facts = crate::gate_keeping::facts_seen(&plan, &after, &[], &[], &[]);
    assert_eq!(facts.iter().map(|one| (one.source.as_str(), one.fact.as_str())).collect::<Vec<_>>(), [("sketch:m2", "You approved the sketch The flow")]);
    assert_eq!(closing(&plan, &[], &facts).len(), 1);
}

const BROWSER: &str = "http://localhost:5391/?walk=browser-tab";
const PROMPT: &str = "http://localhost:5391/?walk=permissions-prompt";

async fn walk(rig: &Rig, id: &SessionId, url: &str, group: Option<&str>) {
    let show = ShowWindow { url: url.into(), session_id: Some(id.clone()), group: group.map(str::to_string), ..Default::default() };
    Arc::clone(&rig.fleet).show_window(None, show).await.expect("shown");
}

async fn approve(rig: &Rig, id: &SessionId, url: &str) {
    // The clock reads milliseconds: let the next row land after the last.
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    rig.send(id, &format!("Approved: {url}")).await;
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
}

async fn two_parts(rig: &Rig) -> SessionId {
    let id = running(rig).await;
    let parts = [gated(None, "Browser tab", PlanGate::WalkApproved), gated(None, "Permissions prompt", PlanGate::WalkApproved)];
    recorded(rig, plan(&id, parts.to_vec())).await;
    id
}

fn closed_groups(record: &SessionRecord) -> Vec<bool> {
    record.plan.as_ref().expect("a plan").groups.iter().map(|group| group.gate_closed.is_some()).collect()
}

#[tokio::test]
async fn approving_the_walk_of_one_part_closes_that_groups_gate_while_the_other_part_still_waits() {
    let rig = rig_placing(None);
    let id = two_parts(&rig).await;
    walk(&rig, &id, BROWSER, Some("g1")).await;
    walk(&rig, &id, PROMPT, Some("g2")).await;
    assert!(rig.fleet.gates_kept(id.as_str()).await.is_empty(), "nothing is approved yet");

    approve(&rig, &id, BROWSER).await;
    let closed = rig.fleet.gates_kept(id.as_str()).await;
    assert_eq!(closed.iter().map(|one| one.group_id.as_str()).collect::<Vec<_>>(), ["g1"]);
    assert_eq!(closed_groups(&read(&rig, &id).await), [true, false], "the permissions prompt still waits");

    approve(&rig, &id, PROMPT).await;
    let closed = rig.fleet.gates_kept(id.as_str()).await;
    assert_eq!(closed.iter().map(|one| one.group_id.as_str()).collect::<Vec<_>>(), ["g2"]);
    assert_eq!(closed_groups(&read(&rig, &id).await), [true, true]);
}

#[tokio::test]
async fn a_group_the_plan_does_not_have_is_refused_with_the_ids_it_does() {
    let rig = rig_placing(None);
    let id = two_parts(&rig).await;
    let show = ShowWindow { url: BROWSER.into(), session_id: Some(id.clone()), group: Some("nope".into()), ..Default::default() };
    let refused = Arc::clone(&rig.fleet).show_window(None, show).await.expect_err("refused");
    let said = refused.error().message.clone();
    assert!(said.contains("`nope`") && said.contains("`g1`") && said.contains("`g2`"), "{said}");
    assert!(rig.fleet.gates_kept(id.as_str()).await.is_empty());
}

#[tokio::test]
async fn walks_shown_with_no_group_still_close_the_first_open_walk_gate_once_every_one_is_approved() {
    let rig = rig_placing(None);
    let id = two_parts(&rig).await;
    walk(&rig, &id, BROWSER, None).await;
    walk(&rig, &id, PROMPT, None).await;
    approve(&rig, &id, BROWSER).await;
    assert!(rig.fleet.gates_kept(id.as_str()).await.is_empty(), "one walk still waits");
    approve(&rig, &id, PROMPT).await;
    let closed = rig.fleet.gates_kept(id.as_str()).await;
    assert_eq!(closed.iter().map(|one| one.group_id.as_str()).collect::<Vec<_>>(), ["g1"], "the first open gate, as before");
}

#[tokio::test]
async fn a_new_walk_for_a_group_opens_its_gate_again_and_its_approval_writes_a_new_row_beside_the_old() {
    let rig = rig_placing(None);
    let id = two_parts(&rig).await;
    walk(&rig, &id, BROWSER, Some("g1")).await;
    approve(&rig, &id, BROWSER).await;
    let first = rig.fleet.gates_kept(id.as_str()).await;
    assert_eq!(first.len(), 1);
    assert_eq!(closed_groups(&read(&rig, &id).await), [true, false]);

    // The look changed: the agent shows a new walk for the same part.
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let newer = "http://localhost:5391/?walk=browser-tab-again";
    walk(&rig, &id, newer, Some("g1")).await;
    assert_eq!(closed_groups(&read(&rig, &id).await), [false, false], "the gate is open again");
    assert!(rig.fleet.gates_kept(id.as_str()).await.is_empty(), "and the old approval closes nothing");

    approve(&rig, &id, newer).await;
    let again = rig.fleet.gates_kept(id.as_str()).await;
    assert_eq!(again.iter().map(|one| (one.group_id.as_str(), one.version)).collect::<Vec<_>>(), [("g1", 1)]);
    assert_eq!(closed_groups(&read(&rig, &id).await), [true, false]);

    let rows = rig.fleet.store().lock().await.session_gates(id.as_str()).unwrap();
    assert_eq!(rows.len(), 2, "the old row stands beside the new one");
    assert_eq!(rows[0], first[0], "and is not edited");
}

#[test]
fn a_walk_gate_closed_again_tells_the_rules_the_gate_closed_and_ends_no_step_twice() {
    let plan = kept(&[gated(None, "Browser tab", PlanGate::WalkApproved), gated(None, "Tests", PlanGate::None)]);
    let row = |version: u32| KeptGate {
        group_id: "g1".into(),
        gate: "walk_approved".into(),
        version,
        closed_at: "2026-11-08T11:00:00.000Z".into(),
        fact: "f".into(),
        source: format!("walk:{version}"),
        commit: None,
        already_red: Vec::new(),
        skipped: false,
    };
    let first = crate::lifecycles::ruling::gate_moments(&plan.groups, &row(0));
    let again = crate::lifecycles::ruling::gate_moments(&plan.groups, &row(1));
    assert!(first.len() > 1, "the first close ends the stretch, closes the gate and starts the next");
    assert_eq!(again.len(), 1, "a close again says only that the gate closed");
}

/// The agent's own connection is pid 1000, as `started_jobs`'s rig places it.
fn a_rig_with_children() -> super::agent_starts::Rig {
    let peers = crate::tests::peer::Placing::nothing();
    peers.holding(1000, 50999, 12345);
    super::agent_starts::rig(Some(peers))
}

async fn child_of(rig: &super::agent_starts::Rig, title: &str) -> SessionId {
    let start = super::agent_starts::begins(rig, title, "Read crates/reader.");
    Arc::clone(&rig.fleet).start_another_session(super::agent_starts::caller(), start).await.expect("held").session_id
}

async fn withdrawn(rig: &super::agent_starts::Rig, child: &SessionId) {
    let command = ipc::StartedCommand { session_id: child.clone(), act: ipc::StartedAct::Withdraw, why: None, condition: None, base: None };
    let answers = Arc::clone(&rig.fleet).command_started_sessions(super::agent_starts::caller(), vec![command]).await.expect("placed");
    answers[0].as_ref().expect("withdrawn");
}

#[tokio::test]
async fn a_jobs_ended_group_stays_open_while_a_started_job_is_live_and_closes_when_the_last_one_ends() {
    let rig = a_rig_with_children();
    let agent = rig.an_agent().await;
    let groups = [gated(None, "Wait for the children", PlanGate::JobsEnded)];
    rig.fleet.record_plan(None, plan(&agent, groups.to_vec()), false).await.expect("recorded");
    assert!(rig.fleet.gates_kept(agent.as_str()).await.is_empty(), "no Job started: nothing has ended");

    let first = child_of(&rig, "Reader").await;
    let second = child_of(&rig, "Writer").await;
    assert!(rig.fleet.gates_kept(agent.as_str()).await.is_empty(), "both are live");

    withdrawn(&rig, &first).await;
    assert!(rig.fleet.gates_kept(agent.as_str()).await.is_empty(), "one is live still");
    assert_eq!(closes(&read_agent(&rig, &agent).await)[0].1, None);

    withdrawn(&rig, &second).await;
    let closed = rig.fleet.gates_kept(agent.as_str()).await;
    assert_eq!(closed.iter().map(|one| one.group_id.as_str()).collect::<Vec<_>>(), ["g1"]);
    assert_eq!(closes(&read_agent(&rig, &agent).await)[0].1.as_deref(), Some("Every Job it started has ended: Reader, Writer"));
    assert!(rig.fleet.gates_kept(agent.as_str()).await.is_empty(), "a close is kept once");
}

async fn read_agent(rig: &super::agent_starts::Rig, id: &SessionId) -> SessionRecord {
    rig.record(id).await
}

#[tokio::test]
async fn a_jobs_ended_gate_raises_no_needs_you_item_and_still_holds_all_done() {
    let rig = rig_placing(None);
    let id = running(&rig).await;
    super::session_plan::recorded(&rig, plan(&id, vec![gated(None, "Wait for the children", PlanGate::JobsEnded)])).await;

    let held = rig.fleet.all_done(None, ipc::AllDone { session_id: Some(id.clone()), summary: Some("It shipped.".into()) }).await.expect("held");
    assert!(held.done_since.is_none(), "the call is held while the gate is open");
    assert_eq!(held.plan.as_ref().and_then(|plan| plan.held.as_ref()).map(|on| on.group.as_str()), Some("g1"));
    assert!(held.waiting_for.iter().all(|one| one.source != WaitingSource::Gate), "it is not his to approve: {:?}", held.waiting_for);
}
