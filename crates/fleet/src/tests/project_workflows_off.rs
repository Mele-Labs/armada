//! The claim, written before the code: **under a project, a workflow's switch turns it off for that project's Jobs
//! only, and the same call turns it back on. Off wins: what the machine or the repository switched off stays off in
//! the project, and each switch names the layer that holds it off.** `docs/practices/protocol/projects.md`, *Values*.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use api::{Projects, Settings};
use ipc::{ManifestId, NewProject, ProjectMember, ResolvedWorkflow, SetProjectWorkflow, ValueLayer};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use super::daemon::{fittings, workflow_named};
use super::main_ci::MANIFEST;
use super::projects::code_of;
use super::projects_kept::file;
use super::session_host::{Shared, StandIn};
use super::tmp::TempDir;
use super::workflow_off::off;
use crate::daemon::Fleet;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// A Fleet serving one repository that holds `bug`, `chore` and `feature`, and a project of it.
async fn a_rig() -> (Arc<Hosted>, TempDir, String, String) {
    let home = TempDir::new();
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.starting().manifest = config::Manifest::parse(Path::new("armada.yml"), MANIFEST).expect("parses");
    fittings.starting().workflows = ["bug", "chore", "feature"].into_iter().map(workflow_named).map(|one| (one.id().clone(), one)).collect();
    let fleet = Arc::new(Fleet::assembled(fittings).hosting_sessions_on(Arc::new(Shared(Arc::new(StandIn::default()))), Duration::from_secs(600)));
    let manifest = fleet.repositories().first().expect("served").manifest().id().as_str().to_string();
    let members = Some(vec![ProjectMember { manifest_id: ManifestId::carried(&manifest), folder: None }]);
    let new = NewProject { name: "Shop".to_string(), members, place: None, values: None };
    let project = fleet.add_project(new).await.expect("added").projects.last().expect("the project").id.clone();
    (fleet, home, manifest, project)
}

fn switched(workflow: &str, on: bool) -> SetProjectWorkflow {
    SetProjectWorkflow { workflow: workflow.to_string(), on }
}

async fn switches(fleet: &Hosted, project: &str, manifest: Option<&str>) -> Vec<(String, bool, ValueLayer)> {
    let answered = fleet.get_values(Some(project.to_string()), manifest.map(ManifestId::carried)).await.expect("read");
    answered.workflows.into_iter().map(|ResolvedWorkflow { id, on, from }| (id, on, from)).collect()
}

fn row(id: &str, on: bool, from: ValueLayer) -> (String, bool, ValueLayer) {
    (id.to_string(), on, from)
}

#[tokio::test]
async fn a_projects_switch_turns_a_workflow_off_for_its_jobs_only_and_back_on() {
    let (fleet, home, manifest, project) = a_rig().await;
    let on = |project: Option<&str>, workflow: &str| fleet.workflow_on(&manifest, project, workflow);

    let listed = fleet.set_project_workflow(project.clone(), switched("chore", false)).await.expect("switched off");
    let values = listed.projects.iter().find(|one| one.id == project).and_then(|one| one.values.clone()).expect("its values");
    assert_eq!(values.workflows_off, ["chore"], "listed with the project's values");
    assert!(std::fs::read_to_string(file(&home, &project)).expect("its file").contains("workflows_off"), "kept in its file");

    assert!(!on(Some(&project), "chore"), "off for a Job in the project");
    assert!(on(None, "chore"), "and on for a Job in no project");
    let offered = fleet.workflows_offered(&manifest, Some(&project), &fleet.repositories().first().expect("served").workflows());
    assert!(!offered.keys().any(|id| id.as_str() == "chore") && offered.len() == 2, "the proposer and the picker are not shown it");
    assert_eq!(
        switches(&fleet, &project, Some(&manifest)).await,
        [row("bug", true, ValueLayer::Default), row("chore", false, ValueLayer::Project), row("feature", true, ValueLayer::Default)],
    );

    fleet.set_project_workflow(project.clone(), switched("chore", true)).await.expect("switched on");
    assert!(on(Some(&project), "chore"), "the same call turns it back on");
    assert!(!std::fs::read_to_string(file(&home, &project)).expect("its file").contains("workflows_off"), "and the last one leaves no list");
}

#[tokio::test]
async fn off_on_the_machine_or_in_the_repository_holds_over_the_project_and_names_its_layer() {
    let (fleet, _home, manifest, project) = a_rig().await;
    fleet.set_project_workflow(project.clone(), switched("bug", false)).await.expect("off");
    fleet.set_project_workflow(project.clone(), switched("chore", false)).await.expect("off");
    fleet.save_settings(off(&["bug"], &[(manifest.as_str(), &["feature"])])).await.expect("saved");

    assert_eq!(
        switches(&fleet, &project, Some(&manifest)).await,
        [row("bug", false, ValueLayer::Machine), row("chore", false, ValueLayer::Project), row("feature", false, ValueLayer::Repository)],
    );
    assert_eq!(
        switches(&fleet, &project, None).await,
        [row("bug", false, ValueLayer::Machine), row("chore", false, ValueLayer::Project), row("feature", true, ValueLayer::Default)],
        "no repository asked: the machine's and the project's",
    );

    fleet.set_project_workflow(project.clone(), switched("bug", true)).await.expect("on in the project");
    assert!(!fleet.workflow_on(&manifest, Some(&project), "bug"), "off wins: the machine's still holds it");
}

#[tokio::test]
async fn a_blank_workflow_or_a_project_he_has_none_of_is_refused() {
    let (fleet, _home, _manifest, project) = a_rig().await;
    let blank = fleet.set_project_workflow(project, switched("  ", false)).await.expect_err("blank");
    assert_eq!(code_of(blank), (422, "fleet.project_blank".to_string()));
    let none = fleet.set_project_workflow("nope".to_string(), switched("bug", false)).await.expect_err("none");
    assert_eq!(code_of(none).0, 404);
}

#[tokio::test]
async fn kit_takes_no_workflows_off() {
    let (fleet, _home, _manifest, _project) = a_rig().await;
    let save = ipc::SaveKitValues { values: ipc::ValuesSet { workflows_off: vec!["bug".into()], ..ipc::ValuesSet::default() } };
    let refused = fleet.save_kit_values(save).await.expect_err("refused");
    assert_eq!(code_of(refused), (422, "fleet.project_values_refused".to_string()));
}
