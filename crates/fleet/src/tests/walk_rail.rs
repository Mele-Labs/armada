//! The claim, written before the code: **Fleet places a Job on its workflow's steps before the plan
//! and around a walk from what it saw (a reply, a plan recorded, a window shown, a window approved),
//! and nudges once when a test is written before the walk is approved. Nothing blocks, and the
//! agent's word is never an input.** The owner's decision of 27 Oct 2026, "A feature Job walks before it tests".
//!
//! The first half is the rules, pure. The second drives a Fleet with a workflow that has a walk step.

use std::sync::Arc;

use api::Sessions;
use core_model::StepEnd;
use ipc::{Instant, SessionRow, SessionVoice, ShowWindow, SessionId};
use testkit::FakeJudge;

use super::session_rail::{
    approved, at, delivers, edit, fitted, follows, on_the_rail, plan_step, record, ruled_on, step, told,
};
use crate::rail::{self, Facts, Fit, RailStep, Skip};
use crate::rail_ends::{ends_when, passed_over_when};
use crate::rail_keeping::thread_facts;
use crate::test_paths::is_test_path;

// ---- The rules ----

fn propose() -> RailStep {
    RailStep { ends_on: vec![StepEnd::Replied], ..step("Propose") }
}

fn walk() -> RailStep {
    RailStep { walks: true, ..step("Walk") }
}

/// Propose, Discuss, Approve plan, Walk, Refine, Deliver.
fn feature() -> Vec<RailStep> {
    vec![propose(), step("Discuss"), plan_step(), walk(), follows("Refine"), delivers("Deliver")]
}

/// Find the cause (records the plan), Fix (follows the plan and walks), Tests, Deliver.
fn bug() -> Vec<RailStep> {
    vec![plan_step(), RailStep { walks: true, ..follows("Fix") }, fitted("Tests", Fit::Checked), delivers("Deliver")]
}

fn recorded() -> Facts {
    Facts { plan_recorded: true, ..Facts::default() }
}

fn approved_naming_a_walk() -> Facts {
    Facts { plan_recorded: true, plan_expects_walk: true, ..approved() }
}

#[test]
fn the_steps_before_the_plan_are_current_until_he_replies_and_until_a_plan_is_recorded() {
    let steps = feature();
    assert_eq!(at(&steps, Facts::default()), Some(0), "Propose before anything is seen");
    assert_eq!(at(&steps, Facts { replied: true, ..Facts::default() }), Some(1), "his reply enters Discuss");
    assert_eq!(at(&steps, recorded()), Some(2), "a recorded plan enters Approve plan");
    assert_eq!(at(&steps, Facts { replied: true, ..recorded() }), Some(2));
}

#[test]
fn the_recording_step_is_not_current_before_a_plan_is_recorded_when_a_step_precedes_it() {
    let steps = feature();
    let busy = Facts { edited: true, checked: true, committed: true, ..Facts::default() };
    assert_eq!(at(&steps, busy), Some(0), "work seen does not move a Job that has not been answered");
    assert_eq!(at(&steps, Facts { replied: true, ..busy }), Some(1));
    let alone = [plan_step(), follows("Implement"), delivers("Deliver")];
    assert_eq!(at(&alone, Facts::default()), Some(0), "with nothing before it, it is current from the start");
}

#[test]
fn a_walk_step_is_current_from_the_plans_approval_until_a_walk_is_approved() {
    let steps = feature();
    assert_eq!(at(&steps, approved_naming_a_walk()), Some(3));
    assert_eq!(at(&steps, Facts { walk_shown: true, ..approved_naming_a_walk() }), Some(3));
    let edited = Facts { walk_shown: true, edited: true, checked: true, committed: true, ..approved_naming_a_walk() };
    assert_eq!(at(&steps, edited), Some(3), "the work is not past it until he approves");
    assert_eq!(at(&steps, Facts { walk_approved: true, ..approved_naming_a_walk() }), Some(4), "a follows_plan step is entered once it is passed");
    assert_eq!(at(&steps, Facts { pushed: true, ..approved_naming_a_walk() }), Some(5));
}

