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
        previews: false,
        ends_on: Vec::new(),
        gate: core_model::StepGate::None,
        criteria: Vec::new(),
        fit: None,
        tabs: Vec::new(),
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

/// A workflow this repository ships, as the rail reads it.
fn shipped(file: &str, text: &str) -> Vec<RailStep> {
    let def = config::WorkflowDef::parse(std::path::Path::new(file), text, &config::Roster::offering_nothing())
        .unwrap_or_else(|refused| panic!("{file} did not parse: {refused}"));
    let resolved = config::ResolvedWorkflow::resolve(&def, &super::daemon::manifest())
        .unwrap_or_else(|refused| panic!("{file} did not resolve: {refused}"));
    rail::steps_of(&resolved)
}

/// **A step placed by its `gate` alone moves the Job on.** The workflows shipped on 9 Nov 2026 state
/// every step by its gate, with no `records_plan` or `walks`, and the rail read neither: 33 Jobs whose
/// plan was approved and whose pull request merged stood on their first step throughout.
#[test]
fn feature_and_bug_leave_their_first_step_when_the_plan_is_approved() {
    for (file, text) in [
        ("feature.json", include_str!("../../../../.armada/workflows/feature.json")),
        ("bug.json", include_str!("../../../../.armada/workflows/bug.json")),
    ] {
        let steps = shipped(file, text);
        assert_eq!(steps.len(), 3, "{file}: propose, try, deliver");
        let recorded = Facts { plan_recorded: true, ..Facts::default() };
        assert_eq!(at(&steps, recorded), Some(0), "{file}: a plan waiting on him keeps the first step");
        let seen = Facts { plan_expects_walk: true, tasks_open: true, edited: true, ..approved() };
        assert_eq!(at(&steps, seen), Some(1), "{file}: approved, with a part he would see, is the try");
        let tried = Facts { offered: true, you_approved: crate::rail_lifecycle::StepBits::of([1]), pushed: true, ..seen };
        assert_eq!(at(&steps, tried), Some(2), "{file}: his Looks good is the delivering step");
        let unseen = Facts { tasks_open: true, edited: true, ..approved() };
        assert_eq!(at(&steps, unseen), Some(1), "{file}: nothing is passed over before the Job has pushed");
        let unseen = Facts { pushed: true, ..unseen };
        assert_eq!(at(&steps, unseen), Some(2), "{file}: with nothing he would see, the try is passed over once pushed");
        assert_eq!(at(&steps, Facts { landed: true, ..unseen }), None, "{file}: merged, and nothing follows");
    }
}

// ---- A step that previews ----

/// A previewing step as `steps_of` reads one: gated `you_approved`, so his Approve is what it lists as its end.
fn previewing(label: &str) -> RailStep {
    RailStep { previews: true, gate: core_model::StepGate::YouApproved, ends_on: vec![StepEnd::YouApproved], ..step(label) }
}

fn looked_good(at: usize) -> crate::rail_lifecycle::StepBits {
    crate::rail_lifecycle::StepBits::of([at])
}

/// **Entered when the plan is approved and the step before it is passed, or when the Job offers a branch.**
#[test]
fn a_previewing_step_is_entered_when_the_plan_is_approved_or_when_the_job_offers_a_branch() {
    let steps = [recording("Propose"), previewing("Try"), delivers("Deliver")];
    let seen = Facts { plan_expects_walk: true, ..approved() };
    assert_eq!(at(&steps, Facts { plan_recorded: true, ..Facts::default() }), Some(0), "a plan waiting on him keeps Propose");
    assert_eq!(at(&steps, seen), Some(1), "approved: Try, with nothing offered yet");
    let early = Facts { plan_recorded: true, offered: true, ..Facts::default() };
    assert_eq!(at(&steps, early), Some(1), "a branch offered enters it before the plan is approved");
    assert_eq!(rail_ends::ends_of(&steps, 1), vec![rail_ends::End::LooksGood]);
    assert_eq!(rail_ends::ends_when(&steps, 1).as_deref(), Some("you say Looks good on the branch offered"));
}

