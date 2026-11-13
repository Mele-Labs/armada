//! A build's position against `origin/main`, against real git: the counts come
//! from git's own symmetric difference, and a commit the repository does not
//! hold is no count rather than a wrong one.

use std::time::Duration;

use super::repo::TempRepo;
use crate::{
    branch_in_build, branch_tip, build_made_of, fetch_main, fetch_origin, origin_branches, position_against_main, BranchInBuild,
    BuildMade,
};

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

fn branch_at(repo: &TempRepo, name: &str, commit: &str) {
    let id = git2::Oid::from_str(commit).expect("an object id");
    repo.open()
        .reference(&format!("refs/heads/{name}"), id, true, "a test's branch")
        .expect("branch written");
}

/// `origin/main` at the first commit, which is what a branch's own commits are counted from.
fn a_repository_at_main() -> (TempRepo, String) {
    let repo = TempRepo::with_a_commit();
    let first = repo.head_str();
    point_origin_main_at(&repo, &first);
    (repo, first)
}

#[test]
fn a_branch_whose_every_commit_is_in_the_build_is_in_it() {
    let (repo, first) = a_repository_at_main();
    let tip = child_of(&repo, &first, "the branch's");
    branch_at(&repo, "armada/work", &tip);
    let built = child_of(&repo, &tip, "a build holding it");
    assert_eq!(branch_in_build(&repo.root_str(), &built, "armada/work"), Some(BranchInBuild::In { tip }));
}

#[test]
fn a_branch_none_of_whose_commits_are_in_the_build_is_not_in_it() {
    let (repo, first) = a_repository_at_main();
    let one = child_of(&repo, &first, "one");
    let two = child_of(&repo, &one, "two");
    branch_at(&repo, "armada/work", &two);
    let built = child_of(&repo, &first, "a build cut from main");
    assert_eq!(branch_in_build(&repo.root_str(), &built, "armada/work"), Some(BranchInBuild::NotIn));
}

#[test]
fn a_branch_ahead_of_the_build_is_in_it_up_to_the_newest_commit_the_build_holds() {
    let (repo, first) = a_repository_at_main();
    let one = child_of(&repo, &first, "one");
    let two = child_of(&repo, &one, "two");
    let three = child_of(&repo, &two, "three");
    branch_at(&repo, "armada/work", &three);
    let built = child_of(&repo, &one, "a build cut when only one was made");
    assert_eq!(
        branch_in_build(&repo.root_str(), &built, "armada/work"),
        Some(BranchInBuild::Partly { newest: one, since: 2 })
    );
}

/// A merge of `theirs` into `ours`, as a session merging a newer main into its branch makes.
fn merge_of(repo: &TempRepo, ours: &str, theirs: &str) -> String {
    let opened = repo.open();
    let find = |one: &str| opened.find_commit(git2::Oid::from_str(one).expect("an object id")).expect("a parent");
    let (ours, theirs) = (find(ours), find(theirs));
    let who = git2::Signature::now("armada", "armada@example.invalid").expect("a signature");
    let tree = ours.tree().expect("a tree");
    let made = opened
        .commit(None, &who, &who, "a merge of main", &tree, &[&ours, &theirs])
        .expect("a merge");
    made.to_string()
}

#[test]
fn a_newer_main_merged_into_the_branch_is_not_counted_as_its_own() {
    let (repo, first) = a_repository_at_main();
    let work = child_of(&repo, &first, "work");
    let built = child_of(&repo, &work, "a build holding the work");
    let newer = child_of(&repo, &first, "main moves on");
    point_origin_main_at(&repo, &newer);
    let merged = merge_of(&repo, &work, &newer);
    branch_at(&repo, "armada/work", &merged);
    assert_eq!(
        branch_in_build(&repo.root_str(), &built, "armada/work"),
        Some(BranchInBuild::Partly { newest: work, since: 1 }),
        "only the merge commit is the branch's own and not in the build"
    );
}

#[test]
fn main_a_branch_with_nothing_of_its_own_and_one_that_is_not_there_say_nothing() {
    let (repo, first) = a_repository_at_main();
    let built = child_of(&repo, &first, "a build");
    branch_at(&repo, "armada/idle", &first);
    for branch in ["main", "armada/idle", "armada/gone"] {
        assert_eq!(branch_in_build(&repo.root_str(), &built, branch), None, "{branch}");
    }
    branch_at(&repo, "armada/work", &child_of(&repo, &first, "work"));
    let missing = "0123456789abcdef0123456789abcdef01234567";
    assert_eq!(branch_in_build(&repo.root_str(), missing, "armada/work"), None, "a build the repository lacks");
}

