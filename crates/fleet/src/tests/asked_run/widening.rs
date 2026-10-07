//! A Drone's asked run and the gate agree on which Checks exist: a manifest
//! that gates the change and that the Job froze nothing for is run here too.
//! `docs/concepts/manifest.md`, *Workspace gating*.

use std::path::Path;
use std::sync::Arc;

use config::{Manifest, ResolvedWorkflow, Roster, WorkflowDef};

use crate::tests::asked_run::{a_fleet_over, Held};
use crate::tests::daemon::{a_proposal, worktree_directory};
use crate::tests::tmp::TempDir;
use crate::tests::tools::checked_by_the_one;

const STEP: &str =
    "version: 1\nworkflow_id: fixture-workflow\nname: fixture\nsteps:\n  - id: implement\n    \
    label: Implement\n    evidence: {submitted: {type: diff}}\n    delivers: false\n    \
    advance_gate: auto\n    mechanical_checks: [{ type: diff_nonempty }, \
    { type: every_manifest_check }]\n";

#[tokio::test]
async fn an_asked_run_runs_the_checks_of_a_gating_manifest_the_job_froze_none_for() {
    let home = TempDir::new();
    let marker = home.path().join("a-ran");
    let def = WorkflowDef::parse(Path::new("fixture.yml"), STEP, &Roster::offering_nothing())
        .expect("the workflow parses");
    let root = Manifest::parse(
        Path::new("armada.yml"),
        "version: 1\nid: 01FIXTUREMANIFEST\n",
    )
    .expect("the root parses");
    // Frozen before the workspace had a manifest.
    let frozen = ResolvedWorkflow::resolve(&def, &root).expect("resolves");
    let fleet = Arc::new(a_fleet_over(
        &home,
        frozen,
        Arc::new(Held::started()),
        3,
        &["a/src/x.ts"],
    ));
    let job = fleet
        .propose(a_proposal("change a"))
        .await
        .expect("a proposed Job");
    worktree_directory(&home, &job);
    let served = fleet
        .repositories()
        .first()
        .expect("the fixture repository");
    let a = Manifest::parse_workspace(
        Path::new("a/armada.yml"),
        "a",
        &format!(
            "version: 1\nid: 01ASKEDA\nchecks:\n  test:\n    run: \"/usr/bin/touch {}\"\n",
            marker.display()
        ),
        served.manifest(),
    )
    .expect("a parses");
    served.workspaces_read(vec![a]);
    crate::tests::admitted::dispatched(&fleet, job.id())
        .await
        .expect("an approved Job");
    let leased = crate::tests::daemon::spec_held(&home, &job).expect("where the Job works");
    std::fs::create_dir_all(Path::new(&leased.worktree_path()).join("a")).expect("a's directory");

    let report = checked_by_the_one(&fleet).await.expect("a report");

    let named: Vec<&str> = report.ran.iter().map(|ran| ran.name.as_str()).collect();
    assert!(
        named.iter().any(|name| name.ends_with("test")),
        "the run left a's Check out: {named:?}"
    );
    assert!(
        marker.exists(),
        "a Check did not run: {:?}",
        report
            .ran
            .iter()
            .map(|r| (&r.name, &r.outcome, &r.detail))
            .collect::<Vec<_>>()
    );
}
