//! The claim, written before the code: **a Job on a step that ends on `pr: approved` is told once to
//! stop, raises no wait while it does, and is woken by one line when the pull request is approved,
//! with the next step's words in the same message; the person writing on the step or skipping it
//! moves the Job on without that line.** Through a Fleet, with the readers' own door
//! (`lifecycle_seen`) and no forge, tracker or process reached. The owner approved the design on
//! 29 Oct 2026.

use std::sync::Arc;

use api::{HostedSessions, Sessions};
use core_model::{ForgeFact, Observed};
use ipc::{SessionFact, SessionId, SessionReport, SessionStanding, SkipSessionStep};
use store::AttachmentState;
use testkit::FakeJudge;

use super::lifecycle_keeping::holds;
use super::session_host::{eventually, Rig};
use super::session_rail::{approved_plan, on_the_rail, record, ruled_on, told};

const WAKE: &str = "PR #812 is Approved. That ends the \"Review\" step of this Job's Rail workflow.";

/// Plan, Review (ends when every pull request is approved), Close out.
pub(super) fn reviewing(ends: &str) -> String {
    format!(
        "version: 1\nworkflow_id: rail\nname: Rail\nsteps:\n  \
        - id: plan\n    label: \"Plan the change\"\n    evidence: {{submitted: {{type: plan}}}}\n    \
        mechanical_checks:\n      - {{ type: plan_recorded, min_tasks: 1 }}\n    delivers: false\n    advance_gate: auto\n  \
        - id: review\n    label: \"Review\"\n    instructions: \"Answer the review.\"\n    ends_on:\n      - \"{ends}\"\n    \
        delivers: false\n    advance_gate: auto\n  \
        - id: close\n    label: \"Close out\"\n    instructions: \"Write the summary.\"\n    delivers: false\n    advance_gate: auto\n"
    )
}

/// Plan, Land (delivers; `ends` gates its merge).
fn landing(ends: &str) -> String {
    format!(
        "version: 1\nworkflow_id: rail\nname: Rail\nsteps:\n  \
        - id: plan\n    label: \"Plan the change\"\n    evidence: {{submitted: {{type: plan}}}}\n    \
        mechanical_checks:\n      - {{ type: plan_recorded, min_tasks: 1 }}\n    delivers: false\n    advance_gate: auto\n  \
        - id: land\n    label: \"Land\"\n    instructions: \"Open the pull request and merge it.\"\n    ends_on:\n      - \"{ends}\"\n    \
        delivers: true\n    advance_gate: auto\n"
    )
}

pub(super) fn resolved(text: &str) -> config::ResolvedWorkflow {
    let def = config::WorkflowDef::parse(std::path::Path::new("rail.yml"), text, &config::Roster::offering_nothing())
        .unwrap_or_else(|refused| panic!("the fixture did not parse: {refused}"));
    config::ResolvedWorkflow::resolve(&def, &super::daemon::manifest())
        .unwrap_or_else(|refused| panic!("the fixture did not resolve: {refused}"))
}

/// A Job holding pull request 812 and on its Review step, with the plan approved.
async fn waiting() -> (Rig, SessionId) {
    let rig = ruled_on(Arc::new(FakeJudge::with_no_objection()), resolved(&reviewing("pr: approved")));
    let id = on_the_rail(&rig).await;
    holds(&rig, &id, "pr", "812", AttachmentState::Standing, &[("state", "open")]).await;
    rig.fleet.lifecycles_moved(record(&rig, &id).await).await;
    approved_plan(&rig, &id).await;
    eventually(|| async { record(&rig, &id).await.workflow_step == Some(1) }).await;
    (rig, id)
}

async fn seen(rig: &Rig, id: &SessionId, fact: ForgeFact) {
    rig.fleet.lifecycle_seen(id.as_str(), "pr", "812", Observed::Forge(fact)).await.expect("a move");
}

/// The line the stand-in was sent, as the agent reads it: the stand-in keeps the JSON it was handed.
fn sent(rig: &Rig, containing: &str) -> String {
    let line = rig.stand_in.process(0).sent.lock().unwrap().iter().find(|line| line.contains(containing)).cloned();
    line.expect("a line was sent").replace("\\\"", "\"").replace("\\n", "\n")
}

#[tokio::test]
async fn a_job_waiting_on_an_approval_is_told_to_stop_and_woken_by_it_with_the_next_steps_words() {
    let (rig, id) = waiting().await;
    eventually(|| async { told(&rig, "Answer the review.") == 1 }).await;
    let said = sent(&rig, "Answer the review.");
    assert!(said.contains("This step ends when every pr is approved, which Fleet watches for."), "{said}");
    assert!(said.contains("End your turn now: Fleet wakes you with a message when it comes"), "{said}");

    seen(&rig, &id, ForgeFact::InReview).await;
    rig.stand_in.finishes(0, "Waiting for the review.");
    eventually(|| async { record(&rig, &id).await.standing == Some(SessionStanding::Replied) }).await;
    rig.fleet.timeline().settled().await;
    let waiting = record(&rig, &id).await;
    assert_eq!(waiting.workflow_step, Some(1), "a reading short of the state moves nothing");
    assert_eq!(waiting.workflow_waiting_on.as_deref(), Some("Waiting on PR #812: Approved"), "the row says what the step waits on");
    assert!(waiting.waiting_for.is_empty(), "the world's to end: no wait for the person");
    assert_eq!((told(&rig, "Fleet does not move it for you"), told(&rig, "Write the summary.")), (0, 0), "no nudge, no next step yet");

    seen(&rig, &id, ForgeFact::Approved).await;
    eventually(|| async { record(&rig, &id).await.workflow_step == Some(2) }).await;
    eventually(|| async { told(&rig, "That ends") == 1 }).await;
    assert_eq!(record(&rig, &id).await.workflow_waiting_on, None, "and says nothing once it is approved");
    let woken = sent(&rig, "That ends");
    assert!(woken.contains(WAKE), "{woken}");
    assert!(woken.contains("Write the summary."), "the next step's words ride in the same message: {woken}");
    assert!(woken.find(WAKE) < woken.find("Write the summary."), "what happened first: {woken}");
    assert_eq!(told(&rig, "Write the summary."), 1, "once");

    rig.fleet.timeline().settled().await;
    assert!(rig.fleet.lifecycle_seen(id.as_str(), "pr", "812", Observed::Forge(ForgeFact::Approved)).await.is_none(), "a repeat moves nothing");
    assert_eq!(told(&rig, "That ends"), 1, "and wakes nothing");
}

