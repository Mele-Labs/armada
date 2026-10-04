//! A stranded slot read, scrapped and stashed, against real repositories: what
//! it holds, what a scrap refuses to lose, and what a stash keeps on the remote.

use std::path::{Path, PathBuf};

use adapter_traits::{RescueRefused, SlotRescue};

use crate::leasing::{Holder, Leased, Pool};
use crate::tests::repo::TempRepo;

fn a_repository() -> TempRepo {
    let repo = TempRepo::with_a_commit();
    repo.commit_one(".gitignore", "target/\n", "ignore builds");
    repo.with_a_bare_remote();
    repo.git(&["push", "-q", "origin", "main"]);
    repo
}

fn here() -> Holder {
    Holder::of(std::process::id()).expect("this process is running")
}

fn gone() -> Holder {
    Holder::Process {
        pid: std::process::id(),
        started: String::from("a start no process has"),
    }
}

fn no_seed(_: &Path, _: &Path) -> Result<(), String> {
    Err(String::from("nothing to seed from"))
}

fn git(at: &Path, args: &[&str]) -> String {
    let run = std::process::Command::new("git")
        .arg("-C")
        .arg(at)
        .args([
            "-c",
            "user.name=armada",
            "-c",
            "user.email=armada@example.invalid",
        ])
        .args(args)
        .output()
        .expect("git on PATH");
    assert!(
        run.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    String::from_utf8_lossy(&run.stdout).trim().to_string()
}

/// One slot, left by a holder that is gone, on branch `half-done` with one
/// commit nothing else has, a tracked file edited and a new file written.
fn a_stranded_slot(repo: &TempRepo) -> (Pool, PathBuf) {
    let pool = Pool::at(repo.root(), 1, "main", Vec::new());
    let slot = match pool.try_lease("half-done", &gone(), 0, &no_seed) {
        Ok(Leased::Took(lease)) => lease.path().to_path_buf(),
        other => panic!("no slot taken: {other:?}"),
    };
    std::fs::write(slot.join("kept.rs"), "fn kept() {}\n").unwrap();
    git(&slot, &["add", "kept.rs"]);
    git(&slot, &["commit", "-q", "-m", "Keep one"]);
    std::fs::write(slot.join("kept.rs"), "fn kept() { 1 }\n").unwrap();
    std::fs::write(slot.join("new.rs"), "fn new() {}\n").unwrap();
    (pool, slot)
}

/// Whether the slot can be leased again, which is what freed means.
fn is_free(pool: &Pool) -> bool {
    matches!(
        pool.try_lease("next", &here(), 0, &no_seed),
        Ok(Leased::Took(_))
    )
}

#[test]
fn a_stranded_slot_says_what_it_holds_and_its_change_against_the_base() {
    let repo = a_repository();
    let (pool, slot) = a_stranded_slot(&repo);

    let work = pool.stranded_work(1).expect("stranded");
    assert_eq!(work.branch.as_deref(), Some("half-done"));
    assert_eq!(work.commit, git(&slot, &["rev-parse", "HEAD"]));
    assert!(work
        .uncommitted
        .iter()
        .any(|path| path.ends_with("kept.rs")));
    assert!(work.uncommitted.iter().any(|path| path.ends_with("new.rs")));
    assert_eq!(work.commits.len(), 1);
    assert_eq!(work.commits[0].subject, "Keep one");
    assert_eq!(work.unpushed, 1);

    let diff = pool.stranded_diff(1).expect("stranded");
    assert!(diff.contains("+fn kept() { 1 }"), "{diff}");
}

#[test]
fn a_slot_that_is_not_stranded_is_neither_read_nor_acted_on() {
    let repo = a_repository();
    let pool = Pool::at(repo.root(), 1, "main", Vec::new());
    match pool.try_lease("held", &here(), 0, &no_seed) {
        Ok(Leased::Took(_)) => {}
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        pool.stranded_work(1),
        Err(RescueRefused::NotStranded(_))
    ));
    assert!(matches!(
        pool.rescue(1, SlotRescue::Scrap),
        Err(RescueRefused::NotStranded(_))
    ));
    assert!(matches!(
        pool.stranded_work(9),
        Err(RescueRefused::NoSuchSlot(9))
    ));
}

/// **A scrap throws the uncommitted files away and frees the slot, and keeps
/// the branch while it holds a commit nothing else has.**
#[test]
fn a_scrap_discards_the_files_and_keeps_a_branch_with_unpushed_commits() {
    let repo = a_repository();
    let (pool, slot) = a_stranded_slot(&repo);

    let done = pool.rescue(1, SlotRescue::Scrap).expect("scrapped");
    assert!(done.branch_kept);
    assert_eq!(done.branch.as_deref(), Some("half-done"));
    assert!(!slot.join("new.rs").exists());
    assert!(git(&slot, &["status", "--porcelain"]).is_empty());
    assert!(!git(repo.root(), &["branch", "--list", "half-done"]).is_empty());
    assert!(is_free(&pool));
}

/// A branch the base already holds goes with the scrap.
#[test]
fn a_scrap_deletes_a_branch_the_base_holds_in_full() {
    let repo = a_repository();
    let pool = Pool::at(repo.root(), 1, "main", Vec::new());
    let slot = match pool.try_lease("nothing-new", &gone(), 0, &no_seed) {
        Ok(Leased::Took(lease)) => lease.path().to_path_buf(),
        other => panic!("{other:?}"),
    };
    std::fs::write(slot.join("scratch.rs"), "x").unwrap();

    let done = pool.rescue(1, SlotRescue::Scrap).expect("scrapped");
    assert!(!done.branch_kept);
    assert!(git(repo.root(), &["branch", "--list", "nothing-new"]).is_empty());
    assert!(is_free(&pool));
}

/// **A stash commits the uncommitted files to the branch and pushes it under
/// its own name**, so the remote has everything the slot held.
#[test]
fn a_stash_commits_and_pushes_the_branch_and_frees_the_slot() {
    let repo = a_repository();
    let (pool, _slot) = a_stranded_slot(&repo);

    let done = pool
        .rescue(
            1,
            SlotRescue::Stash {
                message: String::from("Work left in slot-1, kept"),
            },
        )
        .expect("stashed");
    assert!(done.committed.is_some());
    assert!(done.branch_kept);
    let pushed = git(
        repo.root(),
        &["ls-remote", "--heads", "origin", "half-done"],
    );
    assert!(!pushed.is_empty(), "the branch is on the remote");
    let tip = git(repo.root(), &["rev-parse", "half-done"]);
    assert!(pushed.starts_with(&tip), "the remote has the stash commit");
    let files = git(
        repo.root(),
        &["show", "--name-only", "--format=", "half-done"],
    );
    assert!(
        files.contains("new.rs") && files.contains("kept.rs"),
        "{files}"
    );
    assert!(is_free(&pool));
}

#[test]
fn a_stash_with_no_remote_changes_nothing() {
    let repo = TempRepo::with_a_commit();
    let (pool, slot) = a_stranded_slot(&repo);
    assert!(matches!(
        pool.rescue(
            1,
            SlotRescue::Stash {
                message: String::from("kept")
            }
        ),
        Err(RescueRefused::NoRemote)
    ));
    assert!(slot.join("new.rs").exists());
}