/// **A push is how a preview is made**, so it never passes the step, and the work-ahead rule skips it.
#[test]
fn a_push_never_passes_a_previewing_step_and_work_ahead_flags_nothing_on_it() {
    let steps = [recording("Propose"), previewing("Try"), delivers("Deliver")];
    let offered = Facts { offered: true, plan_expects_walk: true, ..approved() };
    for ahead in [Facts { pushed: true, ..offered }, Facts { pushed: true, pr_opened: true, ..offered }] {
        assert_eq!(at(&steps, ahead), Some(1), "{ahead:?}");
        assert!(rail::passed_unapproved(&steps, &ahead, Some(1)).is_empty(), "nothing is passed: {ahead:?}");
    }
    let unapproved = Facts { plan_recorded: true, pushed: true, offered: true, ..Facts::default() };
    assert_eq!(at(&steps, unapproved), Some(1), "work ahead of an unapproved plan still stops at the offer");
    assert_eq!(rail::passed_unapproved(&steps, &unapproved, Some(1)), vec![0], "though the plan he never approved is flagged");
}

/// **His Looks good is the `you_approved` fact and the only thing that ends it**; the delivering step is then entered, push or not.
#[test]
fn his_looks_good_ends_a_previewing_step_and_the_delivering_step_is_entered() {
    let steps = [recording("Propose"), previewing("Try"), delivers("Deliver")];
    let offered = Facts { offered: true, pushed: true, plan_expects_walk: true, ..approved() };
    assert_eq!(at(&steps, offered), Some(1));
    assert_eq!(at(&steps, Facts { you_approved: looked_good(1), ..offered }), Some(2), "Looks good: Deliver");
    assert_eq!(at(&steps, Facts { you_approved: looked_good(1), pushed: false, ..offered }), Some(2), "before any push too");
    assert_eq!(at(&steps, Facts { replied: true, edited: true, committed: true, checked: true, ..offered }), Some(1), "words, edits and checks do not end it");
    assert_eq!(at(&steps, Facts { you_approved: looked_good(1), landed: true, ..offered }), None, "merged, and nothing follows");
}

/// **A withdrawn offer, or one the branch moved past, ends nothing**: Fleet keeps a note for each offer, so the fact
/// that one was made outlives its withdrawal and the step stays held until he says Looks good.
#[test]
fn a_withdrawn_or_moved_offer_leaves_a_previewing_step_held() {
    let steps = [recording("Propose"), previewing("Try"), delivers("Deliver")];
    let held = Facts { offered: true, pushed: true, ..approved() };
    assert_eq!(at(&steps, held), Some(1), "offered, then withdrawn: the note is kept");
    assert_eq!(at(&steps, Facts { pushed: true, plan_expects_walk: true, ..approved() }), Some(1), "no offer left, and the plan said he would see it");
    assert!(rail::passed_unapproved(&steps, &held, Some(1)).is_empty());
}

/// A plan approved saying he would see nothing, and no branch offered: the step is passed over once the Job pushes, as a walk is.
#[test]
fn a_previewing_step_is_passed_over_when_the_plan_says_nothing_would_be_seen_and_nothing_was_offered() {
    let steps = [recording("Propose"), previewing("Try"), delivers("Deliver")];
    assert_eq!(at(&steps, approved()), Some(1), "approved, nothing pushed: Try");
    assert_eq!(at(&steps, Facts { pushed: true, ..approved() }), Some(2), "pushed with nothing to see: Deliver");
    assert_eq!(at(&steps, Facts { pushed: true, offered: true, ..approved() }), Some(1), "an offer holds it");
}