#[test]
fn a_branch_tip_is_the_local_branch_and_a_missing_branch_has_none() {
    let (repo, first) = a_repository_at_main();
    let work = child_of(&repo, &first, "work");
    branch_at(&repo, "armada/work", &work);
    assert_eq!(branch_tip(&repo.root_str(), "armada/work"), Some(work));
    assert_eq!(branch_tip(&repo.root_str(), "armada/gone"), None);
}

/// A merge of `side` into `first`, worded as `scripts/preview` words it.
fn merged(repo: &TempRepo, first: &str, side: &str, branch: &str) -> String {
    let opened = repo.open();
    let find = |id: &str| opened.find_commit(git2::Oid::from_str(id).expect("an object id")).expect("a commit");
    let (first, side) = (find(first), find(side));
    let who = git2::Signature::now("armada", "armada@example.invalid").expect("a signature");
    let tree = first.tree().expect("a tree");
    let message = format!("Merge branch '{branch}' into preview");
    opened.commit(None, &who, &who, &message, &tree, &[&first, &side]).expect("a merge").to_string()
}

#[test]
fn a_preview_is_read_from_its_merges_down_to_the_main_commit_under_them() {
    let repo = TempRepo::with_a_commit();
    let base = repo.head_str();
    point_origin_main_at(&repo, &base);
    let one = child_of(&repo, &base, "one");
    let two = child_of(&repo, &base, "two");
    let first = merged(&repo, &base, &one, "armada/a");
    let preview = merged(&repo, &first, &two, "armada/b");
    let made = build_made_of(&repo.root_str(), &preview).expect("a preview");
    assert_eq!(made, BuildMade { preview: true, main_commit: base, branches: vec!["armada/b".into(), "armada/a".into()] });
}

#[test]
fn a_commit_on_origin_main_is_a_main_build_and_one_that_is_neither_is_not_said() {
    let repo = TempRepo::with_a_commit();
    let base = repo.head_str();
    let landed = child_of(&repo, &base, "landed");
    point_origin_main_at(&repo, &landed);
    assert_eq!(build_made_of(&repo.root_str(), &landed), Some(BuildMade { preview: false, main_commit: landed.clone(), branches: vec![] }));
    let loose = child_of(&repo, &base, "a branch's own");
    assert_eq!(build_made_of(&repo.root_str(), &loose), None, "not a preview, and not main's");
    assert_eq!(build_made_of(&repo.root_str(), "0123456789abcdef0123456789abcdef01234567"), None, "not here at all");
}

/// A branch `name` in `repo` at a commit on top of `parent`, committed at `unix` seconds.
fn origin_branch_at(repo: &TempRepo, name: &str, parent: &str, unix: i64) -> String {
    let opened = repo.open();
    let parent = opened.find_commit(git2::Oid::from_str(parent).expect("an object id")).expect("the parent");
    let who = git2::Signature::new("armada", "armada@example.invalid", &git2::Time::new(unix, 0)).expect("a signature");
    let tree = parent.tree().expect("a tree");
    let made = opened.commit(None, &who, &who, name, &tree, &[&parent]).expect("a commit");
    opened.reference(&format!("refs/heads/{name}"), made, true, "a test's branch").expect("the branch");
    made.to_string()
}

#[test]
fn a_fetch_of_origin_lists_every_branch_main_lacks_newest_first_and_since_a_time() {
    let origin = TempRepo::with_a_commit();
    let main = origin.head_str();
    origin_branch_at(&origin, "feat/old", &main, 1_600_000_000);
    origin_branch_at(&origin, "native/refresh-set", &main, 1_700_000_000);
    origin.open().reference("refs/heads/feat/landed", git2::Oid::from_str(&main).expect("an id"), true, "landed").expect("landed");
    let repo = TempRepo::empty();
    repo.open().remote("origin", &origin.root_str()).expect("a remote");
    assert_eq!(origin_branches(&repo.root_str(), 0), Vec::new(), "nothing fetched, nothing listed");
    fetch_origin(&repo.root_str(), Duration::from_secs(20)).expect("fetched from a local path");
    let listed = origin_branches(&repo.root_str(), 0);
    let names: Vec<&str> = listed.iter().map(|(branch, _)| branch.as_str()).collect();
    assert_eq!(names, ["native/refresh-set", "feat/old"]);
    assert_eq!(listed[0].1, "2023-11-14T22:13:20Z");
    let recent: Vec<String> = origin_branches(&repo.root_str(), 1_650_000_000).into_iter().map(|(branch, _)| branch).collect();
    assert_eq!(recent, ["native/refresh-set"]);
}