#[test]
fn a_walk_of_its_own_is_passed_over_at_the_plans_approval_when_nothing_was_named_or_shown() {
    let steps = [propose(), step("Discuss"), plan_step(), walk(), follows("Refine")];
    let planned = Facts { plan_recorded: true, ..approved() };
    assert!(planned.walk_passed(&steps[3]));
    assert_eq!(at(&steps, planned), Some(4), "nothing named or shown: Refine, with no push to wait for");
    let pushed = Facts { pushed: true, ..planned };
    assert!(!Facts { plan_expects_walk: true, ..pushed }.walk_passed(&steps[3]), "a named walk is not passed over");
    assert!(!Facts { walk_shown: true, ..pushed }.walk_passed(&steps[3]), "nor one that was shown");
    assert_eq!(at(&steps, Facts { walk_shown: true, ..planned }), Some(3), "a window shown puts the walk back");
    assert!(
        !Facts { pushed: true, plan_recorded: true, ..Facts::default() }.walk_passed(&steps[3]),
        "nothing is passed over before the plan is approved"
    );
}

/// The idle Job of 30 Oct 2026: a bug fix nobody would see finished its tasks and committed, was told Fix
/// ends on his walk approval, and waited to be moved to Tests while Fleet waited for a push.
#[test]
fn a_fix_with_nothing_to_show_moves_on_to_tests_once_its_tasks_are_done_with_no_push() {
    let steps = bug();
    let fixing = Facts { plan_recorded: true, edited: true, committed: true, tasks_open: true, ..approved() };
    assert_eq!(at(&steps, fixing), Some(1), "Fix holds while the plan's tasks are open");
    assert_eq!(at(&steps, Facts { tasks_open: false, ..fixing }), Some(2), "its tasks done and nothing to show: Tests");
    assert_eq!(
        at(&steps, Facts { tasks_open: false, plan_expects_walk: true, ..fixing }),
        Some(1),
        "a fix he would see still waits for his approval"
    );
}

/// **The agent on Fix is told what ends it with nothing to show, and is nudged if it stops there.** It was
/// told only "the person approves the walk", which a Job with no walk read as already met.
#[tokio::test]
async fn the_agent_on_a_fix_is_told_its_tasks_end_it_with_nothing_to_show_and_is_nudged_when_it_stops() {
    let mut steps = bug();
    steps[1].instructions = Some("Make the fix.".into());
    let said = rail::instructed("bug", &steps, 1).unwrap_or_default();
    assert!(said.contains("Where nothing would be seen, it ends when you have marked each of the plan's tasks completed"), "{said}");
    let nudge = rail::stall_nudge("bug", &steps[1], crate::rail_ends::End::TasksDone);
    assert!(nudge.contains("marked each of the plan's tasks completed with TaskUpdate"), "{nudge}");

    let (rig, id) = walking_rig().await;
    let mut stopped = record(&rig, &id).await;
    stopped.workflow_step = Some(1);
    stopped.standing = Some(ipc::SessionStanding::Replied);
    stopped.waiting_for.clear();
    let nothing = rail::Extra::default();
    assert_eq!(rail::stalled(&steps, &stopped, nothing).map(|(at, _)| at), Some(1), "stopped on Fix, nothing to show");
    let walking = rail::Extra { plan_expects_walk: true, ..nothing };
    assert!(rail::stalled(&steps, &stopped, walking).is_none(), "a walk to show is his to approve, not a stall");
}

#[test]
fn a_fix_that_walks_is_current_until_he_approves_and_tests_follow_at_once() {
    let steps = bug();
    let fixing = Facts { plan_recorded: true, plan_expects_walk: true, edited: true, ..approved() };
    assert_eq!(at(&steps, fixing), Some(1), "Fix is current from the plan's approval");
    assert_eq!(at(&steps, Facts { walk_shown: true, checked: true, ..fixing }), Some(1), "a Check run does not leave the walk");
    assert_eq!(at(&steps, Facts { walk_shown: true, walk_approved: true, ..fixing }), Some(2), "his approval enters Tests, with no Check run yet");
}