/// The four workflows this repository ships try a branch where they walked: `previews`, his Approve as its end, no walk.
#[test]
fn every_shipped_walk_step_is_a_previewing_step_that_ends_on_his_looks_good() {
    for (file, text, id) in [
        ("feature.json", include_str!("../../../../.armada/workflows/feature.json"), "try"),
        ("bug.json", include_str!("../../../../.armada/workflows/bug.json"), "fix"),
        ("annotations.json", include_str!("../../../../.armada/workflows/annotations.json"), "try"),
        ("prototype.json", include_str!("../../../../.armada/workflows/prototype.json"), "build"),
    ] {
        let steps = shipped(file, text);
        assert!(steps.iter().all(|step| !step.walks), "{file}: no step is a walk");
        let tries: Vec<&RailStep> = steps.iter().filter(|step| step.previews).collect();
        let [one] = tries.as_slice() else { panic!("{file}: one previewing step, got {}", tries.len()) };
        assert_eq!(one.id, id, "{file}");
        assert_eq!(one.gate, core_model::StepGate::YouApproved, "{file}");
        assert!(one.ends_on.contains(&StepEnd::YouApproved), "{file}: his Approve ends it");
        let said = one.instructions.as_deref().unwrap_or_default();
        assert!(said.contains("offer_preview") && said.contains("Looks good") && !said.contains("show_window"), "{file}: {said}");
        assert_eq!(rail_ends::ends_of(&steps, steps.iter().position(|step| step.previews).unwrap()), vec![rail_ends::End::LooksGood], "{file}");
    }
}

// ---- Through a Fleet ----

pub(super) const REQUEST: &str = "the log reader drops the last line of every file";

