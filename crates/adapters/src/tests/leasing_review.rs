//! A bay leased to read a pull request in: its head fetched from the forge's ref, checked out
//! on a branch of the lease's own, and given back clean.

use adapter_traits::{ReviewBay, SlotPool, SlotStanding, Vcs};

use crate::tests::repo::TempRepo;
use crate::GitVcs;

/// A repository whose remote holds pull request 7's head and whose local clone does not.
fn a_repository_with_pull_request_7() -> (TempRepo, String) {
    let repo = TempRepo::with_a_commit();
    repo.commit_one(".gitignore", "target/\n", "ignore builds");
    repo.with_a_bare_remote();
    repo.git(&["push", "-q", "origin", "main"]);
    repo.git(&["checkout", "-q", "-b", "the-change"]);
    repo.commit_one("README.md", "how to test it\n", "add a readme");
    let head = repo.git(&["rev-parse", "HEAD"]).trim().to_string();
    repo.git(&["push", "-q", "origin", "the-change:refs/pull/7/head"]);
    repo.git(&["checkout", "-q", "main"]);
    repo.git(&["branch", "-q", "-D", "the-change"]);
    (repo, head)
}

fn pool(repo: &TempRepo, count: u32) -> SlotPool {
    SlotPool::of(&repo.root().to_string_lossy(), count, "main", Vec::new())
}

fn branches(repo: &TempRepo) -> String {
    repo.git(&["branch", "--list"])
}

#[test]
fn a_bay_holds_the_pull_requests_head_and_goes_back_with_nothing_left_in_it() {
    let (repo, head) = a_repository_with_pull_request_7();
    let pool = pool(&repo, 2);

    let ReviewBay::Took { slot, path, head: at } = GitVcs::new().lease_review_bay(&pool, 7, "review-a").expect("leased") else {
        panic!("a free pool gave no bay");
    };
    assert_eq!(at, head, "the whole commit the checkout is at");
    assert_eq!(std::fs::read_to_string(std::path::Path::new(&path).join("README.md")).unwrap(), "how to test it\n");
    assert_eq!(GitVcs::new().slot_standing(&pool, slot, "review-a"), SlotStanding::Held);

    // Whatever a read left behind is dropped, so the pool's own refusal for a dirty tree cannot hold it.
    std::fs::write(std::path::Path::new(&path).join("scratch.txt"), "left behind").unwrap();
    std::fs::write(std::path::Path::new(&path).join("README.md"), "changed").unwrap();
    GitVcs::new().release_review_bay(&pool, slot, "review-a").expect("given back");
    assert_eq!(GitVcs::new().slot_standing(&pool, slot, "review-a"), SlotStanding::Free);
    assert!(!branches(&repo).contains("review/"), "the lease's branch is deleted: {}", branches(&repo));
}

#[test]
fn a_full_pool_waits_and_leaves_no_branch_behind() {
    let (repo, _) = a_repository_with_pull_request_7();
    let pool = pool(&repo, 1);
    assert!(matches!(GitVcs::new().lease_review_bay(&pool, 7, "review-a"), Ok(ReviewBay::Took { .. })));
    assert_eq!(GitVcs::new().lease_review_bay(&pool, 7, "review-b"), Ok(ReviewBay::Full));
    assert!(!branches(&repo).contains("review/review-b"), "{}", branches(&repo));
}

#[test]
fn a_pull_request_the_forge_does_not_have_is_refused_in_a_sentence() {
    let (repo, _) = a_repository_with_pull_request_7();
    let pool = pool(&repo, 1);
    let said = GitVcs::new().lease_review_bay(&pool, 99, "review-a").unwrap_err();
    assert!(!said.is_empty());
    assert!(matches!(GitVcs::new().lease_review_bay(&pool, 7, "review-a"), Ok(ReviewBay::Took { .. })), "the slot was not taken by the refusal");
}
