//! The claim, written before the code: **Fleet reads the step a Session is on from what it observed and
//! tells the agent nothing of it. Nothing blocks.**
//! `docs/concepts/session.md`, *The rail*.
//!
//! The first half is the rules, pure. The second drives a Fleet with a workflow and the judge test double.

use std::sync::Arc;

use adapter_traits::{CallDetail, DroneEvent};
use api::{HostedSessions, Refusal, Sessions};
use core_model::{CriterionId, JudgeCriterion, OnRefusal, StepEnd};
use ipc::{
    ApprovePlan, PlanGroupInput, PlanTaskInput, RecordPlan, SessionFact, SessionId, SessionRecord, SessionReport,
    SetSessionWorkflow,
};
use testkit::FakeJudge;

use super::session_host::{eventually, rig_fitted, Rig};
use crate::rail::{self, Facts, Fit, RailStep};
use crate::rail_ends;

// ---- The rules ----

/// What the Judge was asked. The proposer reads a Session's first message through the same client, and is
/// not a Judge's question.
fn asked_of(judge: &FakeJudge) -> Vec<String> {
    judge.asked().into_iter().filter(|one| !one.starts_with("You are deciding what kind of work")).collect()
}

pub(super) fn criterion() -> JudgeCriterion {
    JudgeCriterion {
        criterion_id: CriterionId::new("c1"),
        question: "Does the reader stop at the end?".into(),
        on_refusal: OnRefusal::Ask,
    }
}

pub(super) fn step(label: &str) -> RailStep {
    RailStep {
        id: label.to_lowercase().replace(' ', "_"),
        label: label.into(),
        about: None,
        instructions: None,
        records_plan: false,
        plan_unasked: false,
        follows_plan: false,
        delivers: false,
        asks_delivery: false,
        walks: false,
        ends_on: Vec::new(),
        criteria: Vec::new(),
        fit: None,
    }
}

pub(super) fn plan_step() -> RailStep {
    RailStep { records_plan: true, ..step("Plan the change") }
}

pub(super) fn follows(label: &str) -> RailStep {
    RailStep { follows_plan: true, criteria: vec![criterion()], ..step(label) }
}

pub(super) fn delivers(label: &str) -> RailStep {
    RailStep { delivers: true, ..step(label) }
}

pub(super) fn fitted(label: &str, fit: Fit) -> RailStep {
    RailStep { fit: Some(fit), ..step(label) }
}

pub(super) fn approved() -> Facts {
    Facts { plan_approved: true, ..Facts::default() }
}

pub(super) fn at(steps: &[RailStep], facts: Facts) -> Option<usize> {
    rail::current(steps, &facts)
}

#[test]
fn a_step_that_records_the_plan_is_current_until_a_plan_is_approved() {
    let steps = [plan_step(), follows("Implement"), delivers("Hand off")];
    assert_eq!(at(&steps, Facts::default()), Some(0), "the first step before anything is seen");
    assert_eq!(at(&steps, Facts { edited: true, ..Facts::default() }), Some(0), "an edit does not leave it");
    assert_eq!(at(&steps, approved()), Some(1));
}

#[test]
fn a_step_that_follows_the_plan_is_current_while_tasks_are_open() {
    let steps = [plan_step(), follows("Implement"), delivers("Hand off")];
    let working = Facts { tasks_open: true, edited: true, checked: true, committed: true, ..approved() };
    assert_eq!(at(&steps, working), Some(1), "edits, commits and checks do not move a step with open tasks");
    assert_eq!(at(&steps, Facts { tasks_open: false, ..working }), Some(1), "and with none open it stays until a later step is entered");
}

#[test]
fn a_delivering_step_is_current_from_the_first_push_until_its_pull_request_merges() {
    let steps = [plan_step(), follows("Implement"), delivers("Hand off")];
    assert_eq!(at(&steps, Facts { pushed: true, ..approved() }), Some(2));
    assert_eq!(at(&steps, Facts { pushed: true, pr_opened: true, ..approved() }), Some(2));
    assert_eq!(at(&steps, Facts { pushed: true, landed: true, ..approved() }), None, "merged, and no step follows");
    let more = [plan_step(), delivers("Hand off"), step("Retro")];
    assert_eq!(at(&more, Facts { pushed: true, landed: true, ..approved() }), Some(2), "the step after it is current once it merges");
}

