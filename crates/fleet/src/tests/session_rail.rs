//! The claim, written before the code: **Fleet reads the step a Session is on from what it observed and nudges
//! the agent once about a step it skipped. Nothing blocks.**
//! `docs/concepts/session.md`, *The rail*.
//!
//! The first half is the rules, pure. The second drives a Fleet with a workflow and the judge test double.

use std::sync::Arc;

use adapter_traits::{CallDetail, DroneEvent};
use api::{HostedSessions, Refusal, Sessions};
use core_model::{CriterionId, JudgeCriterion, OnRefusal, StepEnd};
use ipc::{
    ApprovePlan, PlanGroupInput, PlanTaskInput, RecordPlan, SessionFact, SessionId, SessionRecord, SessionReport,
    SetSessionWorkflow, WaitingSource,
};
use testkit::FakeJudge;

use super::session_host::{eventually, rig_fitted, Rig};
use crate::rail::{self, Facts, Fit, RailStep, Skip};
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

#[test]
fn an_edit_before_a_plan_is_approved_and_a_push_with_tasks_open_are_the_two_skips() {
    let steps = [plan_step(), follows("Implement"), delivers("Hand off")];
    assert_eq!(rail::skipped(&steps, &Facts { edited: true, ..Facts::default() }), [(0, Skip::Plan)]);
    assert!(rail::skipped(&steps, &approved()).is_empty());
    let pushed = Facts { pushed: true, tasks_open: true, ..approved() };
    assert_eq!(rail::skipped(&steps, &pushed), [(1, Skip::Tasks)]);
    assert!(rail::skipped(&steps, &Facts { pushed: true, ..approved() }).is_empty(), "tasks done");

    let said = rail::nudge("Feature", &steps[0], Skip::Plan);
    for part in ["Feature", "Plan the change", "record_plan", "Nothing is blocked"] {
        assert!(said.contains(part), "{part}: {said}");
    }
    let about = RailStep { about: Some("Write down what will change, and wait.".into()), ..plan_step() };
    assert!(rail::nudge("Feature", &about, Skip::Plan).contains("Write down what will change"), "the step's own words");
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

#[test]
fn entering_a_step_sends_its_instructions_and_a_step_with_none_sends_nothing() {
    let told = RailStep { instructions: Some("  Record the plan first.\n".into()), ..plan_step() };
    let said = rail::instructed("Bug", &[told], 0).expect("instructions are sent");
    for part in ["Bug workflow", "\"Plan the change\"", "Record the plan first.", "ends when the person approves the plan."] {
        assert!(said.contains(part), "{part}: {said}");
    }
    assert_eq!(rail::instructed("Bug", &[plan_step()], 0), None, "no instructions, no message");
    let blank = RailStep { instructions: Some("  ".into()), ..plan_step() };
    assert_eq!(rail::instructed("Bug", &[blank], 0), None, "blank is none");
}

/// **A step only the Job's own act ends says which act.** The Logbook Job (27 Oct 2026) settled the
/// scope on Discuss and waited for Fleet to move it, while Fleet waited for its plan.
#[test]
fn a_step_before_the_plan_says_it_ends_when_the_job_records_one() {
    let discuss = RailStep { instructions: Some("Ask what is unclear.".into()), ..step("Discuss") };
    let propose = RailStep { ends_on: vec![StepEnd::Replied], ..discuss.clone() };
    let steps = [propose, discuss, plan_step()];
    let said = rail::instructed("feature", &steps, 1).expect("instructions are sent");
    assert!(said.ends_with("This step ends when you record a plan with record_plan."), "{said}");
    let said = rail::instructed("feature", &steps, 0).expect("instructions are sent");
    assert!(said.ends_with("This step ends when the person replies."), "{said}");
}

#[tokio::test]
async fn a_session_is_told_a_steps_instructions_when_it_enters_the_step() {
    let rig = ruled(Arc::new(FakeJudge::with_no_objection()));
    let _id = on_the_rail(&rig).await;
    eventually(|| async { told(&rig, "Record the plan before editing.") == 1 }).await;
    let said = rig.stand_in.process(0).sent.lock().unwrap().iter().find(|line| line.contains("Record the plan")).cloned().unwrap();
    assert!(said.contains("Plan the change"), "{said}");
    assert_eq!(told(&rig, "Record the plan before editing."), 1, "once");
}

/// The rail's workflow with a step before the plan that nobody but the Job ends, as the feature
/// workflow's Discuss is.
fn discussed() -> config::ResolvedWorkflow {
    let text = "version: 1\nworkflow_id: rail\nname: Rail\nsteps:\n  \
        - id: discuss\n    label: \"Discuss\"\n    instructions: \"Settle the scope.\"\n    delivers: false\n    advance_gate: auto\n  \
        - id: plan\n    label: \"Plan the change\"\n    evidence: {submitted: {type: plan}}\n    \
        mechanical_checks:\n      - { type: plan_recorded, min_tasks: 1 }\n    delivers: false\n    advance_gate: auto\n  \
        - id: handoff\n    label: \"Hand off\"\n    delivers: true\n    advance_gate: auto\n";
    let def = config::WorkflowDef::parse(std::path::Path::new("rail.yml"), text, &config::Roster::offering_nothing())
        .unwrap_or_else(|refused| panic!("the fixture did not parse: {refused}"));
    config::ResolvedWorkflow::resolve(&def, &super::daemon::manifest())
        .unwrap_or_else(|refused| panic!("the fixture did not resolve: {refused}"))
}

const STALLED: &str = "Fleet does not move it for you";

/// **A Job that stops on a step only its own act ends is told so, once.** Read from the row: the turn
/// ended on a reply that asks nothing and nothing waits on the person. The Logbook Job, 27 Oct 2026.
#[tokio::test]
async fn a_job_that_reports_and_stops_on_a_step_only_it_can_end_is_told_once() {
    let rig = ruled_on(Arc::new(FakeJudge::with_no_objection()), discussed());
    let id = on_the_rail(&rig).await;
    eventually(|| async { told(&rig, "Settle the scope.") == 1 }).await;

    rig.stand_in.finishes(0, "Settled so far: one line per entry.\nOnce Fleet moves to the plan step, I will record it.");
    eventually(|| async { told(&rig, STALLED) == 1 }).await;
    let said = rig.stand_in.process(0).sent.lock().unwrap().iter().find(|line| line.contains(STALLED)).cloned().unwrap();
    for part in ["Discuss", "you record a plan with record_plan", "ask the person what is still open"] {
        assert!(said.contains(part), "{part}: {said}");
    }

    rig.stand_in.finishes(0, "Still settled.");
    eventually(|| async { record(&rig, &id).await.standing == Some(ipc::SessionStanding::Replied) }).await;
    rig.fleet.timeline().settled().await;
    assert_eq!(told(&rig, STALLED), 1, "once for each step");
}

/// A turn that ends on a question waits on the person, so the Job has not stopped on its own step.
#[tokio::test]
async fn a_job_that_ends_its_turn_on_a_question_is_not_told_it_stopped() {
    let rig = ruled_on(Arc::new(FakeJudge::with_no_objection()), discussed());
    let id = on_the_rail(&rig).await;
    eventually(|| async { told(&rig, "Settle the scope.") == 1 }).await;

    rig.stand_in.finishes(0, "Settled so far: one line per entry.\n\nShould I record the plan now?");
    eventually(|| async { record(&rig, &id).await.waiting_for.iter().any(|one| one.source == WaitingSource::Replied) }).await;
    rig.fleet.timeline().settled().await;
    assert_eq!(told(&rig, STALLED), 0, "the person is asked, so the Job waits on him");
}

const UNDELIVERED: &str = "with no pull request opened";

/// **A Job that pushed and stops with no pull request open is told to deliver, once.** The merge that
/// ends the delivering step is nobody's act until the Job opens one. 28 Oct 2026: a Job pushed with Auto
/// land chosen, wrote "next is opening the PR" and sat idle until the owner asked.
#[tokio::test]
async fn a_job_that_pushed_and_stops_with_no_pull_request_is_told_to_deliver_once() {
    let rig = ruled(Arc::new(FakeJudge::with_no_objection()));
    let id = on_the_rail(&rig).await;
    approved_plan(&rig, &id).await;
    rig.stand_in.says(0, vec![edit("crates/reader/src/lib.rs")]);
    let harness = record(&rig, &id).await.harness;
    rig.fleet
        .report_session(SessionReport { harness, session_id: id.clone(), fact: SessionFact::Pushed { branch: Some("armada/reader".into()) } })
        .await
        .expect("kept");
    eventually(|| async { record(&rig, &id).await.workflow_step == Some(2) }).await;

    rig.stand_in.finishes(0, "Pushed the branch.\nNext is the Hand off step: opening the PR.");
    eventually(|| async { told(&rig, UNDELIVERED) == 1 }).await;
    let said = rig.stand_in.process(0).sent.lock().unwrap().iter().find(|line| line.contains(UNDELIVERED)).cloned().unwrap();
    for part in ["Hand off", "Deliver now", "open the pull request", "Keep local, call finish_up and then all_done"] {
        assert!(said.contains(part), "{part}: {said}");
    }

    rig.stand_in.finishes(0, "Still pushed.");
    eventually(|| async { record(&rig, &id).await.standing == Some(ipc::SessionStanding::Replied) }).await;
    rig.fleet.timeline().settled().await;
    assert_eq!(told(&rig, UNDELIVERED), 1, "once for each step");

    // With its pull request open, the Job waits on the merge Fleet tells it of.
    let mut opened = record(&rig, &id).await;
    let build = opened.build.as_mut().expect("a build record");
    build.stops.push(ipc::BuildStop { id: ipc::BuildStopId::Pr, at: Some(ipc::Instant::carried(String::from("2026-10-28T12:00:00.000Z"))) });
    let steps = rail::steps_of(&workflow());
    assert!(rail::stalled(&steps, &opened, rail::Extra::default()).is_none(), "a pull request open is a wait, not a stall");
}

/// **The step before a delivering one says to carry straight on**, so a Job that pushed does not stop
/// to wait for Fleet's word on the next step.
#[test]
fn the_step_before_delivering_says_to_carry_straight_on_with_it() {
    let refine = RailStep { instructions: Some("Wire it and write the tests.".into()), ..follows("Refine") };
    let steps = [plan_step(), refine, delivers("Deliver")];
    let said = rail::instructed("feature", &steps, 1).expect("instructions are sent");
    assert!(said.contains("This step ends when you push the branch."), "{said}");
    assert!(said.contains("carry straight on with its \"Deliver\" step in the same turn"), "{said}");
    let discuss = RailStep { instructions: Some("Settle the scope.".into()), ..step("Discuss") };
    let said = rail::instructed("feature", &[discuss, plan_step(), delivers("Deliver")], 0).expect("instructions are sent");
    assert!(!said.contains("carry straight on"), "only the step whose end enters delivering: {said}");
}

/// **Only a step the Job's own act ends can stall it.** One the person ends (a reply, an approval) or
/// a merge ends is his to move, and Fleet tells the Job nothing.
#[test]
fn a_step_stalls_only_where_the_jobs_own_act_ends_it() {
    let propose = RailStep { ends_on: vec![StepEnd::Replied], ..step("Propose") };
    let steps = [propose, step("Discuss"), plan_step(), follows("Refine"), delivers("Deliver")];
    let ends = (0..steps.len()).map(|at| rail_ends::end_of(&steps, at).map(|end| end.by_the_job())).collect::<Vec<_>>();
    assert_eq!(ends, [Some(false), Some(true), Some(false), Some(true), Some(false)], "reply, record, approve, push, merge");
}

#[tokio::test]
async fn a_step_skipped_draws_one_nudge_naming_it_and_the_agents_edits_are_not_blocked() {
    let rig = ruled(Arc::new(FakeJudge::with_no_objection()));
    let id = on_the_rail(&rig).await;

    rig.stand_in.says(0, vec![edit("crates/reader/src/lib.rs")]);
    eventually(|| async { told(&rig, "edited files before") == 1 }).await;
    let said = rig.stand_in.process(0).sent.lock().unwrap().iter().find(|line| line.contains("edited files before")).cloned().unwrap();
    for part in ["Plan the change", "Nothing is blocked"] {
        assert!(said.contains(part), "{part}: {said}");
    }

    // A second edit, and a second look at the row: still the one message.
    rig.stand_in.says(0, vec![edit("crates/reader/src/more.rs")]);
    approved_plan(&rig, &id).await;
    eventually(|| async { record(&rig, &id).await.workflow_step == Some(1) }).await;
    rig.fleet.timeline().settled().await;
    assert_eq!(told(&rig, "edited files before"), 1, "once for each step");
    let edit = rig.gate(&id, "Edit", r#"{"file_path":"crates/reader/a.rs"}"#).await;
    assert!(
        !matches!(&edit, ipc::GateAnswer::Hold { held } if held.reason.contains("plan") || held.reason.contains("step")),
        "an edit before the plan is not held for it: {edit:?}"
    );
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
async fn with_workflows_off_there_is_no_step_no_nudge_and_no_boundary_judge() {
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
    assert_eq!(told(&rig, "edited files before"), 0, "no nudge");
    assert!(asked_of(&judge).is_empty(), "no boundary judge: {:?}", asked_of(&judge));
    assert_eq!(says_of_steps(&rig).await, entered, "and no step entered on the timeline after the switch");
}
