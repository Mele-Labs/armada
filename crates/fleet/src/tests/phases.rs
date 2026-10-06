//! A step's phase, and the two grants a canvas draws beside it, as Fleet
//! serves them before a dispatch and once a Job holds them. Since 23.19.

use api::Queries;
use testkit::FakeWorkProduct;

use crate::tests::daemon::{a_proposal_for, fittings, manifest, one};
use crate::tests::tmp::TempDir;

/// Three steps: one declaring `setup`, one declaring nothing, and the one that
/// delivers, declaring nothing either.
fn three_phases() -> config::ResolvedWorkflow {
    let def = config::WorkflowDef::parse(
        std::path::Path::new("fixture.yml"),
        "version: 1\nworkflow_id: fixture-phases\nname: fixture\n\
         steps:\n  - id: prepare\n    label: \"Prepare\"\n    phase: setup\n    \
         delivers: false\n    advance_gate: auto\n  - id: implement\n    label: \"Implement\"\n    \
         evidence: {submitted: {type: diff}}\n    mechanical_checks:\n      - type: diff_nonempty\n    \
         drone_per_task: true\n    delivers: false\n    advance_gate: auto\n  - id: handoff\n    \
         label: \"Hand off\"\n    delivers: true\n    advance_gate: human_always\n",
        &config::Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture did not parse: {refused}"));
    config::ResolvedWorkflow::resolve(&def, &manifest())
        .unwrap_or_else(|refused| panic!("the fixture did not resolve: {refused}"))
}

fn phases_of<'a>(
    steps: impl Iterator<Item = (&'a str, Option<ipc::StepPhase>)>,
) -> Vec<(String, Option<ipc::StepPhase>)> {
    steps.map(|(id, phase)| (id.to_string(), phase)).collect()
}

/// **The declared phase where there is one, and `delivers` where there is
/// not**, the same on the list a person picks from and on the Job they approve.
#[tokio::test]
async fn each_step_is_served_in_its_phase_before_and_after_the_job_holds_it() {
    let home = TempDir::new();
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.starting().workflows = one(three_phases());
    let fleet = crate::daemon::Fleet::assembled(fittings);
    use ipc::StepPhase::{Delivery, Setup, Work};
    let expected = vec![
        ("prepare".to_string(), Some(Setup)),
        ("implement".to_string(), Some(Work)),
        ("handoff".to_string(), Some(Delivery)),
    ];

    let workflows = fleet.list_workflows().await.expect("the workflow list");
    let steps = &workflows[0].steps;
    assert_eq!(
        phases_of(
            steps
                .iter()
                .map(|step| (step.step_id.as_str(), Some(step.phase)))
        ),
        expected
    );
    let per_task: Vec<bool> = steps.iter().map(|step| step.drone_per_task).collect();
    assert_eq!(per_task, [false, true, false]);
    assert!(steps.iter().all(|step| !step.may_dispatch_jobs));

    let job = fleet
        .propose(a_proposal_for("fix the off-by-one", "fixture-phases"))
        .await
        .expect("a Job at the approval gate");
    let detail = fleet
        .get_job(ipc::JobId::from(job.id()))
        .await
        .expect("the Job is served");
    assert_eq!(
        phases_of(
            detail
                .steps
                .iter()
                .map(|step| (step.step_id.as_str(), step.phase))
        ),
        expected
    );
}
