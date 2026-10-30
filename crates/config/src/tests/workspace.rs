//! A Workspace piece file: what it reads to, what it is refused for, and which of two with one id
//! runs. Workspace here means a set of repositories, and nothing about a monorepo.

use std::path::{Path, PathBuf};

use core_model::{ManifestId, PieceSource, Ulid};

use crate::piece::Written;
use crate::{parse_workspace, Workspaces};

fn text(id: &str, name: &str, repositories: &[&str]) -> String {
    let listed: String = repositories.iter().map(|one| format!("  - {one}\n")).collect();
    format!("version: 1\nworkspace_id: {id}\nname: {name}\nrepositories:\n{listed}")
}

fn repository(id: &str) -> ManifestId {
    ManifestId::carried(Ulid::carried(id))
}

fn kit(file: &str, text: String) -> Written {
    Written::in_kit(PathBuf::from("kit/workspaces").join(file), text)
}

fn committed(to: &str, file: &str, text: String) -> Written {
    Written::in_repository(PathBuf::from(to).join(".armada/workspaces").join(file), text)
}

#[test]
fn a_file_reads_to_the_workspace_it_names_and_the_repositories_it_lists() {
    let read = parse_workspace(Path::new("my-store.yml"), &text("my-store", "My store", &["store-web", "store-api"]))
        .expect("a Workspace that loads");
    assert_eq!(read.id().as_str(), "my-store");
    assert_eq!(read.title().as_str(), "My store");
    assert_eq!(read.repositories(), [repository("store-web"), repository("store-api")]);
}

#[test]
fn a_file_is_refused_for_every_key_that_is_wrong_and_names_each() {
    let said = parse_workspace(
        Path::new("my-store.yml"),
        "version: 2\nworkspace_id: my-store\nrepositories: []\nmonorepo: true\n",
    )
    .expect_err("a file that does not load")
    .to_string();
    for fault in ["version", "name", "repositories", "monorepo"] {
        assert!(said.contains(fault), "{fault} is named in: {said}");
    }
}

#[test]
fn a_workspace_that_is_wrong_as_a_whole_is_refused_once_its_keys_are_right() {
    let said = parse_workspace(Path::new("my-store.yml"), &text("my-store", "My store", &["store-web", "store-web"]))
        .expect_err("a repository listed twice")
        .to_string();
    assert!(said.contains("my-store.yml") && said.contains("`store-web` is listed twice"), "{said}");
}

#[test]
fn yours_runs_over_the_repositorys_and_the_one_it_hides_is_still_listed() {
    let workspaces = Workspaces::of([
        committed("store-web", "my-store.yml", text("my-store", "My store", &["store-web", "store-api"])),
        kit("my-store.yml", text("my-store", "My store, mine", &["store-web", "store-api", "back-office"])),
    ]);
    let ran: Vec<_> = workspaces.running().map(|one| one.title().as_str()).collect();
    assert_eq!(ran, ["My store, mine"]);
    let [yours, theirs] = workspaces.files() else { panic!("both are listed: {:?}", workspaces.files()) };
    assert_eq!((yours.source(), yours.overridden_by(), yours.hides()), (PieceSource::Kit, None, Some(PieceSource::Repository)));
    assert_eq!((theirs.source(), theirs.overridden_by(), theirs.hides()), (PieceSource::Repository, Some(PieceSource::Kit), None));
}

#[test]
fn two_copies_that_are_the_same_carry_no_mark() {
    let same = || text("my-store", "My store", &["store-web", "store-api"]);
    let workspaces = Workspaces::of([committed("store-web", "my-store.yml", same()), kit("my-store.yml", same())]);
    assert_eq!(workspaces.files().len(), 2);
    assert!(workspaces.files().iter().all(|file| file.hides().is_none()));
}

#[test]
fn a_workspace_committed_to_one_repository_is_read_from_there_by_the_others() {
    let kept_in_web = committed("store-web", "my-store.yml", text("my-store", "My store", &["store-web", "store-api"]));
    let workspaces = Workspaces::of([kept_in_web]);
    for member in ["store-web", "store-api"] {
        let held: Vec<_> = workspaces.holding(&repository(member)).iter().map(|one| one.id().as_str()).collect();
        assert_eq!(held, ["my-store"], "{member}");
    }
    assert!(workspaces.holding(&repository("back-office")).is_empty());
}

#[test]
fn one_id_committed_to_two_repositories_is_a_fault_and_names_both() {
    let workspaces = Workspaces::of([
        committed("store-web", "my-store.yml", text("my-store", "My store", &["store-web", "store-api"])),
        committed("store-api", "my-store.yml", text("my-store", "My store", &["store-web", "store-api"])),
    ]);
    assert_eq!(workspaces.running().count(), 0, "neither is guessed to be the one");
    let [left] = workspaces.left_out() else { panic!("one entry: {:?}", workspaces.left_out()) };
    assert!(left.said().contains("store-web/.armada/workspaces/my-store.yml"), "{}", left.said());
    assert!(left.said().contains("store-api/.armada/workspaces/my-store.yml"), "{}", left.said());
}

#[test]
fn a_file_that_does_not_fit_is_left_out_with_the_reason_and_the_others_stand() {
    let workspaces = Workspaces::of([
        kit("broken.yml", String::from("version: 1\nworkspace_id: broken\n")),
        kit("my-store.yml", text("my-store", "My store", &["store-web"])),
    ]);
    assert_eq!(workspaces.running().count(), 1);
    let [left] = workspaces.left_out() else { panic!("one entry") };
    assert!(left.path().ends_with("broken.yml") && left.source() == PieceSource::Kit, "{left:?}");
    assert!(left.said().contains("name"), "{}", left.said());
}