#[test]
fn steps_between_the_anchors_advance_in_order_on_the_fact_that_fits_them() {
    let steps = [plan_step(), fitted("Build", Fit::Edited), fitted("Verify", Fit::Checked), delivers("Ship")];
    assert_eq!(at(&steps, Facts { edited: true, ..Facts::default() }), Some(0), "an edit before the plan is approved keeps the plan step");
    assert_eq!(at(&steps, Facts { edited: true, ..approved() }), Some(1));
    assert_eq!(at(&steps, Facts { edited: true, checked: true, ..approved() }), Some(2));
    assert_eq!(at(&steps, Facts { checked: true, ..approved() }), Some(2), "a check without an edit still fits the step it fits");
    assert_eq!(at(&steps, Facts { edited: true, checked: true, pushed: true, ..approved() }), Some(3));
}

#[test]
fn where_a_step_cannot_be_told_from_its_neighbour_the_earlier_one_is_kept() {
    let twin = [plan_step(), fitted("Build", Fit::Edited), fitted("Build again", Fit::Edited), step("Free text")];
    assert_eq!(at(&twin, Facts { edited: true, ..approved() }), Some(1), "two steps that fit one fact: the first");
    let all = Facts { edited: true, checked: true, committed: true, pr_opened: true, ..approved() };
    assert_eq!(at(&twin, all), Some(1), "and a step that fits nothing is never entered by itself");
}

// ---- Through a Fleet ----

pub(super) const REQUEST: &str = "the log reader drops the last line of every file";

pub(super) fn workflow() -> config::ResolvedWorkflow {
    let text = "version: 1\nworkflow_id: rail\nname: Rail\nsteps:\n  \
        - id: plan\n    label: \"Plan the change\"\n    instructions: \"Record the plan before editing.\"\n    evidence: {submitted: {type: plan}}\n    \
        mechanical_checks:\n      - { type: plan_recorded, min_tasks: 1 }\n    delivers: false\n    advance_gate: auto\n  \
        - id: implement\n    label: \"Implement\"\n    follows_plan: true\n    retry_limit: 2\n    \
        evidence: {submitted: {type: diff}}\n    mechanical_checks:\n      - { type: diff_nonempty }\n    delivers: false\n    \
        advance_gate: auto_if_judge_passes\n    judge_checks:\n      -\n        criteria:\n          \
        - criterion_id: c1\n            question: \"Does the reader stop at the end?\"\n            on_refusal: refuse\n  \
        - id: handoff\n    label: \"Hand off\"\n    delivers: true\n    advance_gate: auto\n";
    let def = config::WorkflowDef::parse(std::path::Path::new("rail.yml"), text, &config::Roster::offering_nothing())
        .unwrap_or_else(|refused| panic!("the fixture did not parse: {refused}"));
    config::ResolvedWorkflow::resolve(&def, &super::daemon::manifest())
        .unwrap_or_else(|refused| panic!("the fixture did not resolve: {refused}"))
}

pub(super) fn ruled_on(judge: Arc<FakeJudge>, workflow: config::ResolvedWorkflow) -> Rig {
    rig_fitted(|fittings| {
        fittings.judge = judge;
        fittings.starting().workflows = super::daemon::one(workflow);
    })
}

fn ruled(judge: Arc<FakeJudge>) -> Rig {
    rig_fitted(|fittings| {
        fittings.judge = judge;
        fittings.starting().workflows = super::daemon::one(workflow());
    })
}

pub(super) async fn record(rig: &Rig, id: &SessionId) -> SessionRecord {
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("read").session
}

/// A Session on the workflow with its process running, and the rail watching.
pub(super) async fn on_the_rail(rig: &Rig) -> SessionId {
    crate::timeline::keep_timeline(Arc::clone(&rig.fleet));
    crate::rail_following::keep_the_rail(Arc::clone(&rig.fleet));
    let id = rig.start().await;
    rig.send(&id, REQUEST).await;
    rig.stand_in.init(0);
    Arc::clone(&rig.fleet)
        .set_session_workflow(SetSessionWorkflow { session_id: id.clone(), workflow: Some("rail".into()) })
        .await
        .expect("set");
    id
}

pub(super) fn plan(id: &SessionId) -> RecordPlan {
    RecordPlan {
        summary: "You get a plan.".into(),
        groups: vec![PlanGroupInput {
            id: None,
            title: "Reader".into(),
            agent: "rust-engineer".into(),
            paths: vec!["crates/reader".into()],
            issue: None,
            gives: None,
            lands_in: None,
            own_job: false,
            tasks: vec![PlanTaskInput { id: None, title: "Stop at the end".into() }],
        }],
        sketch: None,
        walk: None,
        session_id: Some(id.clone()),
    }
}

