//! The claim, written before the code: **one approval moves a Job on. A recording step ends on the plan
//! or on anything it lists, and a plan recorded after one of those stands with no card. A step that says
//! `plan_unasked` never asks. A walk step ends only when every walk the Job showed is approved, and a
//! rough walk he approved is that walk.** The owner's decision of 2 Nov 2026, in
//! `.claude/decisions/2026-10-27-a-feature-job-walks-before-it-tests.md`.
//!
//! The first half is the rules, pure. The second drives a Fleet with a workflow shaped as `feature` is.

use std::sync::Arc;

use api::Sessions;
use core_model::StepEnd;
use ipc::{AnswerSketch, Instant, SessionId, SessionRow, SessionVoice, ShowSketch, ShowWindow, SketchHow};
use testkit::FakeJudge;

use super::session_rail::{delivers, follows, on_the_rail, plan, plan_step, record, ruled_on, step, told};
use crate::rail::{self, Extra, RailStep};
use crate::rail_ends::ends_any;
use crate::rail_keeping::thread_facts;

// ---- The rules ----

fn proposing() -> RailStep {
    RailStep { ends_on: vec![StepEnd::SketchApproved, StepEnd::WalkApproved], ..plan_step() }
}

fn feature() -> Vec<RailStep> {
    vec![proposing(), RailStep { walks: true, ..step("Walk") }, follows("Refine"), delivers("Deliver")]
}

#[test]
fn a_recording_step_says_it_ends_on_the_plan_or_anything_it_lists() {
    assert_eq!(ends_any(&feature(), 0), ["you approve the plan", "you approve a sketch", "you approve a walk"]);
    assert_eq!(ends_any(&[plan_step(), delivers("Deliver")], 0), ["you approve the plan"], "a step that lists nothing");
}

#[test]
fn a_step_that_leaves_the_plan_unasked_ends_when_one_is_recorded_or_as_a_step_that_follows_it() {
    let unasked = RailStep { plan_unasked: true, ..plan_step() };
    assert_eq!(ends_any(&[unasked.clone(), follows("Present")], 0), ["a plan is recorded"]);
    let reverting = RailStep { follows_plan: true, ..unasked };
    assert_eq!(ends_any(&[reverting, delivers("Summarise")], 0), ["it is pushed"], "its own work is the rest of the step");
}

#[test]
fn a_plan_stands_only_where_the_step_does_not_ask_or_he_approved_what_it_lists() {
    assert!(!rail::plan_stands(&feature(), Extra::default()), "nothing approved yet: it is asked");
    assert!(rail::plan_stands(&feature(), Extra { sketch_approved: true, ..Extra::default() }));
    assert!(rail::plan_stands(&feature(), Extra { any_walk_approved: true, ..Extra::default() }));
    assert!(!rail::plan_stands(&feature(), Extra { draft_approved: true, ..Extra::default() }), "the step does not list a draft");
    assert!(!rail::plan_stands(&[plan_step()], Extra { sketch_approved: true, ..Extra::default() }), "a step that lists nothing still asks");
    assert!(rail::plan_stands(&[RailStep { plan_unasked: true, ..plan_step() }], Extra::default()));
}

fn at_second(n: u32) -> Instant {
    Instant::carried(&format!("2026-11-02T10:00:{n:02}Z"))
}

fn window(n: u32, url: &str) -> SessionRow {
    SessionRow::Window {
        id: format!("w{n}"),
        at: at_second(n),
        title: "Mock".into(),
        url: url.into(),
        variations: Vec::new(),
        steps: Vec::new(),
        origins: Vec::new(),
        ended_at: None,
    }
}

fn approve(n: u32, url: &str) -> SessionRow {
    SessionRow::Message {
        id: format!("m{n}"),
        at: at_second(n),
        from: SessionVoice::You,
        text: format!("Approved: {url}"),
        files: Vec::new(),
        tags: Vec::new(),
        notice: None,
        queued: false,
    }
}

const A: &str = "http://localhost:5391/?walk=a";
const B: &str = "http://localhost:5391/?walk=b";
const C: &str = "http://localhost:5391/?walk=c";