/// The loop of 30 Oct 2026: with the walk a step of its own after Fix, his approval entered nothing, the
/// Job fell back to Fix, was told to show the walk, and the window shown again voided his approval.
#[test]
fn a_walk_he_approved_never_drops_the_job_back_to_the_step_before_it() {
    let steps = [plan_step(), follows("Fix"), walk(), fitted("Tests", Fit::Checked), delivers("Deliver")];
    let fixed = Facts { plan_recorded: true, edited: true, committed: true, walk_shown: true, ..approved() };
    assert_eq!(at(&steps, Facts { walk_approved: true, ..fixed }), Some(3), "approved, no Check run: Tests");
    assert_eq!(at(&steps, Facts { walk_approved: true, tasks_open: true, ..fixed }), Some(3), "a task left open does not hold him behind it");
    assert_eq!(at(&steps, Facts { pushed: true, ..approved() }), Some(4), "passed over and pushed: Deliver");
}

#[test]
fn a_test_edited_before_the_walk_is_approved_is_a_skip_of_the_walk_step_when_a_walk_was_named_or_shown() {
    let steps = feature();
    let tested = Facts { test_edited: true, ..approved_naming_a_walk() };
    assert_eq!(rail::skipped(&steps, &tested), [(3, Skip::Walk)]);
    assert!(rail::skipped(&steps, &Facts { walk_approved: true, ..tested }).is_empty(), "approved");
    let unnamed = Facts { plan_expects_walk: false, ..tested };
    assert!(rail::skipped(&steps, &unnamed).is_empty(), "no walk named and none shown: tests come at once for a change nobody sees");
    assert_eq!(rail::skipped(&steps, &Facts { walk_shown: true, ..unnamed }), [(3, Skip::Walk)], "a window shown");
    let plain = [plan_step(), follows("Implement"), delivers("Hand off")];
    assert!(rail::skipped(&plain, &tested).is_empty(), "a workflow with no walk step has nothing to skip");
    assert!(rail::skipped(&steps, &approved_naming_a_walk()).is_empty(), "an edit that is no test is not this skip");
}

#[test]
fn where_a_workflow_walks_an_edit_before_the_plan_is_approved_is_no_skip_but_a_test_or_a_push_is() {
    let steps = feature();
    let drafting = Facts { plan_recorded: true, ..Facts::default() };
    assert!(rail::skipped(&steps, &Facts { edited: true, ..drafting }).is_empty(), "a rough mock is allowed while discussing");
    let tested = Facts { edited: true, test_edited: true, ..drafting };
    assert_eq!(rail::skipped(&steps, &tested), [(2, Skip::PlanTested)], "and the walk skip is not told on top of it");
    assert_eq!(rail::skipped(&steps, &Facts { walk_shown: true, ..tested }), [(2, Skip::PlanTested)]);
    assert_eq!(rail::skipped(&steps, &Facts { edited: true, pushed: true, ..drafting }), [(2, Skip::PlanPushed)]);
    assert!(rail::skipped(&steps, &Facts { test_edited: true, ..approved() }).is_empty(), "approved, and nothing named or shown");
    let plain = [plan_step(), follows("Implement"), delivers("Hand off")];
    assert_eq!(rail::skipped(&plain, &Facts { edited: true, ..Facts::default() }), [(0, Skip::Plan)], "without a walk step the edit is still a skip");
    assert!(rail::skipped(&plain, &Facts { edited: true, pushed: true, ..approved() }).is_empty());
}

#[test]
fn the_nudge_for_a_test_before_the_walk_is_worded_as_the_others_are() {
    let steps = feature();
    let said = rail::nudge("Feature", &steps[3], Skip::Walk);
    for part in ["Feature", "\"Walk\"", "edited a test before the walk was approved in", "Nothing is blocked"] {
        assert!(said.contains(part), "{part}: {said}");
    }
    let about = RailStep { about: Some("Show it as a walk.".into()), ..walk() };
    assert!(rail::nudge("Feature", &about, Skip::Walk).contains("Show it as a walk."), "the step's own words");
    assert_eq!(Skip::Walk.fact(), "a test before the walk was approved");
    let plan = rail::nudge("Feature", &steps[2], Skip::PlanTested);
    assert!(plan.contains("edited a test before the plan was approved in") && plan.contains("record_plan"), "{plan}");
    assert!(rail::nudge("Feature", &steps[2], Skip::PlanPushed).contains("pushed before the plan was approved in"));
    assert_eq!(Skip::PlanTested.fact(), "a test before the plan was approved");
    assert_eq!(Skip::PlanPushed.fact(), "pushed before the plan was approved");
}

