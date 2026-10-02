//! `merge_by: push`, asked of a real repository with a bare remote beside it,
//! for `crate::tests::delivery`'s reason.

use std::path::Path;

use adapter_traits::{CommitTime, Delivery, Merged, NotMerged, Vcs, Worktree, WorktreeSpec};

use crate::tests::repo::TempRepo;
use crate::worktree::GitVcs;

const JOB: &str = "01K9PUSHED000000000000001";

/// A repository whose `main` is on its remote, and a Job's branch delivered on
/// top of it — what a pull request is open over when a person presses.
fn delivered(repo: &TempRepo) -> std::path::PathBuf {
    let bare = repo.with_a_bare_remote();
    repo.git(&["config", "user.name", "armada"]);
    repo.git(&["config", "user.email", "armada@example.invalid"]);
    repo.git(&["push", "--set-upstream", "origin", "main"]);
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
    GitVcs::new().push(&worktree).expect("the branch is pushed");
    bare
}

fn in_bare(bare: &Path, args: &[&str]) -> String {
    let run = std::process::Command::new("git")
        .arg("-C")
        .arg(bare)
        .args(args)
        .output()
        .expect("git on PATH");
    String::from_utf8_lossy(&run.stdout).trim().to_string()
}

/// **What lands is the branch's own tree, as a `--no-ff` merge on the base the
/// remote held**, named the way `armada land` names its own.
#[test]
fn the_merge_commit_is_pushed_onto_the_base_holding_the_branchs_tree() {
    let repo = TempRepo::with_a_commit();
    let bare = delivered(&repo);
    let base_before = repo.git(&["rev-parse", "main"]);
    let branch = format!("armada/{JOB}");
    let head = repo.git(&["rev-parse", &branch]);

    let pushed = GitVcs::new()
        .merge_by_push(&repo.root_str(), JOB, None, Some(7))
        .expect("it lands");

    assert_eq!(pushed.base, "main");
    assert_eq!(pushed.merged, Merged::Taken);
    let parents = in_bare(&bare, &["rev-list", "--parents", "-n", "1", "main"]);
    let parents: Vec<&str> = parents.split_whitespace().skip(1).collect();
    assert_eq!(parents, [base_before.as_str(), head.as_str()]);
    assert_eq!(
        in_bare(&bare, &["rev-parse", "main^{tree}"]),
        repo.git(&["rev-parse", &format!("{branch}^{{tree}}")]),
        "the tree that landed is the tree that was gated"
    );
    assert_eq!(
        in_bare(&bare, &["log", "-1", "--format=%B", "main"]),
        format!("Merge pull request #7 from {branch}\n\nLanded-from: {branch}")
    );
}

/// A second press over work already on the base is not a failure, and pushes
/// nothing.
#[test]
fn a_branch_the_base_already_holds_is_already_merged() {
    let repo = TempRepo::with_a_commit();
    let bare = delivered(&repo);
    GitVcs::new()
        .merge_by_push(&repo.root_str(), JOB, None, None)
        .expect("the first lands");
    let landed = in_bare(&bare, &["rev-parse", "main"]);

    let again = GitVcs::new()
        .merge_by_push(&repo.root_str(), JOB, None, None)
        .expect("the second has nothing to do");

    assert_eq!(again.merged, Merged::AlreadyMerged);
    assert_eq!(in_bare(&bare, &["rev-parse", "main"]), landed);
}

/// **A base that moved past the branch is refused, not merged**: a merge of
/// the two would be a tree nothing gated.
#[test]
fn a_base_the_branch_does_not_hold_is_refused_and_nothing_is_pushed() {
    let repo = TempRepo::with_a_commit();
    let bare = delivered(&repo);
    repo.commit_one("elsewhere.txt", "moved on", "something else landed");
    repo.git(&["push", "origin", "main"]);
    let moved = in_bare(&bare, &["rev-parse", "main"]);

    let refused = GitVcs::new()
        .merge_by_push(&repo.root_str(), JOB, None, None)
        .expect_err("the branch does not hold the base");

    assert!(
        matches!(refused, NotMerged::BaseMoved { .. }),
        "{refused:?}"
    );
    assert_eq!(in_bare(&bare, &["rev-parse", "main"]), moved);
}

