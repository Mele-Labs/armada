//! A project is a file kept where he chose for it, and what Fleet makes of what was kept before: Kit's
//! Workspace files, the ones a repository committed, and the task lists in the store. **Every step is
//! run twice**, since a start that did it again would be a start that undid his deletes.

use std::path::{Path, PathBuf};

use api::{Projects, Refusal};
use config::{ProjectPlace, Projects as Held};
use core_model::{ManifestId, ScopeId, Ulid};
use ipc::{NameProject, RunPlace};
use store::{KeptProjectRow, KeptRowItem, KeptTaskState, LOOSE_PROJECT};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::daemon::a_fleet;
use crate::tests::projects_kept::{file, text};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const OLD: &str = "version: 1\nworkspace_id: my-store\nname: My store\nrepositories:\n  - store-web\n  - store-api\n";

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("its folder");
    std::fs::write(path, text).expect("its file");
}

fn repository(id: &str) -> ManifestId {
    ManifestId::carried(Ulid::carried(id))
}

fn scope(id: &str) -> ScopeId {
    ScopeId::new(id).expect("a scope")
}

fn members_of(held: &Held, id: &str) -> Vec<String> {
    let project = held.get(&scope(id)).unwrap_or_else(|| panic!("no project {id}: {:?}", held.left_out()));
    project.members().iter().map(|one| one.repository().as_str().to_string()).collect()
}

fn task(id: &str, manifest: &str, parent: Option<&str>) -> KeptProjectRow {
    KeptProjectRow {
        id: id.to_string(),
        manifest_id: manifest.to_string(),
        item: KeptRowItem::Task {
            title: format!("task {id}"),
            notes: String::new(),
            state: KeptTaskState::Open,
            parent: parent.map(str::to_string),
            issues: Vec::new(),
            pulls: Vec::new(),
        },
    }
}

async fn row_ids(fleet: &Fixture) -> Vec<String> {
    fleet.store().lock().await.projects().expect("read").projects.into_iter().map(|one| one.id).collect()
}

#[tokio::test]
async fn a_kit_workspace_file_becomes_a_project_file_with_a_row_and_the_old_file_goes() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let old = home.path().join("kit/workspaces/my-store.yml");
    write(&old, OLD);

    let done = fleet.projects_migrated().await;

    assert_eq!((done.converted, done.imported, done.derived), (1, 0, 0), "{done:?}");
    assert!(!old.exists(), "the old Kit file is removed");
    assert!(file(&home, "my-store").is_file());
    let held = fleet.projects_held();
    assert_eq!(members_of(&held, "my-store"), ["store-web", "store-api"]);
    assert!(held.get(&scope("my-store")).expect("held").members().iter().all(|one| one.folder_of().is_none()));
    assert_eq!(row_ids(&fleet).await, ["my-store"], "a row at the end of his order");

    let again = fleet.projects_migrated().await;
    assert_eq!((again.converted, again.imported, again.derived), (0, 0, 0), "a second start changes nothing: {again:?}");
    assert_eq!(row_ids(&fleet).await, ["my-store"]);
}

#[tokio::test]
async fn a_workspace_a_repository_committed_is_imported_once_left_in_place_and_not_again_after_a_delete() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let root = fleet.repositories().first().expect("served").root().to_string();
    let committed = PathBuf::from(&root).join(".armada/workspaces/my-store.yml");
    write(&committed, OLD);

    let done = fleet.projects_migrated().await;
    assert_eq!((done.converted, done.imported), (0, 1), "{done:?}");
    assert_eq!(std::fs::read_to_string(&committed).expect("still there"), OLD, "the repository's file is never edited");
    assert_eq!(members_of(&fleet.projects_held(), "my-store"), ["store-web", "store-api"]);
    assert!(file(&home, "my-store").is_file(), "kept on this Mac, not in the repository");

    fleet.delete_project("my-store".to_string()).await.expect("deleted");
    assert!(!file(&home, "my-store").exists());

    let again = fleet.projects_migrated().await;
    assert_eq!((again.imported, again.converted, again.derived), (0, 0, 0), "{again:?}");
    assert!(fleet.projects_held().get(&scope("my-store")).is_none(), "the project he deleted is not brought back");
    assert!(row_ids(&fleet).await.is_empty());
    assert!(committed.exists());
}

#[tokio::test]
async fn a_committed_workspace_whose_id_is_already_a_project_is_left_alone() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let root = fleet.repositories().first().expect("served").root().to_string();
    write(&file(&home, "my-store"), &text("my-store", "Mine", &["store-web"]));
    write(&PathBuf::from(&root).join(".armada/workspaces/my-store.yml"), OLD);

    let done = fleet.projects_migrated().await;

    assert_eq!(done.imported, 0, "{done:?}");
    assert_eq!(members_of(&fleet.projects_held(), "my-store"), ["store-web"], "what he kept stands");
}

