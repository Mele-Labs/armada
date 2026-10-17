//! `armada clean`, against a real repository.
//!
//! **Real git, because the thing under test is git's own opinion** — whether a
//! base checkout is registered, whether a registration outlived a directory. A
//! fake would be asserting this crate's guess.

use std::path::Path;
use std::process::Command;

use adapter_traits::{BaseSpec, Vcs};
use adapters::GitVcs;

use crate::clean::{clean, CleanRefused, FileGone, Scope};
use crate::serve::STORE_FILE;
use crate::tests::TempDir;

pub(super) const MANIFEST_ID: &str = "a-test-project";

/// A git repository with one commit and an `armada.yml` naming [`MANIFEST_ID`].
pub(super) fn a_repository() -> TempDir {
    let dir = TempDir::new();
    dir.write(
        "armada.yml",
        &format!("version: 1\nid: {MANIFEST_ID}\nchecks:\n  build:\n    run: /bin/sh -c true\n"),
    );
    // `main` by name, not by whoever's `init.defaultBranch` is set.
    git(
        dir.path(),
        &["-c", "init.defaultBranch=main", "init", "--quiet"],
    );
    git(dir.path(), &["add", "."]);
    git(
        dir.path(),
        &[
            "-c",
            "user.name=armada",
            "-c",
            "user.email=armada@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "the first commit",
        ],
    );
    dir
}

pub(super) fn git(at: &Path, args: &[&str]) -> String {
    let run = Command::new("git")
        .arg("-C")
        .arg(at)
        .args(args)
        .output()
        .expect("git on PATH — a test nothing can run is a test that does not exist");
    assert!(run.status.success(), "git {args:?} failed");
    String::from_utf8_lossy(&run.stdout).into_owned()
}

/// The repository's head commit, with a base checkout of it on disk.
fn a_base_checkout(repo: &Path) -> String {
    let commit = git(repo, &["rev-parse", "HEAD"]).trim().to_string();
    let spec = BaseSpec::at(&repo.to_string_lossy(), &commit).expect("a legal spec");
    GitVcs::new().base_checkout(&spec).expect("a base checkout");
    commit
}

#[test]
fn every_base_checkout_is_given_back() {
    let repo = a_repository();
    let machine = TempDir::new();
    let commit = a_base_checkout(repo.path());
    let at = repo.path().join(".armada").join("bases").join(&commit);
    assert!(at.exists());

    let cleaned = clean(repo.path(), machine.path(), Scope::Repository).expect("a clean");

    assert!(cleaned.faults.is_empty(), "{:?}", cleaned.faults);
    assert_eq!(cleaned.bases, vec![commit]);
    assert!(!at.exists());
    assert!(!cleaned.touched_nothing());
}

/// A directory under `.armada/worktrees/` is evidence, not litter.
#[test]
fn a_worktree_directory_is_reported_and_left_alone() {
    let repo = a_repository();
    let machine = TempDir::new();
    let at = repo.path().join(".armada/worktrees/by-hand");
    std::fs::create_dir_all(&at).expect("a directory");

    let cleaned = clean(repo.path(), machine.path(), Scope::Repository).expect("a clean");

    assert_eq!(cleaned.unclaimed.len(), 1);
    assert!(cleaned.unclaimed[0].ends_with("by-hand"));
    assert!(at.exists());
}

#[test]
fn all_takes_the_store_and_the_files_beside_it() {
    let repo = a_repository();
    let machine = TempDir::new();
    machine.write(STORE_FILE, "");
    machine.write("mcp.json", "{}");

    clean(repo.path(), machine.path(), Scope::AndTheMachine).expect("a clean");

    assert!(!machine.path().join(STORE_FILE).exists());
    assert!(!machine.path().join("mcp.json").exists());
}

/// Absence is an answer. A file that was not there reads differently from one
/// that was taken, and both are printed.
#[test]
fn a_machine_file_that_was_never_there_is_said_rather_than_skipped() {
    let repo = a_repository();
    let machine = TempDir::new();

    let cleaned = clean(repo.path(), machine.path(), Scope::AndTheMachine).expect("a clean");

    assert!(cleaned
        .machine
        .iter()
        .all(|file| matches!(file, FileGone::Absent(_))));
    assert!(cleaned.touched_nothing());
}

/// **The refusal `--all` exists for**, and bare `clean` takes it too: both
/// would forget records a live Fleet is holding in memory.
#[test]
fn a_clean_is_refused_while_a_fleet_holds_the_store() {
    let repo = a_repository();
    let machine = TempDir::new();
    pretend_a_fleet_is_running(machine.path());

    for scope in [Scope::Repository, Scope::AndTheMachine] {
        let refused = clean(repo.path(), machine.path(), scope)
            .expect_err("a live Fleet is holding the store");
        let CleanRefused::FleetIsRunning { pid, .. } = refused else {
            panic!("the refusal names the Fleet: {refused}");
        };
        assert_eq!(pid, std::process::id(), "and names it by pid");
    }
}

/// A runtime file naming this very process, which is a pid that is certainly
/// held by the process that wrote it.
fn pretend_a_fleet_is_running(machine: &Path) {
    let pid = std::process::id();
    let fleet::Holder::Held(started_at) = fleet::holder_of(pid).expect("this process is visible")
    else {
        panic!("a running process holds its own pid");
    };
    let file = fleet::runtime::RuntimeFile {
        protocol_id: ipc::ProtocolId::current(),
        pid,
        port: 12345,
        started_at,
    };
    std::fs::write(
        machine.join(fleet::runtime::FILE_NAME),
        ipc::encode(&file).expect("four scalars serialise"),
    )
    .expect("the runtime file");
}

#[test]
fn a_directory_that_is_not_a_repository_is_refused_by_naming_the_manifest() {
    let nowhere = TempDir::new();
    let machine = TempDir::new();

    let refused = clean(nowhere.path(), machine.path(), Scope::Repository)
        .expect_err("there is no Manifest here")
        .to_string();

    assert!(refused.contains("armada.yml"), "{refused}");
}