fn the_jobs_worktree(repo: &TempRepo) -> Worktree {
    let spec = WorktreeSpec::for_job(&repo.root_str(), JOB).expect("a legal spec");
    Worktree::at(spec.worktree_path(), spec.branch())
}

/// The base on the remote moves on in a file the Job did not touch.
fn base_moved_elsewhere(repo: &TempRepo, bare: &Path) -> String {
    repo.commit_one("elsewhere.txt", "moved on", "something else landed");
    repo.git(&["push", "origin", "main"]);
    in_bare(bare, &["rev-parse", "main"])
}

/// **Brought up and pushed again, what lands is one merge commit on top of the
/// base that moved**, carrying the branch with that base merged into it.
#[test]
fn a_moved_base_merged_in_lands_as_one_merge_commit_on_top_of_it() {
    let repo = TempRepo::with_a_commit();
    let bare = delivered(&repo);
    let moved = base_moved_elsewhere(&repo, &bare);

    let merged = GitVcs::new()
        .merge_the_moved_base_in(&repo.root_str(), &the_jobs_worktree(&repo), None)
        .expect("it merges clean");
    let pushed = GitVcs::new()
        .merge_by_push(&repo.root_str(), JOB, None, None)
        .expect("the branch holds the base now");

    assert_eq!(merged.onto, moved);
    assert!(
        merged.touched.contains(&String::from("elsewhere.txt"))
            && merged.touched.contains(&String::from("work.txt")),
        "either side, for each Check's `when`: {:?}",
        merged.touched
    );
    assert_eq!(pushed.merged, Merged::Taken);
    let parents = in_bare(&bare, &["rev-list", "--parents", "-n", "1", "main"]);
    let parents: Vec<&str> = parents.split_whitespace().skip(1).collect();
    assert_eq!(parents, [moved.as_str(), merged.head.as_str()]);
}

/// **A conflict leaves the branch exactly as it was**, with no merge left
/// half-made in the worktree, and pushes nothing.
#[test]
fn a_moved_base_that_conflicts_leaves_the_branch_as_it_was() {
    let repo = TempRepo::with_a_commit();
    let bare = delivered(&repo);
    repo.commit_one(
        "work.txt",
        "somebody else's work",
        "the same file, otherwise",
    );
    repo.git(&["push", "origin", "main"]);
    let moved = in_bare(&bare, &["rev-parse", "main"]);
    let branch = format!("armada/{JOB}");
    let before = repo.git(&["rev-parse", &branch]);

    let refused = GitVcs::new()
        .merge_the_moved_base_in(&repo.root_str(), &the_jobs_worktree(&repo), None)
        .expect_err("the two disagree");

    assert!(
        matches!(&refused, NotMerged::Conflicted { said } if said.contains("work.txt")),
        "{refused:?}"
    );
    assert_eq!(repo.git(&["rev-parse", &branch]), before);
    assert_eq!(in_bare(&bare, &["rev-parse", "main"]), moved);
}

/// A red gate puts the branch back to where it was before the merge.
#[test]
fn a_branch_put_back_is_where_it_was_before_the_merge() {
    let repo = TempRepo::with_a_commit();
    let bare = delivered(&repo);
    base_moved_elsewhere(&repo, &bare);
    let branch = format!("armada/{JOB}");
    let before = repo.git(&["rev-parse", &branch]);
    let worktree = the_jobs_worktree(&repo);
    let merged = GitVcs::new()
        .merge_the_moved_base_in(&repo.root_str(), &worktree, None)
        .expect("it merges clean");

    GitVcs::new()
        .put_back(&worktree, &merged)
        .expect("it goes back");

    assert_eq!(merged.was, before);
    assert_eq!(repo.git(&["rev-parse", &branch]), before);
}