#[test]
fn a_walk_step_passes_only_when_every_walk_shown_since_the_plan_is_approved() {
    let plan = at_second(1);
    let shown = [window(2, A), window(3, B), window(4, C)];
    let with = |approves: &[SessionRow]| {
        let rows: Vec<SessionRow> = shown.iter().chain(approves).cloned().collect();
        thread_facts(&rows, Some(&plan)).walk_approved
    };
    assert!(!with(&[]));
    assert!(!with(&[approve(5, A)]), "one of three");
    assert!(!with(&[approve(5, A), approve(6, B)]), "two of three");
    assert!(with(&[approve(5, A), approve(6, B), approve(7, C)]));
}

#[test]
fn a_window_replaced_by_a_newer_one_of_its_page_waits_on_nothing() {
    let plan = at_second(1);
    let moved = "http://localhost:6000/?walk=a";
    let rows = [window(2, A), window(3, moved), approve(4, moved)];
    assert!(thread_facts(&rows, Some(&plan)).walk_approved);
}

#[test]
fn a_walk_shown_again_after_his_approve_waits_again() {
    let plan = at_second(1);
    let rows = [window(2, A), approve(3, A), window(4, B)];
    assert!(!thread_facts(&rows, Some(&plan)).walk_approved, "a later walk puts the Job back on Walk");
}

// ---- Through a Fleet ----

fn proposing_workflow(propose: &str) -> config::ResolvedWorkflow {
    let text = format!(
        "version: 1\nworkflow_id: rail\nname: Rail\nsteps:\n  \
        - id: propose\n    label: Propose\n    evidence: {{submitted: {{type: plan}}}}\n    {propose}\n    \
        mechanical_checks:\n      - {{ type: plan_recorded, min_tasks: 1 }}\n    delivers: false\n    advance_gate: auto\n  \
        - id: walk\n    label: Walk\n    walks: true\n    delivers: false\n    advance_gate: auto\n  \
        - id: refine\n    label: Refine\n    follows_plan: true\n    delivers: false\n    advance_gate: auto\n  \
        - id: handoff\n    label: \"Hand off\"\n    delivers: true\n    advance_gate: auto\n"
    );
    let def = config::WorkflowDef::parse(std::path::Path::new("rail.yml"), &text, &config::Roster::offering_nothing())
        .unwrap_or_else(|refused| panic!("the fixture did not parse: {refused}"));
    config::ResolvedWorkflow::resolve(&def, &super::daemon::manifest())
        .unwrap_or_else(|refused| panic!("the fixture did not resolve: {refused}"))
}

const LISTS: &str = "ends_on: [sketch_approved, walk_approved]";
const PAGE: &str = "http://localhost:5391/?walk=reader";
const OTHER: &str = "http://localhost:5391/?walk=other";

async fn rig_with(propose: &str) -> (super::session_host::Rig, SessionId) {
    let rig = ruled_on(Arc::new(FakeJudge::with_no_objection()), proposing_workflow(propose));
    let id = on_the_rail(&rig).await;
    (rig, id)
}

async fn show(rig: &super::session_host::Rig, id: &SessionId, url: &str) {
    rig.fleet
        .show_window(None, ShowWindow { url: url.into(), title: None, session_id: Some(id.clone()), ..Default::default() })
        .await
        .expect("shown");
}

async fn sketch_approved(rig: &super::session_host::Rig, id: &SessionId) {
    let drawing: ShowSketch = ipc::decode(
        "a sketch to show",
        format!(
            r#"{{"title":"Flow","session_id":"{}","scene":{{"nodes":[{{"id":"a","kind":"box","x":0,"y":0,"title":"Bridge"}}],"edges":[]}}}}"#,
            id.as_str()
        )
        .as_bytes(),
    )
    .expect("a sketch");
    let shown = rig.fleet.show_sketch(None, drawing).await.expect("drawn");
    let drawn = shown.attachments.iter().find(|one| one.target == "sketch:Flow").and_then(|one| one.detail.get("drawn")).cloned().expect("drawn");
    Arc::clone(&rig.fleet)
        .answer_sketch(AnswerSketch { session_id: id.clone(), sketch: "sketch:Flow".into(), drawn, how: SketchHow::Approved, words: None })
        .await
        .expect("approved");
}

