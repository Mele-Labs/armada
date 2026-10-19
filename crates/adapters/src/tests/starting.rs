//! A folder in no repository becomes one with a first commit a worktree can
//! branch from, and a repository with no remote leases slots from it.

use std::path::Path;

use super::repo::TempRepo;
use crate::leasing::{Holder, Leased, Pool};
use crate::{inferred_base, inside_a_repository, start_repository};

fn no_seed(_: &Path, _: &Path) -> Result<(), String> {
    Err(String::from("nothing to seed from"))
}

/// A folder beside `repo`, in no repository, swept with it.
fn a_plain_folder(repo: &TempRepo) -> std::path::PathBuf {
    let folder = repo.root().with_extension("plain");
    std::fs::create_dir_all(&folder).expect("a folder");
    folder
}

#[test]
fn a_folder_in_no_repository_is_started_on_main_with_an_empty_first_commit() {
    let beside = TempRepo::empty();
    let folder = a_plain_folder(&beside);
    std::fs::write(folder.join("notes.txt"), "mine").expect("a file");
    assert!(!inside_a_repository(&folder));

    start_repository(&folder).expect("started");

    assert!(inside_a_repository(&folder));
    let repo = git2::Repository::open(&folder).expect("a repository");
    let head = repo.head().expect("a head");
    assert_eq!(head.shorthand(), Some("main"));
    let commit = head.peel_to_commit().expect("a commit");
    assert_eq!(commit.tree().expect("a tree").len(), 0, "the first commit is empty");
    assert_eq!(commit.parent_count(), 0);
    assert!(folder.join("notes.txt").exists(), "its own files stay as they were");
    assert_eq!(inferred_base(&folder).as_deref(), Some("main"));
}

#[test]
fn the_slots_directory_is_left_out_of_the_new_repositorys_status() {
    let beside = TempRepo::empty();
    let folder = a_plain_folder(&beside);
    start_repository(&folder).expect("started");
    std::fs::create_dir_all(folder.join(".armada/slots/slot-1")).expect("a slot");
    std::fs::write(folder.join(".armada/slots/slot-1/a"), "x").expect("a file");

    let repo = git2::Repository::open(&folder).expect("a repository");
    let mut options = git2::StatusOptions::new();
    options.include_untracked(true).include_ignored(false);
    let statuses = repo.statuses(Some(&mut options)).expect("a status");
    let shown: Vec<_> = statuses.iter().filter_map(|one| one.path().map(String::from)).collect();
    assert!(shown.is_empty(), "{shown:?}");
}

#[test]
fn a_folder_inside_a_repository_reads_as_inside_one_and_a_project_made_there_is_its_own() {
    let repo = TempRepo::with_a_commit();
    let inside = repo.root().join("sub");
    std::fs::create_dir_all(&inside).expect("a subfolder");
    assert!(inside_a_repository(&inside));

    start_repository(&inside).expect("a new project inside another");

    assert!(inside.join(".git").exists());
    assert_eq!(
        git2::Repository::discover(&inside).expect("a repository").workdir(),
        Some(inside.as_path().join("").as_path()),
        "the project is its own repository, not the outer one's subfolder"
    );
}

#[test]
fn a_repository_with_no_remote_leases_a_slot_from_its_first_commit() {
    let beside = TempRepo::empty();
    let folder = a_plain_folder(&beside);
    start_repository(&folder).expect("started");
    let holder = Holder::of(std::process::id()).expect("this process is running");

    let pool = Pool::at(&folder, 2, "main", Vec::new());
    let leased = pool.try_lease("sessions/first", &holder, 0, &no_seed).expect("a lease");

    let Leased::Took(lease) = leased else {
        panic!("a repository with no remote should lease a slot");
    };
    assert!(lease.path().starts_with(folder.join(".armada/slots")));
    let slot = git2::Repository::open(lease.path()).expect("the slot is a worktree");
    assert_eq!(slot.head().expect("a head").shorthand(), Some("sessions/first"));
}

#[test]
fn a_repository_on_a_branch_of_its_own_names_that_branch_as_its_base() {
    let repo = TempRepo::empty();
    git2::Repository::open(repo.root())
        .expect("a repository")
        .set_head("refs/heads/trunk")
        .expect("trunk");
    repo.commit_everything("first");
    assert_eq!(inferred_base(repo.root()).as_deref(), Some("trunk"));
}

#[test]
fn a_repository_on_master_names_master_as_its_base() {
    let repo = TempRepo::empty();
    git2::Repository::open(repo.root())
        .expect("a repository")
        .set_head("refs/heads/master")
        .expect("master");
    repo.commit_everything("first");
    assert_eq!(inferred_base(repo.root()).as_deref(), Some("master"));
}
