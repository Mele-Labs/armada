//! A Job's branch is caught up to the base the remote holds, not the local
//! branch of the same name.
//!
//! **The shape that broke Job 14 (12 Oct 2026).** The owner's `main` is checked
//! out in his own checkout and is rarely fast-forwarded, so it sits well behind
//! `origin/main`. Every catch-up merged that stale tip, reported the branch up
//! to date, and left the pull request conflicting on the forge.
//!
//! Every case here starts from the same repository: `origin` holds a commit the
//! local `main` and its tracking ref have not got.

use std::path::PathBuf;

use adapter_traits::{
    Base, BaseFetched, BroughtUpToDate, CommitTime, Delivery, KeptCurrent, Standing, Vcs,
    Worktree, WorktreeSpec,
};

use crate::tests::repo::TempRepo;
use crate::worktree::GitVcs;

const JOB: &str = "01K9REMOTEBASE000000000001";

/// A repository whose local `main` is behind the remote's, and a Job's branch
/// cut from where `main` was, pushed.
struct Stale {
    repo: TempRepo,
    bare: PathBuf,
    worktree: Worktree,
    /// Where the local `main` still is.
    old: String,
    /// Where `origin` has `main`, by a commit the local repository never made.
    new: String,
}

fn stale() -> Stale {
    let repo = TempRepo::with_a_commit();
    let bare = repo.with_a_bare_remote();
    repo.git(&["push", "--set-upstream", "origin", "main"]);
    let old = repo.git(&["rev-parse", "main"]);

    let spec = WorktreeSpec::for_job(&repo.root_str(), JOB).expect("a legal spec");
    let worktree = GitVcs::new().create_worktree(&spec).expect("a worktree");
    std::fs::write(format!("{}/work.txt", worktree.path()), "the job's work").expect("the file");
    GitVcs::new()
        .commit_all(
            &worktree,
            "the job's work",
            CommitTime::seconds_since_epoch(1_787_734_800),
        )
        .expect("a commit");
    GitVcs::new().push(&worktree).expect("the first push");

    // The remote's base moves on without this machine: committed here so the
    // bare repository has it, then taken back off the local branch and its
    // tracking ref, which is how a machine that has not fetched reads.
    repo.commit_one("landed.txt", "landed on the remote", "landed on the remote");
    repo.git(&["push", "origin", "main"]);
    let new = repo.git(&["rev-parse", "main"]);
    repo.git(&["reset", "--hard", &old]);
    repo.git(&["update-ref", "refs/remotes/origin/main", &old]);

    Stale {
        repo,
        bare,
        worktree,
        old,
        new,
    }
}

fn main_base() -> Base {
    Base::Inferred(String::from("main"))
}

fn holds(worktree: &Worktree, commit: &str) -> bool {
    std::process::Command::new("git")
        .args(["-C", worktree.path(), "merge-base", "--is-ancestor", commit, "HEAD"])
        .status()
        .expect("git on PATH")
        .success()
}

// -------------------------------------------------- the ref that is read

#[test]
fn a_stale_local_main_does_not_hide_that_the_branch_is_behind_the_remotes() {
    let held = stale();
    held.repo.git(&["fetch", "origin"]);

    assert_eq!(
        GitVcs::new()
            .standing(&held.worktree, &main_base())
            .expect("a reading"),
        Standing::Behind { commits: 1 },
        "the local main is level with the branch's cut; the remote's is a commit past it"
    );
}

#[test]
fn the_catch_up_merges_what_the_remote_holds_and_leaves_the_local_main_alone() {
    let held = stale();
    held.repo.git(&["fetch", "origin"]);

    let moved = GitVcs::new()
        .bring_up_to_date(&held.worktree, &main_base())
        .expect("a catch-up");

    assert_eq!(
        moved,
        BroughtUpToDate::Clean {
            base: String::from("main"),
            commits: 1
        }
    );
    assert!(
        holds(&held.worktree, &held.new),
        "the branch holds the remote's tip"
    );
    assert!(
        std::path::Path::new(&format!("{}/landed.txt", held.worktree.path())).exists(),
        "and the file that landed there"
    );
    assert_eq!(
        held.repo.git(&["rev-parse", "main"]),
        held.old,
        "the person's own branch is theirs, and was not moved"
    );
}

