//! The claim, written before the code: **Fleet places a Job on its workflow's steps before the plan
//! and around a walk from what it saw (a reply, a plan recorded, a window shown, a window approved),
//! and nudges once when a test is written before the walk is approved. Nothing blocks, and the
//! agent's word is never an input.** The owner's decision of 27 Oct 2026, "A feature Job walks before it tests".
//!
//! The first half is the rules, pure. The second drives a Fleet with a workflow that has a walk step.

use std::sync::Arc;

use api::Sessions;
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
    RailStep { awaits_reply: true, ..step("Propose") }
}

fn walk() -> RailStep {
    RailStep { walks: true, ..step("Walk") }
}

/// Propose, Discuss, Approve plan, Walk, Refine, Deliver.
fn feature() -> Vec<RailStep> {
    vec![propose(), step("Discuss"), plan_step(), walk(), follows("Refine"), delivers("Deliver")]
}

/// Find the cause (records the plan), Fix, Short walk, Tests, Deliver.
fn bug() -> Vec<RailStep> {
    vec![plan_step(), follows("Fix"), walk(), fitted("Tests", Fit::Checked), delivers("Deliver")]
}

fn recorded() -> Facts {
    Facts { plan_recorded: true, ..Facts::default() }
}

fn approved_naming_a_walk() -> Facts {
    Facts { plan_recorded: true, plan_names_walk: true, ..approved() }
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
fn a_walk_is_passed_over_when_the_plan_named_none_and_no_window_was_shown() {
    let steps = feature();
    let nothing = Facts { plan_recorded: true, ..approved() };
    assert!(nothing.walk_passed());
    assert_eq!(at(&steps, nothing), Some(4), "Refine follows at once");
    assert_eq!(at(&steps, Facts { walk_shown: true, ..nothing }), Some(3), "a window shown brings the walk back");
    assert!(!Facts { plan_recorded: true, ..Facts::default() }.walk_passed(), "nothing is passed over before the plan is approved");
}

#[test]
fn a_walk_between_a_fix_and_its_tests_is_entered_when_a_window_is_shown_and_tests_wait_for_it() {
    let steps = bug();
    let fixing = Facts { plan_recorded: true, edited: true, ..approved() };
    assert_eq!(at(&steps, fixing), Some(1), "no window yet: the fix");
    assert_eq!(at(&steps, Facts { walk_shown: true, checked: true, ..fixing }), Some(2), "a Check run does not leave the walk");
    assert_eq!(at(&steps, Facts { walk_shown: true, walk_approved: true, checked: true, ..fixing }), Some(3));
}

#[test]
fn a_test_edited_before_the_walk_is_approved_is_a_skip_of_the_walk_step() {
    let steps = feature();
    let tested = Facts { test_edited: true, ..approved_naming_a_walk() };
    assert_eq!(rail::skipped(&steps, &tested), [(3, Skip::Walk)]);
    assert_eq!(rail::skipped(&steps, &Facts { test_edited: true, plan_recorded: true, ..Facts::default() }), [(3, Skip::Walk)], "before the plan is approved too");
    assert!(rail::skipped(&steps, &Facts { walk_approved: true, ..tested }).is_empty(), "approved");
    assert!(rail::skipped(&steps, &Facts { plan_names_walk: false, ..tested }).is_empty(), "passed over: nothing would be seen");
    assert_eq!(rail::skipped(&steps, &Facts { plan_names_walk: false, walk_shown: true, ..tested }), [(3, Skip::Walk)]);
    let plain = [plan_step(), follows("Implement"), delivers("Hand off")];
    assert!(rail::skipped(&plain, &tested).is_empty(), "a workflow with no walk step has nothing to skip");
    assert!(rail::skipped(&steps, &approved_naming_a_walk()).is_empty(), "an edit that is no test is not this skip");
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
}

#[test]
fn what_ends_each_step_is_read_off_its_anchor() {
    let ends = |steps: &[RailStep]| (0..steps.len()).map(|one| ends_when(steps, one)).collect::<Vec<_>>();
    assert_eq!(
        ends(&feature()),
        [
            Some("you reply"),
            Some("a plan is recorded"),
            Some("you approve the plan"),
            Some("you approve the walk"),
            Some("it is pushed"),
            Some("it is merged")
        ]
    );
    assert_eq!(
        ends(&bug()),
        [Some("you approve the plan"), Some("a walk is shown"), Some("you approve the walk"), Some("it is pushed"), Some("it is merged")]
    );
    let loose = [fitted("Build", Fit::Edited), step("Free text")];
    assert_eq!(ends(&loose), [None, None]);
    assert_eq!(ends_when(&feature(), 9), None, "no such step");
    assert_eq!(passed_over_when(&walk()), Some("nothing would be seen"));
    assert_eq!(passed_over_when(&step("Refine")), None);
}

fn at_second(n: u32) -> Instant {
    Instant::carried(&format!("2026-10-27T09:00:{n:02}.000Z"))
}

fn said(n: u32, from: SessionVoice, text: &str) -> SessionRow {
    SessionRow::Message { id: format!("m{n}"), at: at_second(n), from, text: text.into(), files: Vec::new(), tags: Vec::new(), notice: None }
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
    assert!(!thread_facts(&[message(1, you())]).replied, "his request is not a reply");
    assert!(!thread_facts(&[message(1, you()), message(2, SessionVoice::Agent)]).replied, "the agent has spoken and he has not");
    assert!(thread_facts(&[message(1, you()), message(2, SessionVoice::Agent), message(3, you())]).replied);
    assert!(!thread_facts(&[message(1, you()), message(2, SessionVoice::Agent), message(3, fleet())]).replied, "Fleet's notice is not his word");
    let tool = SessionRow::Tool { id: "t1".into(), at: at_second(2), text: "ask_the_person".into() };
    assert!(thread_facts(&[message(1, you()), tool, message(3, you())]).replied, "an answer to a question the agent asked");
}

#[test]
fn a_window_is_shown_by_its_row_and_approved_only_by_his_approve_at_or_after_it() {
    let url = "http://localhost:5391/?walk=a";
    assert_eq!(thread_facts(&[]).walk_shown, false);
    let shown = thread_facts(&[window(1, url)]);
    assert!(shown.walk_shown && !shown.walk_approved);
    assert!(thread_facts(&[window(1, url), approve(2, url)]).walk_approved);
    assert!(!thread_facts(&[approve(1, url), window(2, url)]).walk_approved, "a page shown again must be approved again");
    assert!(!thread_facts(&[window(1, url), approve(2, "http://localhost:5391/?walk=b")]).walk_approved, "another page's approve");
    let fleet = said(2, SessionVoice::Session { id: "fleet".into(), title: "Fleet".into() }, &format!("Approved: {url}"));
    assert!(!thread_facts(&[window(1, url), fleet]).walk_approved, "only the person approves");
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
    assert_eq!(step_of(&rig, &id).await, Some(3), "nothing shown and none named: the walk is passed over, Refine follows");
    shown(&rig, &id).await;
    assert_eq!(step_of(&rig, &id).await, Some(2), "a window shown is the walk");
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
