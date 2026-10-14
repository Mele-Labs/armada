//! A session's branch deleted only once the base holds all of it, against real git. The remote's
//! base counts, since a merge lands there before the local base catches up.

use super::repo::TempRepo;
use crate::reclaim::delete_merged_branch;

fn ref_at(repo: &TempRepo, name: &str, commit: &str) {
    let id = git2::Oid::from_str(commit).expect("an object id");
    repo.open().reference(name, id, true, "a test's ref").expect("ref written");
}

fn child_of(repo: &TempRepo, parent: &str) -> String {
    let opened = repo.open();
    let parent = opened
        .find_commit(git2::Oid::from_str(parent).expect("an object id"))
        .expect("the parent");
    let who = git2::Signature::now("armada", "armada@example.invalid").expect("a signature");
    let tree = parent.tree().expect("a tree");
    opened
        .commit(None, &who, &who, "work", &tree, &[&parent])
        .expect("a commit")
        .to_string()
}

fn exists(repo: &TempRepo, name: &str) -> bool {
    repo.open().find_branch(name, git2::BranchType::Local).is_ok()
}

#[test]
fn a_branch_the_remote_base_holds_is_deleted_though_the_local_base_lags() {
    let repo = TempRepo::with_a_commit();
    let first = repo.head_str();
    let tip = child_of(&repo, &first);
    ref_at(&repo, "refs/heads/armada/session-1", &tip);
    ref_at(&repo, "refs/remotes/origin/main", &tip);

    assert_eq!(delete_merged_branch(&repo.root_str(), "armada/session-1"), Ok(true));
    assert!(!exists(&repo, "armada/session-1"));
}

#[test]
fn a_branch_with_a_commit_the_base_lacks_stays() {
    let repo = TempRepo::with_a_commit();
    let first = repo.head_str();
    ref_at(&repo, "refs/remotes/origin/main", &first);
    let tip = child_of(&repo, &first);
    ref_at(&repo, "refs/heads/armada/session-1", &tip);

    let refused = delete_merged_branch(&repo.root_str(), "armada/session-1").unwrap_err();
    assert!(refused.contains("commits the base does not"), "{refused}");
    assert!(exists(&repo, "armada/session-1"));
}

#[test]
fn a_branch_already_gone_is_not_an_error_and_the_base_is_never_deleted() {
    let repo = TempRepo::with_a_commit();
    assert_eq!(delete_merged_branch(&repo.root_str(), "armada/session-2"), Ok(false));
    let refused = delete_merged_branch(&repo.root_str(), "main").unwrap_err();
    assert!(refused.contains("is the base"), "{refused}");
}
