//! The claim, written before the code: **Fleet reads the step a Session is on from what it observed, nudges
//! the agent once about a step it skipped, runs a judge at the boundary of a step that asks the Judge
//! something, and lets the person send a judge's concern on or drop it. Nothing blocks.**
//! `docs/concepts/session.md`, *The rail* and *The judges*.
//!
//! The first half is the rules, pure. The second drives a Fleet with a workflow and the judge test double.

use std::sync::Arc;

use adapter_traits::{CallDetail, DroneEvent};
use api::{HostedSessions, Refusal, Sessions};
use core_model::{CriterionId, JudgeCriterion, OnRefusal};
use ipc::{
    ApprovePlan, ConcernFrom, ConcernHow, DismissConcern, JudgeSession, PlanGroupInput, PlanTaskInput, RecordPlan,
    SendConcern, SessionFact, SessionId, SessionRecord, SessionReport, SetSessionWorkflow, WaitingSource,
};
use testkit::FakeJudge;

use super::session_host::{eventually, rig_fitted, Rig};
use crate::rail::{self, Facts, Fit, RailStep, Skip};
use crate::session_host::session_judging::place_in_diff;

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
        awaits_reply: false,
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
fn a_boundary_judge_is_for_the_step_just_left_if_it_asks_and_was_not_read() {
    let steps = [plan_step(), follows("Implement"), delivers("Hand off")];
    assert_eq!(rail::left_behind(&steps, Some(0), Some(1), false, &[]), None, "the plan step asks the Judge nothing");
    assert_eq!(rail::left_behind(&steps, Some(1), Some(2), false, &[]), Some(1));
    assert_eq!(rail::left_behind(&steps, Some(1), Some(2), false, &[1]), None, "once for each step");
    assert_eq!(rail::left_behind(&steps, Some(2), Some(1), false, &[]), None, "moving back leaves nothing behind");
    assert_eq!(rail::left_behind(&steps, Some(1), Some(1), false, &[]), None);
    let last = [plan_step(), RailStep { delivers: true, criteria: vec![criterion()], ..step("Hand off") }];
    assert_eq!(rail::left_behind(&last, Some(1), None, true, &[]), Some(1), "merging leaves the last step");
    assert_eq!(rail::left_behind(&last, Some(1), None, false, &[]), None);
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

#[test]
fn a_cited_line_of_the_diff_part_is_placed_in_its_file_as_the_change_leaves_it() {
    let brief = "Work: Implement\n\
                 The work so far:\n\
                 diff --git a/src/a.rs b/src/a.rs\n\
                 --- a/src/a.rs\n\
                 +++ b/src/a.rs\n\
                 @@ -10,3 +20,4 @@\n\
                 \x20context\n\
                 -removed\n\
                 +added one\n\
                 +added two\n\
                 diff --git a/src/b.rs b/src/b.rs\n\
                 --- a/src/b.rs\n\
                 +++ b/src/b.rs\n\
                 @@ -1 +5 @@\n\
                 +other\n";
    let place = |line: u32| place_in_diff(brief, line);
    assert_eq!(place(7), Some(("src/a.rs".into(), 20)), "a context line");
    assert_eq!(place(9), Some(("src/a.rs".into(), 21)), "the first added line, the removed one not counted");
    assert_eq!(place(10), Some(("src/a.rs".into(), 22)));
    assert_eq!(place(15), Some(("src/b.rs".into(), 5)), "a new file's header resets the count");
    assert_eq!(place(1), None, "a line above any file");
    assert_eq!(place(5), None, "a header is not a place");
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

#[test]
fn entering_a_step_sends_its_instructions_and_a_step_with_none_sends_nothing() {
    let told = RailStep { instructions: Some("  Record the plan first.\n".into()), ..plan_step() };
    let said = rail::instructed("Bug", &told).expect("instructions are sent");
    for part in ["Bug workflow", "\"Plan the change\"", "Record the plan first."] {
        assert!(said.contains(part), "{part}: {said}");
    }
    assert_eq!(rail::instructed("Bug", &plan_step()), None, "no instructions, no message");
    let blank = RailStep { instructions: Some("  ".into()), ..plan_step() };
    assert_eq!(rail::instructed("Bug", &blank), None, "blank is none");
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

#[tokio::test]
async fn a_step_that_asks_the_judge_nothing_is_not_read_when_the_session_leaves_it() {
    let judge = Arc::new(FakeJudge::with_no_objection());
    let rig = ruled(Arc::clone(&judge));
    let id = on_the_rail(&rig).await;
    approved_plan(&rig, &id).await;
    eventually(|| async { says_of_steps(&rig).await.iter().any(|one| one.contains("→ Implement")) }).await;
    rig.fleet.timeline().settled().await;
    assert!(asked_of(&judge).is_empty(), "the plan step has no judge_checks: {:?}", asked_of(&judge));
    assert!(record(&rig, &id).await.judge.is_none());
}

/// Boundary judges are off (16 Oct 2026, the owner's decision): leaving a step that asks the Judge
/// reads nothing, and only the tile does.
#[tokio::test]
async fn leaving_a_step_that_asks_the_judge_reads_nothing_while_boundary_judges_are_off() {
    let judge = Arc::new(FakeJudge::with_no_objection());
    let rig = ruled(Arc::clone(&judge));
    let id = on_the_rail(&rig).await;
    approved_plan(&rig, &id).await;
    rig.stand_in.says(0, vec![edit("crates/reader/src/lib.rs")]);
    let harness = record(&rig, &id).await.harness;
    rig.fleet
        .report_session(SessionReport { harness, session_id: id.clone(), fact: SessionFact::Pushed { branch: Some("armada/reader".into()) } })
        .await
        .expect("kept");

    eventually(|| async { record(&rig, &id).await.workflow_step == Some(2) }).await;
    rig.fleet.timeline().settled().await;
    assert!(asked_of(&judge).is_empty(), "no judge reads a step left behind: {:?}", asked_of(&judge));
    let read = record(&rig, &id).await;
    assert!(read.judge.is_none() && !read.judging);
}

#[tokio::test]
async fn the_tile_reads_now_and_never_sees_the_agents_account_of_its_own_work() {
    let judge = Arc::new(FakeJudge::with_no_objection());
    let rig = ruled(Arc::clone(&judge));
    let id = on_the_rail(&rig).await;
    approved_plan(&rig, &id).await;
    rig.stand_in.finishes(0, "AGENT ACCOUNT: everything is fine");

    let reply = Arc::clone(&rig.fleet).judge_session(JudgeSession { session_id: id.clone() }).await.expect("asked");
    assert!(reply.judging, "answered at once, reading");
    eventually(|| async { record(&rig, &id).await.judge.is_some() }).await;

    let read = record(&rig, &id).await;
    assert!(!read.judging);
    let judged = read.judge.expect("a reading");
    assert_eq!(judged.concerns, 0, "zero is a reading, shown as no concerns");
    assert!(read.waiting_for.iter().all(|one| one.source != WaitingSource::Judge));
    let asked = asked_of(&judge);
    assert_eq!(asked.len(), 2, "the plan question and the step's own");
    for brief in &asked {
        assert!(brief.contains(REQUEST) && brief.contains("Stop at the end"), "{brief}");
        assert!(!brief.contains("AGENT ACCOUNT"), "the transcript is not shown: {brief}");
    }
}

fn two_concerns() -> Arc<FakeJudge> {
    Arc::new(FakeJudge::answering(&[
        (
            "carry out the plan",
            "verdict: not_met\nexpected: every task of the plan carried out\nproduced: nothing under crates/reader changed\nconsequence: the reader still drops a line",
        ),
        (
            "stop at the end",
            "verdict: not_met\nexpected: the loop to end at the last line\nproduced: the bound is one too many\nconsequence: a blank row is read",
        ),
    ]))
}

async fn raised(rig: &Rig, id: &SessionId) -> SessionRecord {
    let reply = Arc::clone(&rig.fleet).judge_session(JudgeSession { session_id: id.clone() }).await.expect("asked");
    assert!(reply.judging);
    eventually(|| async { record(rig, id).await.judge.is_some() }).await;
    record(rig, id).await
}

#[tokio::test]
async fn each_concern_is_a_waiting_item_that_says_what_produced_it() {
    let rig = ruled(two_concerns());
    let id = on_the_rail(&rig).await;
    approved_plan(&rig, &id).await;
    let read = raised(&rig, &id).await;

    assert_eq!(read.judge.map(|one| one.concerns), Some(2));
    let items: Vec<_> = read.waiting_for.iter().filter(|one| one.source == WaitingSource::Judge).collect();
    assert_eq!(items.iter().map(|one| one.id.as_str()).collect::<Vec<_>>(), ["judge:1", "judge:2"]);
    assert!(items[0].text.contains("nothing under crates/reader changed") && items[1].text.contains("one too many"));
    assert!(items.iter().all(|one| one.concern.as_ref().is_some_and(|c| c.from == ConcernFrom::Tile && c.settled.is_none())));
    assert!(items.iter().all(|one| one.act.as_ref().is_some_and(|act| act.target == id.as_str())));

    // Read again: the same words are not raised twice, though the reading counts them.
    let again = raised_again(&rig, &id).await;
    assert_eq!(again.waiting_for.iter().filter(|one| one.source == WaitingSource::Judge).count(), 2);
}

async fn raised_again(rig: &Rig, id: &SessionId) -> SessionRecord {
    let before = record(rig, id).await.judge.map(|one| one.at);
    Arc::clone(&rig.fleet).judge_session(JudgeSession { session_id: id.clone() }).await.expect("asked");
    eventually(|| async { let now = record(rig, id).await; !now.judging && now.judge.map(|one| one.at) != before }).await;
    record(rig, id).await
}

#[tokio::test]
async fn sending_a_concern_tells_the_agent_as_his_message_and_dismissing_one_tells_it_nothing() {
    let rig = ruled(two_concerns());
    let id = on_the_rail(&rig).await;
    approved_plan(&rig, &id).await;
    raised(&rig, &id).await;

    let sent = Arc::clone(&rig.fleet).send_concern(SendConcern { session_id: id.clone(), item_id: "judge:1".into() }).await.expect("sent");
    assert!(sent.waiting_for.iter().all(|one| one.id != "judge:1"), "it no longer waits");
    let [settled] = &sent.settled[..] else { panic!("one settled: {:?}", sent.settled) };
    assert_eq!(settled.id, "judge:1");
    assert_eq!(settled.concern.as_ref().and_then(|c| c.settled.as_ref()).map(|s| s.how), Some(ConcernHow::Sent));
    eventually(|| async { told(&rig, "raised this concern") == 1 }).await;
    let said = rig.stand_in.process(0).sent.lock().unwrap().iter().find(|line| line.contains("raised this concern")).cloned().unwrap();
    assert!(said.contains("nothing under crates/reader changed") && said.contains("Judge tile"), "{said}");

    let dismissed = Arc::clone(&rig.fleet)
        .dismiss_concern(DismissConcern { session_id: id.clone(), item_id: "judge:2".into() })
        .await
        .expect("dismissed");
    assert!(dismissed.waiting_for.iter().all(|one| one.source != WaitingSource::Judge));
    assert_eq!(
        dismissed.settled.iter().map(|one| (one.id.as_str(), one.concern.as_ref().and_then(|c| c.settled.as_ref()).map(|s| s.how))).collect::<Vec<_>>(),
        [("judge:1", Some(ConcernHow::Sent)), ("judge:2", Some(ConcernHow::Dismissed))],
        "both stay on the record, for the thread to draw as a line"
    );
    rig.fleet.timeline().settled().await;
    assert_eq!(told(&rig, "raised this concern"), 1, "dismissing told the agent nothing");

    for item_id in ["judge:1", "judge:2", "judge:9", "nonsense"] {
        let again = Arc::clone(&rig.fleet).send_concern(SendConcern { session_id: id.clone(), item_id: item_id.into() }).await;
        assert!(matches!(&again, Err(Refusal::IllegalMove(why)) if why.code == "fleet.session_waiting_unheld"), "{item_id}: {again:?}");
    }
}

#[tokio::test]
async fn a_call_that_fails_raises_nothing_and_leaves_the_last_reading_alone() {
    let rig = ruled(Arc::new(FakeJudge::that_fails("not asked")));
    let id = on_the_rail(&rig).await;
    let reply = Arc::clone(&rig.fleet).judge_session(JudgeSession { session_id: id.clone() }).await.expect("asked");
    assert!(reply.judging);
    eventually(|| async { !record(&rig, &id).await.judging }).await;
    let read = record(&rig, &id).await;
    assert!(read.judge.is_none() && read.waiting_for.iter().all(|one| one.source != WaitingSource::Judge), "{read:?}");
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
async fn with_workflows_off_there_is_no_step_no_nudge_and_no_boundary_judge_but_the_tile_still_reads() {
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

    let reply = Arc::clone(&rig.fleet).judge_session(JudgeSession { session_id: id.clone() }).await.expect("asked");
    assert!(reply.judging);
    eventually(|| async { record(&rig, &id).await.judge.is_some() }).await;
    assert_eq!(asked_of(&judge).len(), 1, "the tile reads against the plan alone");
    assert!(asked_of(&judge)[0].contains("Stop at the end"));
}