#[test]
fn what_ends_each_step_is_read_off_its_anchor() {
    let ends = |steps: &[RailStep]| (0..steps.len()).map(|one| ends_when(steps, one)).collect::<Vec<_>>();
    let said = |words: &[Option<&str>]| words.iter().map(|word| word.map(String::from)).collect::<Vec<_>>();
    assert_eq!(
        ends(&feature()),
        said(&[
            Some("you reply"),
            Some("a plan is recorded"),
            Some("you approve the plan"),
            Some("you approve the walk"),
            Some("it is pushed"),
            Some("it is merged")
        ])
    );
    assert_eq!(
        ends(&bug()),
        said(&[Some("you approve the plan"), Some("you approve the walk"), Some("it is pushed"), Some("it is merged")])
    );
    let loose = [fitted("Build", Fit::Edited), step("Free text")];
    assert_eq!(ends(&loose), said(&[None, None]));
    assert_eq!(ends_when(&feature(), 9), None, "no such step");
    assert_eq!(passed_over_when(&walk()), Some("nothing would be seen"));
    assert_eq!(passed_over_when(&step("Refine")), None);
}

fn at_second(n: u32) -> Instant {
    Instant::carried(&format!("2026-10-27T09:00:{n:02}.000Z"))
}

fn said(n: u32, from: SessionVoice, text: &str) -> SessionRow {
    SessionRow::Message { id: format!("m{n}"), at: at_second(n), from, text: text.into(), files: Vec::new(), tags: Vec::new(), notice: None, queued: false }
}

fn message(n: u32, from: SessionVoice) -> SessionRow {
    said(n, from, "hello")
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
    said(n, SessionVoice::You, &format!("Approved: {url}"))
}

#[test]
fn the_person_has_replied_only_with_a_message_of_his_after_the_agents_first_turn() {
    let you = || SessionVoice::You;
    let fleet = || SessionVoice::Session { id: "fleet".into(), title: "Fleet".into() };
    assert!(!thread_facts(&[message(1, you())], None).replied, "his request is not a reply");
    assert!(!thread_facts(&[message(1, you()), message(2, SessionVoice::Agent)], None).replied, "the agent has spoken and he has not");
    assert!(thread_facts(&[message(1, you()), message(2, SessionVoice::Agent), message(3, you())], None).replied);
    assert!(!thread_facts(&[message(1, you()), message(2, SessionVoice::Agent), message(3, fleet())], None).replied, "Fleet's notice is not his word");
    let tool = SessionRow::Tool { id: "t1".into(), at: at_second(2), text: "ask_the_person".into(), subagent: None };
    assert!(thread_facts(&[message(1, you()), tool, message(3, you())], None).replied, "an answer to a question the agent asked");
}

#[test]
fn a_window_is_shown_by_its_row_and_approved_only_by_his_approve_at_or_after_it() {
    let url = "http://localhost:5391/?walk=a";
    assert_eq!(thread_facts(&[], None).walk_shown, false);
    let shown = thread_facts(&[window(1, url)], None);
    assert!(shown.walk_shown && !shown.any_walk_approved);
    assert!(thread_facts(&[window(1, url), approve(2, url)], None).any_walk_approved);
    assert!(!thread_facts(&[approve(1, url), window(2, url)], None).any_walk_approved, "a page shown again must be approved again");
    assert!(!thread_facts(&[window(1, url), approve(2, "http://localhost:5391/?walk=b")], None).any_walk_approved, "another page's approve");
    let fleet = said(2, SessionVoice::Session { id: "fleet".into(), title: "Fleet".into() }, &format!("Approved: {url}"));
    assert!(!thread_facts(&[window(1, url), fleet], None).any_walk_approved, "only the person approves");
}

