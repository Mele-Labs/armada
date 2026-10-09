//! A build's position against `origin/main`, against real git: the counts come
//! from git's own symmetric difference, and a commit the repository does not
//! hold is no count rather than a wrong one.

use std::time::Duration;

use super::repo::TempRepo;
use crate::{fetch_main, position_against_main};

fn point_origin_main_at(repo: &TempRepo, commit: &str) {
    let opened = repo.open();
    let id = git2::Oid::from_str(commit).expect("an object id");
    opened
        .reference("refs/remotes/origin/main", id, true, "a test's origin/main")
        .expect("origin/main written");
}

/// A commit on top of `parent` that `HEAD` does not move to.
fn child_of(repo: &TempRepo, parent: &str, message: &str) -> String {
    let opened = repo.open();
    let parent = opened
        .find_commit(git2::Oid::from_str(parent).expect("an object id"))
        .expect("the parent");
    let who = git2::Signature::now("armada", "armada@example.invalid").expect("a signature");
    let tree = parent.tree().expect("a tree");
    let made = opened
        .commit(None, &who, &who, message, &tree, &[&parent])
        .expect("a commit");
    made.to_string()
}

#[test]
fn a_build_at_origin_main_is_level_with_it() {
    let repo = TempRepo::with_a_commit();
    let head = repo.head_str();
    point_origin_main_at(&repo, &head);
    assert_eq!(position_against_main(&repo.root_str(), &head), Some((0, 0)));
}

#[test]
fn a_build_counts_what_it_holds_and_what_main_holds() {
    let repo = TempRepo::with_a_commit();
    let first = repo.head_str();
    // main moves on by two, and the build is cut from the first with one of its own.
    let second = child_of(&repo, &first, "second");
    let third = child_of(&repo, &second, "third");
    point_origin_main_at(&repo, &third);
    let built = child_of(&repo, &first, "a branch's");
    assert_eq!(position_against_main(&repo.root_str(), &built), Some((1, 2)));
}

#[test]
fn a_commit_the_repository_does_not_hold_is_no_count() {
    let repo = TempRepo::with_a_commit();
    point_origin_main_at(&repo, &repo.head_str());
    let missing = "0123456789abcdef0123456789abcdef01234567";
    assert_eq!(position_against_main(&repo.root_str(), missing), None);
    assert_eq!(position_against_main(&repo.root_str(), ""), None);
}

#[test]
fn no_origin_main_is_no_count() {
    let repo = TempRepo::with_a_commit();
    assert_eq!(position_against_main(&repo.root_str(), &repo.head_str()), None);
}

#[test]
fn a_fetch_brings_origin_main_up_and_a_repository_with_no_origin_says_so() {
    let origin = TempRepo::with_a_commit();
    let repo = TempRepo::empty();
    assert!(
        fetch_main(&repo.root_str(), Duration::from_secs(20)).is_err(),
        "no origin to fetch from"
    );
    repo.open()
        .remote("origin", &origin.root_str())
        .expect("a remote");
    fetch_main(&repo.root_str(), Duration::from_secs(20)).expect("fetched from a local path");
    let fetched = repo
        .open()
        .find_reference("refs/remotes/origin/main")
        .expect("origin/main exists now")
        .target()
        .expect("a commit")
        .to_string();
    assert_eq!(fetched, origin.head_str());
}