pub(super) fn workflow() -> config::ResolvedWorkflow {
    let text = "version: 1\nworkflow_id: rail\nname: Rail\nsteps:\n  \
        - id: plan\n    label: \"Plan the change\"\n    instructions: \"Record the plan before editing.\"\n    tabs: [plan, qa, now]\n    evidence: {submitted: {type: plan}}\n    \
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
        sketches: Vec::new(),
        summary: "You get a plan.".into(),
        groups: vec![PlanGroupInput {
            sketches: Vec::new(),
            id: None,
            title: "Reader".into(),
            agent: "rust-engineer".into(),
            paths: vec!["crates/reader".into()],
            issue: None,
            gives: None,
            lands_in: None,
            own_job: false,
            gate: Default::default(),
            tasks: vec![PlanTaskInput { id: None, title: "Stop at the end".into() }],
        }],
        sketch: None,
        walk: None,
        ask_again: false,
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

// ---- A step held by its gate ----

/// A step with no anchor, gated on `gate`: how `oversee`, `summarise` and `implement` read in the shipped workflows.
fn gated(label: &str, gate: core_model::StepGate) -> RailStep {
    RailStep { gate, ..step(label) }
}

/// The first step, as `steps_of` reads a plan step once PR 3955 lands; here it says so itself.
fn recording(label: &str) -> RailStep {
    RailStep { records_plan: true, gate: core_model::StepGate::PlanApproved, ..step(label) }
}

/// **A step gated `jobs_ended` or `document_approved` with no anchor holds the Job until its fact.** The
/// orchestrate workflow stood on Plan from the plan's approval to the end (20 Nov 2026).
#[test]
fn an_orchestrate_shaped_job_moves_through_oversee_and_summarise_on_the_facts_that_gate_them() {
    use core_model::StepGate::{DocumentApproved, JobsEnded};
    let steps = [recording("Plan"), gated("Oversee", JobsEnded), gated("Summarise", DocumentApproved)];
    assert_eq!(at(&steps, Facts { plan_recorded: true, ..Facts::default() }), Some(0), "a plan waiting on him keeps Plan");
    assert_eq!(at(&steps, approved()), Some(1), "approved: oversee holds the Job while its Jobs run");
    assert_eq!(at(&steps, Facts { document_approved: true, ..approved() }), Some(1), "a page approved early does not pass oversee");
    assert_eq!(at(&steps, Facts { jobs_ended: true, ..approved() }), Some(2), "every Job it started has ended: summarise");
    // The last step's end has come and nothing follows it: the rail is absent, as for a last step that waits on a lifecycle.
    let done = Facts { jobs_ended: true, document_approved: true, ..approved() };
    assert_eq!(at(&steps, done), None, "the summary approved: nothing follows");
}

/// **The step after a held one is entered at its end, push or not**: refactor's handoff delivers, and the Job
/// has pushed nothing when he approves the page that ends Implement.
#[test]
fn a_refactor_shaped_job_enters_its_delivering_step_when_the_page_is_approved_before_any_push() {
    use core_model::StepGate::DocumentApproved;
    let steps = [recording("Plan"), gated("Implement", DocumentApproved), delivers("Handoff")];
    assert_eq!(at(&steps, approved()), Some(1), "approved, nothing pushed: implement");
    assert_eq!(at(&steps, Facts { pushed: true, ..approved() }), Some(2), "a push is work for the step after, so the Job moves on");
    assert_eq!(rail::passed_unapproved(&steps, &Facts { pushed: true, ..approved() }, Some(2)), vec![1], "and the page it never approved is flagged");
    assert_eq!(at(&steps, Facts { document_approved: true, ..approved() }), Some(2), "the page approved: handoff, before any push");
    assert_eq!(at(&steps, Facts { document_approved: true, pushed: true, landed: true, ..approved() }), None, "merged, and nothing follows");
}

/// A `sketch_approved` and a `checks_green` gate hold the same way, and each reads its own fact.
#[test]
fn a_step_gated_on_a_sketch_or_green_checks_holds_until_that_fact_alone() {
    use core_model::StepGate::{ChecksGreen, SketchApproved};
    let steps = [recording("Plan"), gated("Sketch", SketchApproved), gated("Verify", ChecksGreen), fitted("Close", Fit::Committed)];
    assert_eq!(at(&steps, approved()), Some(1));
    assert_eq!(at(&steps, Facts { checks_green: true, ..approved() }), Some(1), "green checks do not pass the sketch");
    let sketched = Facts { sketch_approved: true, ..approved() };
    assert_eq!(at(&steps, sketched), Some(2), "the sketch approved: verify holds");
    assert_eq!(at(&steps, Facts { pushed: true, committed: true, ..sketched }), Some(2), "a push and a commit do not pass it");
    assert_eq!(at(&steps, Facts { checks_green: true, ..sketched }), Some(3), "Fleet's Checks passed: the step after");
}

// ---- Work ahead of a step ----

/// A step gated `you_approved`, as `steps_of` reads it: his Approve is also what it lists as its end.
fn pressed(label: &str) -> RailStep {
    RailStep { gate: core_model::StepGate::YouApproved, ends_on: vec![StepEnd::YouApproved], ..step(label) }
}

/// **A push or a pull request seen while a step waits on his Approve moves the Job on and flags the step.**
/// Propose stood from the plan to the merge on 41 Jobs that held a pull request (20 Nov 2026).
#[test]
fn work_ahead_of_an_unapproved_plan_moves_the_step_on_and_flags_the_passed_one() {
    let steps = [recording("Propose"), RailStep { walks: true, gate: core_model::StepGate::WalkApproved, ..step("Walk") }, delivers("Deliver")];
    let recorded = Facts { plan_recorded: true, ..Facts::default() };
    assert_eq!(at(&steps, recorded), Some(0), "nothing ahead: the plan waits on him");
    for ahead in [Facts { pushed: true, ..recorded }, Facts { pushed: true, pr_opened: true, ..recorded }, Facts { landed: true, pushed: true, ..recorded }] {
        let now = at(&steps, ahead);
        assert_eq!(now, if ahead.landed { None } else { Some(2) }, "{ahead:?}");
        assert_eq!(rail::passed_unapproved(&steps, &ahead, now), vec![0], "Propose is passed without his Approve: {ahead:?}");
    }
}

#[test]
fn work_ahead_still_stops_at_a_walk_the_plan_expects() {
    let steps = [recording("Propose"), RailStep { walks: true, gate: core_model::StepGate::WalkApproved, ..step("Walk") }, delivers("Deliver")];
    let ahead = Facts { plan_recorded: true, pushed: true, plan_expects_walk: true, ..Facts::default() };
    assert_eq!(at(&steps, ahead), Some(1), "a push never passes a walk: the Job stands on it");
    assert_eq!(rail::passed_unapproved(&steps, &ahead, Some(1)), vec![0], "though the plan it never approved is flagged");
}

#[test]
fn an_approved_step_is_not_flagged_and_work_with_nothing_to_deliver_to_flags_nothing() {
    let steps = [recording("Propose"), follows("Build"), delivers("Deliver")];
    let pushed = Facts { pushed: true, ..approved() };
    assert_eq!(at(&steps, pushed), Some(2));
    assert!(rail::passed_unapproved(&steps, &pushed, Some(2)).is_empty(), "he approved the plan");
    let no_delivery = [recording("Frame"), fitted("Build", Fit::Edited), step("Write up")];
    let ahead = Facts { plan_recorded: true, pushed: true, ..Facts::default() };
    assert_eq!(at(&no_delivery, ahead), Some(0), "no later step delivers, so a push belongs to none");
    assert!(rail::passed_unapproved(&no_delivery, &ahead, Some(0)).is_empty());
}

#[test]
fn a_step_gated_on_his_approve_of_an_item_is_flagged_when_work_passes_it_and_not_when_he_pressed() {
    let steps = [recording("Plan"), pressed("Sign off"), delivers("Deliver")];
    let ahead = Facts { pushed: true, ..approved() };
    assert_eq!(at(&steps, ahead), Some(2));
    assert_eq!(rail::passed_unapproved(&steps, &ahead, Some(2)), vec![1]);
    let pressed = Facts { you_approved: crate::rail_lifecycle::StepBits::of([1]), ..ahead };
    assert!(rail::passed_unapproved(&steps, &pressed, Some(2)).is_empty(), "his press is the fact");
}

/// **Only his press ends a step that waits on it**: the agent saying it is done is no fact the rail has.
#[test]
fn the_agents_word_does_not_end_a_step_that_waits_on_his_press_and_his_press_does() {
    let steps = [recording("Plan"), pressed("Sign off"), step("Close out")];
    let said = Facts { replied: true, edited: true, committed: true, checked: true, ..approved() };
    assert_eq!(at(&steps, said), Some(1), "words, edits and checks do not end it");
    let pressed = Facts { you_approved: crate::rail_lifecycle::StepBits::of([1]), ..said };
    assert_eq!(at(&steps, pressed), Some(2), "his Approve does");
}

// ---- The last step that lists nothing ----

#[test]
fn a_last_step_that_lists_nothing_is_entered_once_the_steps_before_it_pass_and_ends_when_the_job_does() {
    let steps = [recording("Plan"), follows("Do"), step("Close out")];
    let working = Facts { tasks_open: true, ..approved() };
    assert_eq!(at(&steps, working), Some(1), "tasks open: the step before it");
    assert_eq!(at(&steps, approved()), Some(2), "every step before it passed: the Job stands on it");
    assert_eq!(rail_ends::ends_of(&steps, 2), vec![rail_ends::End::JobEnds]);
    assert_eq!(rail_ends::ends_when(&steps, 2).as_deref(), Some("the Job ends"));
}

#[test]
fn a_middle_step_nothing_ends_is_warned_of_and_one_a_later_step_enters_or_a_gate_ends_is_not() {
    let nothing = [recording("Plan"), step("Free"), step("Close out")];
    assert!(crate::workflow_page::warning_of(&nothing, 1).is_some_and(|said| said.contains("passes through it at once")), "a middle step with nothing to end it");
    assert!(crate::workflow_page::warning_of(&nothing, 2).is_none(), "the last ends with the Job");
    assert!(crate::workflow_page::warning_of(&nothing, 0).is_none(), "the plan's approval ends the first");
    let entered = [recording("Plan"), step("Free"), fitted("Build", Fit::Edited)];
    assert!(crate::workflow_page::warning_of(&entered, 1).is_none(), "the step after it is entered on a fact Fleet sees");
    let gated_step = [recording("Plan"), pressed("Sign off"), step("Close out")];
    assert!(crate::workflow_page::warning_of(&gated_step, 1).is_none(), "his press ends it");
}