#[tokio::test]
async fn the_person_writing_on_the_step_moves_the_job_on_and_no_line_says_it_was_approved() {
    let (rig, id) = waiting().await;
    eventually(|| async { told(&rig, "Answer the review.") == 1 }).await;
    rig.send(&id, "Go on without the review.").await;
    eventually(|| async { record(&rig, &id).await.workflow_step == Some(2) }).await;
    eventually(|| async { told(&rig, "Write the summary.") == 1 }).await;
    rig.fleet.timeline().settled().await;
    assert_eq!(told(&rig, "That ends"), 0, "nothing got there");
}

#[tokio::test]
async fn skipping_the_step_moves_the_job_on_and_no_line_says_it_was_approved() {
    let (rig, id) = waiting().await;
    eventually(|| async { told(&rig, "Answer the review.") == 1 }).await;
    Arc::clone(&rig.fleet).skip_session_step(SkipSessionStep { session_id: id.clone(), step: 1, skipped: true }).await.expect("skipped");
    eventually(|| async { record(&rig, &id).await.workflow_step == Some(2) }).await;
    eventually(|| async { told(&rig, "Write the summary.") == 1 }).await;
    rig.fleet.timeline().settled().await;
    assert_eq!(told(&rig, "That ends"), 0);
}

#[tokio::test]
async fn a_job_that_holds_no_pull_request_is_not_on_the_step_until_it_does() {
    let rig = ruled_on(Arc::new(FakeJudge::with_no_objection()), resolved(&reviewing("pr: approved")));
    let id = on_the_rail(&rig).await;
    approved_plan(&rig, &id).await;
    rig.fleet.timeline().settled().await;
    assert_eq!(record(&rig, &id).await.workflow_step, Some(0), "plan approved, nothing to wait on: the rail holds where it was");
    assert_eq!(told(&rig, "Answer the review."), 0);
}

/// A Job holding pull request 812, past its plan, and pushed: on Land.
async fn on_land() -> (Rig, SessionId) {
    let rig = ruled_on(Arc::new(FakeJudge::with_no_objection()), resolved(&landing("pr: approved")));
    let id = on_the_rail(&rig).await;
    holds(&rig, &id, "pr", "812", AttachmentState::Standing, &[("state", "open")]).await;
    rig.fleet.lifecycles_moved(record(&rig, &id).await).await;
    approved_plan(&rig, &id).await;
    let harness = record(&rig, &id).await.harness;
    let pushed = SessionReport { harness, session_id: id.clone(), fact: SessionFact::Pushed { branch: Some("armada/reader".into()) } };
    rig.fleet.report_session(pushed).await.expect("kept");
    eventually(|| async { record(&rig, &id).await.workflow_step == Some(1) }).await;
    (rig, id)
}

const LIFTED: &str = "That lifts the wait on the \"Land\" step";

#[tokio::test]
async fn land_is_told_to_hold_the_merge_and_woken_once_to_land_when_the_pull_request_is_approved() {
    let (rig, id) = on_land().await;
    eventually(|| async { told(&rig, "Do not merge until every pr is approved.") == 1 }).await;

    seen(&rig, &id, ForgeFact::ReviewersOn).await;
    rig.fleet.timeline().settled().await;
    assert_eq!((told(&rig, "lifts the wait"), record(&rig, &id).await.workflow_step), (0, Some(1)), "reviewers asked is not approval");

    seen(&rig, &id, ForgeFact::Approved).await;
    eventually(|| async { told(&rig, "lifts the wait") == 1 }).await;
    let woken = sent(&rig, "lifts the wait");
    assert!(woken.contains(&format!("Fleet: PR #812 is Approved. {LIFTED} of this Job's Rail workflow: land it now.")), "{woken}");
    assert_eq!(record(&rig, &id).await.workflow_step, Some(1), "the step is still Land: only the merge passes it");

    rig.fleet.timeline().settled().await;
    assert!(rig.fleet.lifecycle_seen(id.as_str(), "pr", "812", Observed::Forge(ForgeFact::Approved)).await.is_none());
    seen(&rig, &id, ForgeFact::InReview).await;
    seen(&rig, &id, ForgeFact::Approved).await;
    rig.fleet.timeline().settled().await;
    assert_eq!(told(&rig, "lifts the wait"), 1, "an approval taken back and given again says nothing the thread does not hold");
}

#[tokio::test]
async fn the_person_writing_on_land_lifts_the_wait_himself_and_no_line_follows() {
    let (rig, id) = on_land().await;
    eventually(|| async { told(&rig, "Do not merge until every pr is approved.") == 1 }).await;
    rig.send(&id, "Merge it when you are ready.").await;
    seen(&rig, &id, ForgeFact::Approved).await;
    rig.fleet.timeline().settled().await;
    assert_eq!(told(&rig, "lifts the wait"), 0);
}