#[test]
fn a_test_path_is_one_by_the_conventions_kept_in_one_place() {
    for path in [
        "apps/desktop/src/a.test.tsx",
        "crates/fleet/src/tests/rail.rs",
        "crates/acceptance/tests/triggers.rs",
        "packages/screens/src/__tests__/x.ts",
        "tools/hooks_test.py",
    ] {
        assert!(is_test_path(path), "{path}");
    }
    for path in ["crates/fleet/src/rail.rs", "docs/practices/rust.md", "apps/desktop/src/testing.ts", "tests", "crates/contest/src/lib.rs", ""] {
        assert!(!is_test_path(path), "{path}");
    }
}

// ---- Through a Fleet ----

fn walking_workflow() -> config::ResolvedWorkflow {
    let text = "version: 1\nworkflow_id: rail\nname: Rail\nsteps:\n  \
        - id: propose\n    label: Propose\n    awaits_reply: true\n    delivers: false\n    advance_gate: auto\n  \
        - id: plan\n    label: \"Plan the change\"\n    evidence: {submitted: {type: plan}}\n    \
        mechanical_checks:\n      - { type: plan_recorded, min_tasks: 1 }\n    delivers: false\n    advance_gate: auto\n  \
        - id: walk\n    label: Walk\n    walks: true\n    delivers: false\n    advance_gate: auto\n  \
        - id: refine\n    label: Refine\n    follows_plan: true\n    delivers: false\n    advance_gate: auto\n  \
        - id: handoff\n    label: \"Hand off\"\n    delivers: true\n    advance_gate: auto\n";
    let def = config::WorkflowDef::parse(std::path::Path::new("rail.yml"), text, &config::Roster::offering_nothing())
        .unwrap_or_else(|refused| panic!("the fixture did not parse: {refused}"));
    config::ResolvedWorkflow::resolve(&def, &super::daemon::manifest())
        .unwrap_or_else(|refused| panic!("the fixture did not resolve: {refused}"))
}

const PAGE: &str = "http://localhost:5391/?walk=reader";

async fn walking_rig() -> (super::session_host::Rig, SessionId) {
    let rig = ruled_on(Arc::new(FakeJudge::with_no_objection()), walking_workflow());
    let id = on_the_rail(&rig).await;
    (rig, id)
}

async fn shown(rig: &super::session_host::Rig, id: &SessionId) {
    rig.fleet
        .show_window(None, ShowWindow { url: PAGE.into(), title: None, session_id: Some(id.clone()), ..Default::default() })
        .await
        .expect("shown");
}

async fn plan_approved(rig: &super::session_host::Rig, id: &SessionId) {
    super::session_rail::approved_plan(rig, id).await;
}

async fn step_of(rig: &super::session_host::Rig, id: &SessionId) -> Option<u32> {
    record(rig, id).await.workflow_step
}

#[tokio::test]
async fn a_window_shown_after_the_plan_is_approved_puts_the_job_on_the_walk_until_he_approves_it() {
    let (rig, id) = walking_rig().await;
    plan_approved(&rig, &id).await;
    assert_eq!(step_of(&rig, &id).await, Some(3), "approved with no walk named or shown: Refine");
    shown(&rig, &id).await;
    assert_eq!(step_of(&rig, &id).await, Some(2), "a window shown puts the Job on the walk");
    rig.send(&id, &format!("Approved: {PAGE}")).await;
    assert_eq!(step_of(&rig, &id).await, Some(3), "approved: Refine");
}