pub(super) async fn approved_plan(rig: &Rig, id: &SessionId) {
    approved_plan_saying(rig, id, false).await;
}

/// As [`approved_plan`], the agent saying `seen`: the person would see the change.
pub(super) async fn approved_plan_saying(rig: &Rig, id: &SessionId, seen: bool) {
    rig.fleet.record_plan(None, plan(id), seen).await.expect("recorded");
    Arc::clone(&rig.fleet).approve_plan(ApprovePlan { session_id: id.clone(), version: 1 }).await.expect("approved");
}

pub(super) fn edit(path: &str) -> DroneEvent {
    DroneEvent::Called { tool: testkit::EDITS.into(), call: "toolu_1".into(), detail: CallDetail::of(path) }
}

pub(super) fn told(rig: &Rig, containing: &str) -> usize {
    rig.stand_in.process(0).sent.lock().unwrap().iter().filter(|line| line.contains(containing)).count()
}

async fn says_of_steps(rig: &Rig) -> Vec<String> {
    rig.fleet.timeline().settled().await;
    rig.fleet
        .fleet_timeline(ipc::TimelineAsk::default())
        .await
        .expect("a timeline")
        .events
        .into_iter()
        .filter(|one| one.kind.as_wire() == "session.step")
        .map(|one| one.says)
        .collect()
}

#[tokio::test]
async fn the_step_moves_on_what_was_observed_and_each_move_is_entered_on_the_timeline() {
    let rig = ruled(Arc::new(FakeJudge::with_no_objection()));
    let id = on_the_rail(&rig).await;
    assert_eq!(record(&rig, &id).await.workflow_step, Some(0), "the first step before anything is seen");

    approved_plan(&rig, &id).await;
    assert_eq!(record(&rig, &id).await.workflow_step, Some(1), "approval leaves the plan step");
    eventually(|| async { says_of_steps(&rig).await.contains(&"step: Plan the change → Implement".to_string()) }).await;
}

#[tokio::test]
async fn the_steps_a_job_began_are_sent_with_when_each_began_and_in_order() {
    let rig = ruled(Arc::new(FakeJudge::with_no_objection()));
    let id = on_the_rail(&rig).await;
    eventually(|| async { record(&rig, &id).await.workflow_visits.len() == 1 }).await;
    approved_plan(&rig, &id).await;
    eventually(|| async { record(&rig, &id).await.workflow_visits.len() == 2 }).await;

    let visits = record(&rig, &id).await.workflow_visits;
    assert_eq!(visits.iter().map(|one| one.step).collect::<Vec<_>>(), vec![0, 1]);
    assert!(visits[0].since <= visits[1].since, "oldest first: {visits:?}");
}

#[tokio::test]
async fn how_a_job_started_is_sent_and_a_later_tuning_does_not_change_it() {
    let rig = ruled(Arc::new(FakeJudge::with_no_objection()));
    let id = rig.start().await;
    rig.fleet
        .tune_session(ipc::TuneSession { session_id: id.clone(), model: Some("another-model".into()), effort: Some("high".into()), mode: ipc::SessionMode::Plan })
        .await
        .expect("tuned");
    rig.send(&id, REQUEST).await;
    rig.stand_in.init(0);
    let started = record(&rig, &id).await.started.expect("kept as the process first ran");
    assert_eq!(started.model.as_deref(), Some("another-model"));
    assert_eq!(started.effort.as_deref(), Some("high"));
    assert_eq!(started.mode, Some(ipc::SessionMode::Plan));

    rig.fleet
        .tune_session(ipc::TuneSession { session_id: id.clone(), model: Some("a-model".into()), effort: None, mode: ipc::SessionMode::Auto })
        .await
        .expect("tuned");
    assert_eq!(record(&rig, &id).await.started, Some(started), "what it began with stands");
}

