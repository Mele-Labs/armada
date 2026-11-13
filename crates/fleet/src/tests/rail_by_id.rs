//! A Job's rail is kept by step id, so a workflow saved with a step added before the one a Job stands on does
//! not move the Job. `docs/concepts/session.md`, *The rail*.

use std::sync::Arc;

use testkit::FakeJudge;

use super::lifecycle_waking::resolved;
use super::session_rail::{on_the_rail, record, ruled_on, workflow};
use super::session_host::eventually;
use crate::repositories::Catalogued;

/// The `rail` workflow with a `triage` step added in front of Plan.
fn with_triage() -> config::ResolvedWorkflow {
    resolved(
        "version: 1\nworkflow_id: rail\nname: Rail\nsteps:\n  \
        - id: triage\n    label: \"Triage\"\n    delivers: false\n    advance_gate: auto\n  \
        - id: plan\n    label: \"Plan the change\"\n    evidence: {submitted: {type: plan}}\n    \
        mechanical_checks:\n      - { type: plan_recorded, min_tasks: 1 }\n    delivers: false\n    advance_gate: auto\n  \
        - id: implement\n    label: \"Implement\"\n    follows_plan: true\n    delivers: false\n    advance_gate: auto\n  \
        - id: handoff\n    label: \"Hand off\"\n    delivers: true\n    advance_gate: auto\n",
    )
}

#[tokio::test]
async fn a_step_added_before_the_one_a_job_stands_on_does_not_move_the_job() {
    let rig = ruled_on(Arc::new(FakeJudge::with_no_objection()), workflow());
    let id = on_the_rail(&rig).await;
    eventually(|| async { !record(&rig, &id).await.workflow_visits.is_empty() }).await;
    let before = record(&rig, &id).await;
    assert_eq!(before.workflow_step, Some(0), "it stands on Plan, the first step");

    let served = rig.fleet.repositories().serving(rig.manifest.as_str()).expect("served");
    let added = with_triage();
    served.catalogued(Catalogued { workflows: super::daemon::one(added), left_out: Vec::new(), files: Vec::new() });

    let after = record(&rig, &id).await;
    assert_eq!(after.workflow_step, Some(1), "Plan is the second step now, and the Job is still on it");
    assert_eq!(after.workflow_visits.iter().map(|visit| visit.step).collect::<Vec<_>>(), [1], "and so is the visit it kept");
}

#[test]
fn a_step_before_the_one_the_job_stood_on_that_it_never_began_is_passed_over() {
    use crate::rail::Facts;
    use crate::rail_lifecycle::StepBits;
    use super::session_rail::{at, plan_step, step};
    let steps = [step("Triage"), plan_step(), step("Build")];
    assert_eq!(at(&steps, Facts::default()), Some(0), "a Job with no kept place is on the first step");
    let stood = Facts { stood: Some(1), visited: StepBits::of([1]), ..Facts::default() };
    assert_eq!(at(&steps, stood), Some(1), "the Job stood on Plan, and Triage was added behind it");
    let began = Facts { visited: StepBits::of([0, 1]), ..stood };
    assert_eq!(at(&steps, began), Some(0), "a step it did begin is not passed over");
}
