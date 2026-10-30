//! A repository's lifecycles and the Workspaces (sets of repositories) that scope them, read from
//! Kit's folder and each repository's own `.armada/`. The merge is `config`'s; what is proved here
//! is that the right folders are read, and that a Workspace committed to one repository is seen
//! from the others.

use std::path::{Path, PathBuf};

use config::Reach;
use core_model::{LifecycleId, ManifestId, PieceSource};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::repositories::{Located, Served};
use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

fn write(path: PathBuf, text: &str) {
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("its folder");
    std::fs::write(path, text).expect("its file");
}

fn ticket(scope: Option<&str>, word: &str) -> String {
    let scope = scope.map(|one| format!("scope: {one}\n")).unwrap_or_default();
    format!(
        "version: 1\nlifecycle_id: ticket\nname: Ticket\n{scope}states:\n  - {{ id: in_review, label: In review, read: {{ tracker: [{word}] }} }}\n  - {{ id: closed, label: Closed, read: {{ tracker: [Done] }} }}\nends: [closed]\n"
    )
}

fn my_store(holding: &[&ManifestId]) -> String {
    let listed: String = holding.iter().map(|one| format!("  - {}\n", one.as_str())).collect();
    format!("version: 1\nworkspace_id: my-store\nname: My store\nrepositories:\n{listed}")
}

fn kit(home: &TempDir, kind: &str, file: &str) -> PathBuf {
    home.path().join("kit").join(kind).join(file)
}

fn armada(root: &str, kind: &str, file: &str) -> PathBuf {
    Path::new(root).join(".armada").join(kind).join(file)
}

/// The repository the fixture starts in.
fn first(fleet: &Fixture) -> Served {
    fleet.repositories().first().expect("the fixture's repository")
}

/// A folder with no `armada.yml`, served under the Manifest id Fleet gives it.
fn another(fleet: &Fixture, home: &TempDir, name: &str) -> Served {
    let root = home.path().join(name).to_string_lossy().to_string();
    std::fs::create_dir_all(&root).expect("a folder");
    let located = Located { records_root: format!("{root}-records"), root: root.clone(), set_up: None };
    fleet.repositories().add(located).expect("served");
    Served::hosting(&fleet.repositories().at(&root).expect("held"))
}

fn id_of(served: &Served) -> ManifestId {
    served.manifest().id().clone()
}

fn ticket_words(applying: &config::Lifecycles) -> String {
    let id = LifecycleId::new("ticket").expect("an id");
    format!("{:?}", applying.get(&id).expect("a ticket").states()[0].read())
}

#[tokio::test]
async fn a_repository_with_no_piece_files_runs_exactly_what_armada_carries() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let applying = fleet.lifecycles_applying_to(&id_of(&first(&fleet))).expect("read");
    let mut ran: Vec<&str> = applying.running().map(|one| one.id().as_str()).collect();
    ran.sort_unstable();
    assert_eq!(ran, ["deploy", "job", "pr", "ticket"]);
    assert!(applying.left_out().is_empty());
    assert_eq!(fleet.workspaces_read().files().len(), 0);
}

#[tokio::test]
async fn yours_in_kit_runs_over_the_repositorys_and_both_are_listed_from_their_own_folders() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let repository = first(&fleet);
    write(kit(&home, "lifecycles", "tickets.yml"), &ticket(None, "Mine"));
    write(armada(repository.root(), "lifecycles", "tickets.yml"), &ticket(None, "Theirs"));

    let applying = fleet.lifecycles_applying_to(&id_of(&repository)).expect("read");
    assert!(ticket_words(&applying).contains("Mine"));
    let files: Vec<_> = applying.files().iter().filter(|file| file.lifecycle().id().as_str() == "ticket").collect();
    assert_eq!(
        files.iter().map(|file| file.rank().source).collect::<Vec<_>>(),
        [PieceSource::Kit, PieceSource::Repository, PieceSource::Armada]
    );
    assert_eq!(files[0].hides(), Some(PieceSource::Repository), "yours says what it hides");
    assert!(files[1].path().is_some_and(|path| path.starts_with(repository.root())));
}

#[tokio::test]
async fn a_workspace_committed_to_one_repository_scopes_a_lifecycle_in_the_others() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let (web, api, back_office) = (first(&fleet), another(&fleet, &home, "store-api"), another(&fleet, &home, "back-office"));
    // The Workspace is committed to store-web alone. Kit scopes a ticket lifecycle to it.
    write(armada(web.root(), "workspaces", "my-store.yml"), &my_store(&[&id_of(&web), &id_of(&api)]));
    write(kit(&home, "lifecycles", "tickets.yml"), &ticket(Some("my-store"), "Code Review"));

    for member in [&web, &api] {
        let applying = fleet.lifecycles_applying_to(&id_of(member)).expect("read");
        assert!(ticket_words(&applying).contains("Code Review"), "{}", member.root());
        let ticket = applying.files().iter().find(|file| file.lifecycle().id().as_str() == "ticket").expect("a ticket");
        assert_eq!(ticket.rank().reach, Reach::Workspace);
    }
    let outside = fleet.lifecycles_applying_to(&id_of(&back_office)).expect("read");
    assert!(!ticket_words(&outside).contains("Code Review"), "the Workspace does not hold it");
}

#[tokio::test]
async fn a_file_that_does_not_fit_is_named_and_the_others_stand() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    write(kit(&home, "lifecycles", "broken.yml"), "version: 1\nlifecycle_id: ticket\n");
    write(kit(&home, "workspaces", "also-broken.yml"), "not: a workspace\n");
    write(kit(&home, "lifecycles", "notes.txt"), "not a piece file at all\n");

    let applying = fleet.lifecycles_applying_to(&id_of(&first(&fleet))).expect("read");
    assert_eq!(applying.left_out().len(), 1, "{:?}", applying.left_out());
    let left = &applying.left_out()[0];
    assert!(left.path().ends_with("broken.yml") && left.source() == PieceSource::Kit);
    assert_eq!(applying.running().count(), 4);
    let workspaces = fleet.workspaces_read();
    assert!(workspaces.left_out().iter().any(|one| one.path().ends_with("also-broken.yml")));
}

#[tokio::test]
async fn a_repository_nobody_serves_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[]));
    let nobody = ManifestId::carried(core_model::Ulid::carried("01NOBODY"));
    assert!(fleet.lifecycles_applying_to(&nobody).is_err());
}