#[tokio::test]
async fn a_task_list_becomes_a_project_whose_members_are_the_repositories_its_rows_name_and_loose_tasks_stay_loose() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    {
        let mut store = fleet.store().lock().await;
        store.add_project("shop", "Shop", "2026-11-10T00:00:00Z").expect("added");
        store.add_project("empty", "Nothing yet", "2026-11-10T00:00:00Z").expect("added");
        store.add_project_row("shop", &task("t1", "web", None)).expect("row");
        store.add_project_row("shop", &task("t2", "api", None)).expect("row");
        store.add_project_row("shop", &task("t3", "web", None)).expect("row");
        // A subtask takes its parent's repository, whatever its own row says.
        store.add_project_row("shop", &task("t4", "ios", Some("t2"))).expect("row");
        store.add_project_row(LOOSE_PROJECT, &task("loose", "web", None)).expect("row");
    }

    let done = fleet.projects_migrated().await;

    assert_eq!((done.derived, done.converted, done.imported), (2, 0, 0), "{done:?}");
    let held = fleet.projects_held();
    assert_eq!(members_of(&held, "shop"), ["web", "api"], "each once, in the order first seen");
    assert!(members_of(&held, "empty").is_empty(), "a project with no rows holds nothing");
    assert_eq!(held.get(&scope("shop")).expect("held").name().as_str(), "Shop");
    let kept = fleet.store().lock().await.projects().expect("read");
    assert_eq!(kept.loose.len(), 1, "loose tasks are untouched");
    assert_eq!(kept.projects.iter().map(|one| one.rows.len()).collect::<Vec<_>>(), [4, 0], "and so are the rows of a project");

    let again = fleet.projects_migrated().await;
    assert_eq!((again.derived, again.converted, again.imported), (0, 0, 0), "{again:?}");
}

#[tokio::test]
async fn a_new_project_is_empty_on_this_mac_and_renaming_it_writes_the_file_and_the_row() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));

    let made = fleet.add_project(NameProject { name: "Shop".to_string() }).await.expect("added");
    let id = made.projects[0].id.clone();
    assert!(file(&home, &id).is_file(), "kept on this Mac by default");
    assert!(fleet.projects_held().get(&scope(&id)).expect("held").members().is_empty(), "made empty");

    let renamed = fleet.rename_project(id.clone(), NameProject { name: "Storefront".to_string() }).await.expect("renamed");
    assert_eq!(renamed.projects[0].name, "Storefront");
    assert!(std::fs::read_to_string(file(&home, &id)).expect("a file").contains("name: Storefront"), "the file is the truth");
    assert_eq!(fleet.store().lock().await.projects().expect("read").projects[0].name, "Storefront", "and the store keeps the last name");
}

#[tokio::test]
async fn the_name_in_the_file_is_shown_and_the_stores_only_where_the_place_cannot_be_read() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let id = fleet.add_project(NameProject { name: "Shop".to_string() }).await.expect("added").projects[0].id.clone();

    // Another machine renamed it in the file.
    write(&file(&home, &id), &text(&id, "Storefront", &[]));
    let seen = fleet.list_projects().await.expect("listed");
    assert_eq!(seen.projects[0].name, "Shop", "held until Fleet reads again");
    fleet.projects_reload().await;
    assert_eq!(fleet.list_projects().await.expect("listed").projects[0].name, "Storefront");

    // Its file cannot be read: it is left out with the reason, Fleet goes on, and the store's name shows.
    std::fs::remove_file(file(&home, &id)).expect("removed");
    let held = fleet.projects_reload().await;
    assert!(held.get(&scope(&id)).is_none());
    assert!(held.left_out().iter().any(|one| one.said().contains("not there")), "{:?}", held.left_out());
    assert_eq!(fleet.list_projects().await.expect("listed").projects[0].name, "Storefront", "the last name the store saw");
}

#[tokio::test]
async fn a_project_cannot_be_kept_in_a_repository() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let id = fleet.add_project(NameProject { name: "Shop".to_string() }).await.expect("added").projects[0].id.clone();

    let refused = fleet.project_place_set(&id, RunPlace::Repository, None, None).await.expect_err("refused");
    match refused {
        Refusal::Unacceptable(error) => {
            assert_eq!(error.code, "fleet.project_place_refused");
            assert!(error.message.contains("not kept in a repository"), "{}", error.message);
        }
        other => panic!("not a 422: {other:?}"),
    }
    assert!(file(&home, &id).is_file(), "nothing moved");
}

#[tokio::test]
async fn a_project_moved_to_a_synced_folder_is_written_and_read_there_and_a_folder_that_goes_leaves_it_out() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let id = fleet.add_project(NameProject { name: "Shop".to_string() }).await.expect("added").projects[0].id.clone();
    let synced = home.path().join("synced");
    std::fs::create_dir_all(&synced).expect("a folder");

    fleet.project_place_set(&id, RunPlace::Synced, Some(synced.to_string_lossy().to_string()), None).await.expect("moved");

    let there = synced.join("projects").join(format!("{id}.yml"));
    assert!(there.is_file(), "written in the synced folder");
    assert!(!file(&home, &id).exists(), "and taken from Kit, where Fleet kept it");
    let held = fleet.projects_held();
    assert_eq!(held.file(&scope(&id)).map(|one| one.place()), Some(ProjectPlace::Synced));

    std::fs::remove_dir_all(&synced).expect("the folder went");
    let held = fleet.projects_reload().await;
    assert!(held.get(&scope(&id)).is_none(), "left out, and Fleet runs on");
    assert!(held.left_out().iter().any(|one| one.place() == ProjectPlace::Synced && !one.said().is_empty()), "{:?}", held.left_out());
    assert!(fleet.list_projects().await.is_ok());
}

#[tokio::test]
async fn a_member_may_be_a_folder_of_a_repository_and_the_project_still_holds_the_repository() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    write(
        &file(&home, "shop"),
        "version: 2\nproject_id: shop\nname: Shop\nmembers:\n  - repository: mono\n    folder: apps/api\n",
    );
    let held = fleet.projects_reload().await;
    let project = held.get(&scope("shop")).expect("held");
    assert!(matches!(project.members(), [one] if one.folder_of().is_some_and(|folder| folder.as_str() == "apps/api")));
    assert_eq!(held.holding(&repository("mono")).len(), 1);
}
