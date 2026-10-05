//! What a Fleet killed part-way through a landing leaves in a real repository,
//! and that the next one reads it right: a merge half-made is cleared, and a
//! push already made is found and never made twice.

use adapter_traits::{Delivery, Landable, Merged};

use crate::onto_base;
use crate::tests::merging_by_push::{
    base_moved_elsewhere, delivered, in_bare, the_jobs_worktree, JOB,
};
use crate::tests::repo::TempRepo;
use crate::worktree::GitVcs;

/// A branch whose work and the base's both touch `work.txt`, with the merge of
/// the base into it started and stopped there, as a killed Fleet leaves it.
fn killed_mid_merge(repo: &TempRepo, bare: &std::path::Path) -> std::path::PathBuf {
    repo.commit_one("work.txt", "somebody else's", "the same file, otherwise");
    repo.git(&["push", "origin", "main"]);
    let _ = in_bare(bare, &["rev-parse", "main"]);
    let worktree = the_jobs_worktree(repo);
    let at = std::path::PathBuf::from(worktree.path());
    let merged = std::process::Command::new("git")
        .arg("-C")
        .arg(&at)
        .args(["merge", "--no-edit", "refs/remotes/origin/main"])
        .output()
        .expect("git on PATH");
    assert!(!merged.status.success(), "the two disagree, so it stops");
    at
}

fn in_a(worktree: &std::path::Path, args: &[&str]) -> String {
    let run = std::process::Command::new("git")
        .arg("-C")
        .arg(worktree)
        .args(args)
        .output()
        .expect("git on PATH");
    String::from_utf8_lossy(&run.stdout).trim().to_string()
}

/// **A worktree left half-merged is cleared and reused**: the branch is where
/// it was committed, nothing is left unmerged, and the next gate can read it.
#[test]
fn a_worktree_left_half_merged_is_cleared_and_usable_again() {
    let repo = TempRepo::with_a_commit();
    let bare = delivered(&repo);
    let branch = format!("armada/{JOB}");
    let before = repo.git(&["rev-parse", &branch]);
    let at = killed_mid_merge(&repo, &bare);
    assert!(
        !in_a(&at, &["status", "--porcelain"]).is_empty(),
        "the kill left something behind"
    );

    GitVcs::new()
        .settle_worktree(&the_jobs_worktree(&repo))
        .expect("it is cleared");

    assert_eq!(repo.git(&["rev-parse", &branch]), before);
    assert_eq!(in_a(&at, &["status", "--porcelain"]), "");
    assert!(
        GitVcs::new()
            .the_unchecked_head(&repo.root_str(), &the_jobs_worktree(&repo), None, None)
            .is_ok(),
        "a gate reads it again, where it refused a merge part-way through"
    );
}

/// Where nothing is half-made, settling touches nothing.
#[test]
fn settling_a_clean_worktree_changes_nothing() {
    let repo = TempRepo::with_a_commit();
    delivered(&repo);
    let branch = format!("armada/{JOB}");
    let before = repo.git(&["rev-parse", &branch]);

    GitVcs::new()
        .settle_worktree(&the_jobs_worktree(&repo))
        .expect("nothing to clear");

    assert_eq!(repo.git(&["rev-parse", &branch]), before);
}

/// **A push already made is found, named, and never made twice.** The first
/// call is the turn that died after its push; the second is the next Fleet's.
#[test]
fn a_landing_whose_push_was_made_is_named_and_not_merged_again() {
    let repo = TempRepo::with_a_commit();
    let bare = delivered(&repo);
    let moved = base_moved_elsewhere(&repo, &bare);
    let merged = GitVcs::new()
        .merge_the_moved_base_in(&repo.root_str(), &the_jobs_worktree(&repo), None)
        .expect("it merges clean");
    let first = GitVcs::new()
        .merge_by_push(
            &repo.root_str(),
            JOB,
            None,
            Some(3),
            Landable::Checked(&merged.tree),
        )
        .expect("the push is made");
    let landed = in_bare(&bare, &["rev-parse", "main"]);

    let next = GitVcs::new()
        .merge_by_push(&repo.root_str(), JOB, None, Some(3), Landable::Unchecked)
        .expect("the base already holds it");

    assert_eq!(next.merged, Merged::AlreadyMerged);
    assert_eq!(next.merge.as_deref(), Some(landed.as_str()));
    assert_eq!(next.merge, first.merge, "the merge the push made");
    assert_eq!(in_bare(&bare, &["rev-parse", "main"]), landed);
    let merges = in_bare(
        &bare,
        &[
            "rev-list",
            "--first-parent",
            "--merges",
            "--count",
            &format!("{moved}..main"),
        ],
    );
    assert_eq!(merges, "1", "one landing on the base, not two");
}

/// The base holds a branch by a road that was not a merge of it: no merge to
/// name, and none is invented.
#[test]
fn a_branch_the_base_holds_without_a_merge_names_none() {
    let repo = TempRepo::with_a_commit();
    let _bare = delivered(&repo);
    let head = repo.git(&["rev-parse", &format!("armada/{JOB}")]);
    repo.git(&["merge", "--ff-only", &head]);
    repo.git(&["push", "origin", "main"]);

    assert_eq!(
        onto_base::merge_naming(repo.root(), "refs/remotes/origin/main", &head),
        None
    );
}