/// A step is the Job's to end only where its own act ends it. One the person ends (a reply, an approval)
/// or a merge ends is his to move.
#[test]
fn a_step_is_the_jobs_to_end_only_where_its_own_act_ends_it() {
    let propose = RailStep { ends_on: vec![StepEnd::Replied], ..step("Propose") };
    let steps = [propose, step("Discuss"), plan_step(), follows("Refine"), delivers("Deliver")];
    let ends = (0..steps.len()).map(|at| rail_ends::end_of(&steps, at).map(|end| end.by_the_job())).collect::<Vec<_>>();
    assert_eq!(ends, [Some(false), Some(true), Some(false), Some(true), Some(false)], "reply, record, approve, push, merge");
}

// ---- Switched off ----

fn workflows_switched(rig: &Rig, on: bool) -> impl std::future::Future<Output = ()> + '_ {
    let save = ipc::SaveSettings {
        changes: std::collections::BTreeMap::from([("features.workflows".to_string(), Some(ipc::SettingValue::Bool(on)))]),
    };
    async move {
        api::Settings::save_settings(rig.fleet.as_ref(), save).await.expect("saved");
    }
}

#[tokio::test]
async fn with_workflows_off_the_proposer_is_shown_none_and_still_proposes_the_title() {
    let judge = Arc::new(FakeJudge::saying("workflow: bug\ntitle: Log reader drops a line\nsettings: effort=high"));
    let rig = rig_fitted(|fittings| {
        fittings.judge = Arc::clone(&judge) as _;
        fittings.starting().workflows = [super::daemon::workflow_named("bug")].into_iter().map(|one| (one.id().clone(), one)).collect();
    });
    workflows_switched(&rig, false).await;
    let id = rig.start_requesting(REQUEST, |_| {}).await.id;
    eventually(|| async { record(&rig, &id).await.proposal.is_none_or(|one| !one.reading) }).await;

    let read = record(&rig, &id).await;
    assert_eq!(read.workflow, None, "never picks a workflow, though the answer named one");
    assert_eq!(read.title.as_deref(), Some("Log reader drops a line"), "the title is still proposed");
    assert!(judge.asked()[0].contains("this repository holds none"), "{}", judge.asked()[0]);
}

#[tokio::test]
async fn with_workflows_off_a_workflow_cannot_be_set_but_can_be_cleared() {
    let rig = ruled(Arc::new(FakeJudge::with_no_objection()));
    let id = rig.start().await;
    workflows_switched(&rig, false).await;
    let set = |workflow: Option<&str>| {
        Arc::clone(&rig.fleet).set_session_workflow(SetSessionWorkflow { session_id: id.clone(), workflow: workflow.map(String::from) })
    };
    let refused = set(Some("rail")).await;
    assert!(matches!(&refused, Err(Refusal::Unacceptable(why)) if why.code == "fleet.session_workflows_off"), "{refused:?}");
    assert_eq!(record(&rig, &id).await.workflow, None);

    workflows_switched(&rig, true).await;
    assert_eq!(set(Some("rail")).await.expect("set").workflow.as_deref(), Some("rail"));
    workflows_switched(&rig, false).await;
    assert_eq!(set(None).await.expect("cleared").workflow, None, "turning a workflow off is always allowed");
}

#[tokio::test]
async fn with_workflows_off_there_is_no_step_and_no_boundary_judge() {
    let judge = Arc::new(FakeJudge::with_no_objection());
    let rig = ruled(Arc::clone(&judge));
    let id = on_the_rail(&rig).await;
    assert_eq!(record(&rig, &id).await.workflow_step, Some(0));
    // The rail enters step 0 once it sees the Session on the workflow, whenever the runtime lets it run.
    // Wait for that, so what is counted below is only what follows the switch.
    eventually(|| async { !says_of_steps(&rig).await.is_empty() }).await;
    let entered = says_of_steps(&rig).await;
    workflows_switched(&rig, false).await;

    rig.stand_in.says(0, vec![edit("crates/reader/src/lib.rs")]);
    approved_plan(&rig, &id).await;
    let harness = record(&rig, &id).await.harness;
    rig.fleet
        .report_session(SessionReport { harness, session_id: id.clone(), fact: SessionFact::Pushed { branch: Some("armada/reader".into()) } })
        .await
        .expect("kept");
    rig.fleet.timeline().settled().await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let read = record(&rig, &id).await;
    assert_eq!(read.workflow.as_deref(), Some("rail"), "the Session keeps the workflow it has");
    assert_eq!(read.workflow_step, None, "no current step");
    assert!(asked_of(&judge).is_empty(), "no boundary judge: {:?}", asked_of(&judge));
    assert_eq!(says_of_steps(&rig).await, entered, "and no step entered on the timeline after the switch");
}