#[tokio::test]
async fn a_test_edited_before_the_walk_is_approved_draws_one_nudge_kept_against_the_step() {
    let (rig, id) = walking_rig().await;
    shown(&rig, &id).await;
    plan_approved(&rig, &id).await;

    rig.stand_in.says(0, vec![edit("crates/reader/tests/reader.rs")]);
    super::session_host::eventually(|| async { told(&rig, "edited a test before the walk was approved") == 1 }).await;
    rig.stand_in.says(0, vec![edit("crates/reader/tests/more.rs")]);
    rig.fleet.timeline().settled().await;
    assert_eq!(told(&rig, "edited a test before the walk was approved"), 1, "once for the Job");

    let source = walking_workflow().source().as_wire();
    super::session_host::eventually(|| async { !rig.fleet.step_nudges("rail", source, "walk").await.is_empty() }).await;
    let kept = rig.fleet.step_nudges("rail", source, "walk").await;
    assert_eq!(kept.len(), 1);
    assert_eq!((kept[0].session_id.as_str(), kept[0].fact.as_str()), (id.as_str(), "a test before the walk was approved"));
    assert!(kept[0].counts());
    assert!(rig.fleet.nudge_not_a_drift(kept[0].id).await);
    assert!(!rig.fleet.step_nudges("rail", source, "walk").await[0].counts(), "marked, it stops counting");
}

#[tokio::test]
async fn a_test_is_not_nudged_when_the_walk_was_approved_or_nothing_would_be_seen() {
    let (rig, id) = walking_rig().await;
    plan_approved(&rig, &id).await;
    rig.stand_in.says(0, vec![edit("crates/reader/tests/reader.rs")]);
    rig.fleet.timeline().settled().await;
    assert_eq!(told(&rig, "edited a test before"), 0, "no window, none named: nothing to walk");

    let (rig, id) = walking_rig().await;
    shown(&rig, &id).await;
    plan_approved(&rig, &id).await;
    rig.send(&id, &format!("Approved: {PAGE}")).await;
    rig.stand_in.says(0, vec![edit("crates/reader/tests/reader.rs")]);
    rig.fleet.timeline().settled().await;
    assert_eq!(told(&rig, "edited a test before"), 0, "approved");
}

#[tokio::test]
async fn before_the_plan_is_approved_a_rough_mock_is_no_skip_but_a_test_and_a_push_are_nudged_once() {
    let (rig, id) = walking_rig().await;
    rig.stand_in.says(0, vec![edit("apps/desktop/src/mock/rough.ts")]);
    rig.fleet.timeline().settled().await;
    assert_eq!(told(&rig, "before the plan was approved"), 0, "a mock edited while discussing");
    assert_eq!(told(&rig, "edited files before"), 0);

    rig.stand_in.says(0, vec![edit("crates/reader/tests/reader.rs")]);
    super::session_host::eventually(|| async { told(&rig, "edited a test before the plan was approved") == 1 }).await;
    rig.stand_in.says(0, vec![edit("crates/reader/tests/more.rs")]);
    rig.fleet.timeline().settled().await;
    assert_eq!(told(&rig, "edited a test before the plan was approved"), 1, "once for the step");
    assert_eq!(told(&rig, "edited a test before the walk was approved"), 0, "and not the walk's nudge on top of it");

    let (rig, id2) = walking_rig().await;
    let harness = record(&rig, &id2).await.harness;
    rig.fleet
        .report_session(ipc::SessionReport { harness, session_id: id2.clone(), fact: ipc::SessionFact::Pushed { branch: Some("armada/reader".into()) } })
        .await
        .expect("kept");
    super::session_host::eventually(|| async { told(&rig, "pushed before the plan was approved") == 1 }).await;
    let _ = id;
}

#[tokio::test]
async fn a_plan_that_says_seen_expects_a_walk_before_any_window_is_shown_and_one_that_does_not_does_not() {
    let (rig, id) = walking_rig().await;
    super::session_rail::approved_plan_saying(&rig, &id, true).await;
    rig.stand_in.says(0, vec![edit("crates/reader/tests/reader.rs")]);
    super::session_host::eventually(|| async { told(&rig, "edited a test before the walk was approved") == 1 }).await;

    let (rig, id) = walking_rig().await;
    super::session_rail::approved_plan_saying(&rig, &id, false).await;
    rig.stand_in.says(0, vec![edit("crates/reader/tests/reader.rs")]);
    rig.fleet.timeline().settled().await;
    assert_eq!(told(&rig, "edited a test before"), 0, "said nothing seen, named no walk, showed none");
}