fn plan_is_asked(record: &ipc::SessionRecord) -> bool {
    record.waiting_for.iter().any(|one| one.source == ipc::WaitingSource::Plan)
}

#[tokio::test]
async fn with_nothing_approved_a_plan_is_put_to_him_and_its_approval_moves_the_job_on() {
    let (rig, id) = rig_with(LISTS).await;
    let recorded = rig.fleet.record_plan(None, plan(&id), true).await.expect("recorded");
    assert!(plan_is_asked(&recorded), "{:?}", recorded.waiting_for);
    assert_eq!(recorded.workflow_step, Some(0));
    Arc::clone(&rig.fleet).approve_plan(ipc::ApprovePlan { session_id: id.clone(), version: 1 }).await.expect("approved");
    assert_eq!(record(&rig, &id).await.workflow_step, Some(1), "on to Walk");
}

#[tokio::test]
async fn a_plan_recorded_after_an_approved_sketch_stands_with_no_card_and_a_walk_still_comes() {
    let (rig, id) = rig_with(LISTS).await;
    sketch_approved(&rig, &id).await;
    assert_eq!(record(&rig, &id).await.workflow_step, Some(0), "Propose holds until the tasks are recorded");
    let recorded = rig.fleet.record_plan(None, plan(&id), true).await.expect("recorded");
    assert!(!plan_is_asked(&recorded), "{:?}", recorded.waiting_for);
    assert!(recorded.plan.as_ref().is_some_and(|plan| plan.approved), "it stands");
    assert_eq!(recorded.workflow_step, Some(1), "on to Walk, not past it");
}

#[tokio::test]
async fn a_job_stopped_after_his_approval_with_no_plan_is_nudged_to_record_it() {
    let (rig, id) = rig_with(LISTS).await;
    let mut stopped = record(&rig, &id).await;
    stopped.workflow_step = Some(0);
    stopped.standing = Some(ipc::SessionStanding::Replied);
    stopped.waiting_for.clear();
    let steps = feature();
    assert!(rail::stalled(&steps, &stopped, Extra::default()).is_none(), "nothing approved: the proposal is his to answer");

    let approved = Extra { sketch_approved: true, ..Extra::default() };
    let (at, end) = rail::stalled(&steps, &stopped, approved).expect("stopped with his approval and no tasks");
    assert_eq!((at, end.clone()), (0, crate::rail_ends::End::PlanRecorded));
    let nudge = rail::stall_nudge("rail", &steps[0], end);
    assert!(nudge.contains("record_plan"), "{nudge}");

    stopped.plan = rig.fleet.record_plan(None, plan(&id), true).await.expect("recorded").plan;
    assert!(rail::stalled(&steps, &stopped, approved).is_none(), "the plan is recorded, so Fleet moves it on");
}

#[tokio::test]
async fn a_rough_walk_he_approved_is_the_walk_and_no_second_one_is_asked() {
    let (rig, id) = rig_with(LISTS).await;
    show(&rig, &id, PAGE).await;
    rig.send(&id, &format!("Approved: {PAGE}")).await;
    let recorded = rig.fleet.record_plan(None, plan(&id), true).await.expect("recorded");
    assert!(!plan_is_asked(&recorded), "{:?}", recorded.waiting_for);
    assert_eq!(recorded.workflow_step, Some(2), "past Walk to Refine");
}

#[tokio::test]
async fn several_rough_walks_all_wait_once_one_of_them_is_approved() {
    let (rig, id) = rig_with(LISTS).await;
    show(&rig, &id, PAGE).await;
    show(&rig, &id, OTHER).await;
    rig.send(&id, &format!("Approved: {PAGE}")).await;
    rig.fleet.record_plan(None, plan(&id), true).await.expect("recorded");
    assert_eq!(record(&rig, &id).await.workflow_step, Some(1), "the other walk still waits");
    rig.send(&id, &format!("Approved: {OTHER}")).await;
    assert_eq!(record(&rig, &id).await.workflow_step, Some(2));
}