#[test]
fn a_pull_requests_branch_is_kept_current_against_the_remotes_base() {
    let held = stale();
    held.repo.git(&["fetch", "origin"]);

    let spec = WorktreeSpec::for_job(&held.repo.root_str(), JOB).expect("a legal spec");
    let outcome = GitVcs::new().kept_current(&spec, "main");

    assert_eq!(
        outcome,
        KeptCurrent::Rebased {
            onto: held.new.clone(),
            commits: 1
        },
        "the tip merged is the remote's, not `{}`",
        held.old
    );
    assert_eq!(
        held.repo.git(&["rev-parse", &format!("armada/{JOB}")]),
        std::process::Command::new("git")
            .args([
                "-C",
                &held.bare.to_string_lossy(),
                "rev-parse",
                &format!("armada/{JOB}")
            ])
            .output()
            .map(|run| String::from_utf8_lossy(&run.stdout).trim().to_string())
            .expect("git on PATH"),
        "and what was merged is what the remote now holds"
    );
}

#[test]
fn the_base_tip_a_sweep_compares_is_the_remotes() {
    let held = stale();
    held.repo.git(&["fetch", "origin"]);

    assert_eq!(
        GitVcs::new().base_tip(&held.repo.root_str(), "main"),
        Some(held.new.clone())
    );
}

// ------------------------------------------------------------ the fetch

#[test]
fn the_fetch_brings_the_remotes_base_onto_this_machine_and_moves_nothing_else() {
    let held = stale();

    assert_eq!(
        GitVcs::new().fetch_base(held.worktree.path(), "main"),
        BaseFetched::Fetched {
            remote: String::from("origin")
        }
    );
    assert_eq!(held.repo.git(&["rev-parse", "origin/main"]), held.new);
    assert_eq!(
        held.repo.git(&["rev-parse", "main"]),
        held.old,
        "the local branch is a person's and is not fast-forwarded"
    );

    let moved = GitVcs::new()
        .bring_up_to_date(&held.worktree, &main_base())
        .expect("a catch-up");
    assert!(
        matches!(moved, BroughtUpToDate::Clean { commits: 1, .. }),
        "{moved:?}"
    );
    assert!(holds(&held.worktree, &held.new));
}

#[test]
fn a_remote_that_cannot_be_reached_is_failed_with_gits_own_words_and_moves_nothing() {
    let held = stale();
    held.repo.git(&["remote", "set-url", "origin", "/nowhere/at/all.git"]);

    let BaseFetched::Failed { remote, said } = GitVcs::new().fetch_base(&held.repo.root_str(), "main")
    else {
        panic!("a remote that is not there cannot have been fetched from");
    };
    assert_eq!(remote, "origin");
    assert!(said.contains("/nowhere/at/all.git"), "git's sentence: {said}");
    assert_eq!(
        held.repo.git(&["rev-parse", "origin/main"]),
        held.old,
        "the tracking ref is as it was: it is not the remote's base, and is not claimed to be"
    );
}

#[test]
fn a_repository_with_no_remote_has_only_its_local_branch_to_read() {
    let repo = TempRepo::with_a_commit();
    assert_eq!(
        GitVcs::new().fetch_base(&repo.root_str(), "main"),
        BaseFetched::Local
    );
}

#[test]
fn a_base_the_remote_never_had_is_read_from_the_local_branch() {
    let repo = TempRepo::with_a_commit();
    repo.with_a_bare_remote();
    assert_eq!(
        GitVcs::new().fetch_base(&repo.root_str(), "main"),
        BaseFetched::Local,
        "nothing was ever pushed there, so there is no second reading"
    );
}
