//! Armada's own project, which Fleet makes by itself for the armada repository, once per Mac
//! (`docs/concepts/project.md`, *The built-in project*).

use api::Projects;
use config::Manifest;
use core_model::DeliverySetting;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::daemon::fittings;
use crate::tests::projects_kept::{file, planted};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// A Fleet serving the repository whose manifest id is `armada`.
fn serving_armada(home: &TempDir) -> Fixture {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    fittings.starting().manifest = Manifest::parse(std::path::Path::new("armada.yml"), "version: 1\nid: armada\n").expect("a manifest");
    Fleet::assembled(fittings)
}

fn names(fleet: &Fixture) -> Vec<String> {
    fleet.projects_held().running().map(|one| one.name().as_str().to_string()).collect()
}

#[tokio::test]
async fn a_mac_serving_armada_with_no_project_gets_one_with_auto_land_and_only_once() {
    let home = TempDir::new();
    let fleet = serving_armada(&home);

    let done = fleet.projects_migrated().await;

    assert!(done.left.is_empty(), "{done:?}");
    let held = fleet.projects_held();
    let made: Vec<_> = held.running().collect();
    assert_eq!(made.len(), 1, "one project: {:?}", names(&fleet));
    let project = made[0];
    assert_eq!(project.name().as_str(), "Armada");
    assert_eq!(project.members().iter().map(|one| one.repository().as_str()).collect::<Vec<_>>(), ["armada"]);
    assert!(project.members().iter().all(|one| one.folder_of().is_none()), "the whole repository");
    assert_eq!(project.values().delivery, Some(DeliverySetting::AutoLand));
    assert!(file(&home, project.id().as_str()).is_file(), "kept on This Mac");
    let id = project.id().as_str().to_string();
    let kept = fleet.store().lock().await.projects().expect("read");
    assert_eq!(kept.projects.iter().map(|one| one.id.clone()).collect::<Vec<_>>(), [id.clone()], "a row in his order");

    fleet.projects_migrated().await;
    assert_eq!(fleet.projects_held().running().map(|one| one.id().as_str().to_string()).collect::<Vec<_>>(), [id], "a second start makes nothing");
}

#[tokio::test]
async fn a_project_he_deleted_is_not_made_again() {
    let home = TempDir::new();
    let fleet = serving_armada(&home);
    fleet.projects_migrated().await;
    let id = fleet.projects_held().running().next().expect("made").id().as_str().to_string();

    fleet.delete_project(id.clone()).await.expect("deleted");
    assert!(names(&fleet).is_empty());
    fleet.projects_migrated().await;

    assert!(names(&fleet).is_empty(), "not brought back");
    assert!(!file(&home, &id).exists());
    assert!(fleet.store().lock().await.projects().expect("read").projects.is_empty());
}

#[tokio::test]
async fn a_project_that_already_holds_armada_means_nothing_is_made_and_it_is_not_made_after_a_delete() {
    let home = TempDir::new();
    let fleet = serving_armada(&home);
    planted(&fleet, &home, "mine", "Mine", &["armada"]).await;
    // A row in his order, as a project he made has, so the delete below finds it.
    fleet.store().lock().await.add_project("mine", "Mine", "2026-11-13T00:00:00Z").expect("a row");

    fleet.projects_migrated().await;

    assert_eq!(names(&fleet), ["Mine"], "what he kept stands");
    fleet.delete_project("mine".to_string()).await.expect("deleted");
    fleet.projects_migrated().await;
    assert!(names(&fleet).is_empty(), "recorded as made, so not made after his delete");
}