#[tokio::test]
async fn a_plan_held_behind_a_walk_stands_with_his_approve_of_it() {
    let (rig, id) = rig_with(LISTS).await;
    show(&rig, &id, PAGE).await;
    let held = rig.fleet.record_plan(None, plan(&id), true).await.expect("recorded");
    assert!(!plan_is_asked(&held), "held behind the walk");
    rig.send(&id, &format!("Approved: {PAGE}")).await;
    let after = record(&rig, &id).await;
    assert!(!plan_is_asked(&after), "{:?}", after.waiting_for);
    assert!(after.plan.as_ref().is_some_and(|plan| plan.approved), "it stands");
    assert_eq!(after.workflow_step, Some(2), "that walk was the walk");
    assert_eq!(told(&rig, "approved plan"), 1, "the Job is told once");
}

#[tokio::test]
async fn a_plan_already_put_to_him_stays_his_to_answer_when_he_approves_a_sketch() {
    let (rig, id) = rig_with(LISTS).await;
    rig.fleet.record_plan(None, plan(&id), true).await.expect("recorded");
    sketch_approved(&rig, &id).await;
    assert!(plan_is_asked(&record(&rig, &id).await), "both were proposed, and he has answered one");
}

#[tokio::test]
async fn a_step_that_leaves_the_plan_unasked_never_puts_it_to_him() {
    let (rig, id) = rig_with("plan_unasked: true").await;
    let recorded = rig.fleet.record_plan(None, plan(&id), false).await.expect("recorded");
    assert!(!plan_is_asked(&recorded), "{:?}", recorded.waiting_for);
    assert!(recorded.plan.as_ref().is_some_and(|plan| plan.approved));
}

#[tokio::test]
async fn a_step_that_lists_nothing_still_asks_after_a_sketch() {
    let (rig, id) = rig_with("about: \"Plans.\"").await;
    sketch_approved(&rig, &id).await;
    let recorded = rig.fleet.record_plan(None, plan(&id), true).await.expect("recorded");
    assert!(plan_is_asked(&recorded), "bug and its kin are untouched");
}

#[tokio::test]
async fn three_walks_hold_the_job_on_walk_until_the_third_is_approved() {
    let (rig, id) = rig_with(LISTS).await;
    rig.fleet.record_plan(None, plan(&id), true).await.expect("recorded");
    Arc::clone(&rig.fleet).approve_plan(ipc::ApprovePlan { session_id: id.clone(), version: 1 }).await.expect("approved");
    for url in [A, B, C] {
        show(&rig, &id, url).await;
    }
    for url in [A, B] {
        rig.send(&id, &format!("Approved: {url}")).await;
        assert_eq!(record(&rig, &id).await.workflow_step, Some(1), "{url} is not the last");
    }
    rig.send(&id, &format!("Approved: {C}")).await;
    assert_eq!(record(&rig, &id).await.workflow_step, Some(2));
}

#[tokio::test]
async fn a_walk_taken_back_no_longer_holds_the_step() {
    let (rig, id) = rig_with(LISTS).await;
    rig.fleet.record_plan(None, plan(&id), true).await.expect("recorded");
    Arc::clone(&rig.fleet).approve_plan(ipc::ApprovePlan { session_id: id.clone(), version: 1 }).await.expect("approved");
    show(&rig, &id, A).await;
    show(&rig, &id, B).await;
    rig.send(&id, &format!("Approved: {A}")).await;
    assert_eq!(record(&rig, &id).await.workflow_step, Some(1));
    rig.fleet
        .withdraw_window(None, ipc::WithdrawWindow { session_id: Some(id.clone()), url: B.into() })
        .await
        .expect("taken back");
    assert_eq!(record(&rig, &id).await.workflow_step, Some(2));
}
