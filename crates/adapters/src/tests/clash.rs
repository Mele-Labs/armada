//! Whether two worktrees' changes conflict, against real git: the verdict is `git merge-tree`'s.

use std::path::PathBuf;

use super::repo::TempRepo;
use crate::clash::{against, between, changed_between, changed_here, changed_on, file_against, file_between, landed_in, Verdict};

fn lines() -> String {
    (1..=20).map(|n| format!("line {n}\n")).collect()
}

/// A repository with `f.txt` and `g.txt` committed, and two worktrees cut from it.
fn two_worktrees() -> (TempRepo, PathBuf, PathBuf) {
    let repo = TempRepo::with_a_commit();
    repo.commit_one("f.txt", &lines(), "add f");
    repo.commit_one("g.txt", "g\n", "add g");
    let (one, two) = (
        repo.root().join(".armada/slots/slot-1"),
        repo.root().join(".armada/slots/slot-2"),
    );
    for (path, branch) in [(&one, "one"), (&two, "two")] {
        repo.git(&["worktree", "add", "-q", &path.to_string_lossy(), "-b", branch]);
    }
    (repo, one, two)
}

fn edit(worktree: &PathBuf, file: &str, line: usize, to: &str) {
    let path = worktree.join(file);
    let mut text: Vec<String> = std::fs::read_to_string(&path)
        .expect("the file")
        .lines()
        .map(str::to_string)
        .collect();
    text[line - 1] = to.to_string();
    std::fs::write(path, text.join("\n") + "\n").expect("written");
}

#[test]
fn one_file_changed_in_two_places_far_apart_is_clean() {
    let (_repo, one, two) = two_worktrees();
    edit(&one, "f.txt", 1, "one's first");
    edit(&two, "f.txt", 20, "two's last");
    assert_eq!(between(&one, &two), Ok(Verdict::Clean));
}

#[test]
fn the_same_line_changed_twice_is_a_conflict_naming_the_file() {
    let (_repo, one, two) = two_worktrees();
    edit(&one, "f.txt", 5, "one");
    edit(&two, "f.txt", 5, "two");
    edit(&two, "g.txt", 1, "only two touches this");
    assert_eq!(between(&one, &two), Ok(Verdict::Conflict(vec!["f.txt".into()])));
}

#[test]
fn a_file_neither_committed_nor_staged_is_in_the_comparison() {
    let (_repo, one, two) = two_worktrees();
    std::fs::write(one.join("new.txt"), "one\n").expect("written");
    std::fs::write(two.join("new.txt"), "two\n").expect("written");
    assert_eq!(between(&one, &two), Ok(Verdict::Conflict(vec!["new.txt".into()])));
}

#[test]
fn reading_a_worktree_leaves_its_index_as_it_was() {
    let (repo, one, two) = two_worktrees();
    edit(&one, "f.txt", 5, "one");
    edit(&two, "f.txt", 5, "two");
    let before = repo.git(&["-C", &one.to_string_lossy(), "status", "--porcelain"]);
    let _ = between(&one, &two);
    assert_eq!(repo.git(&["-C", &one.to_string_lossy(), "status", "--porcelain"]), before);
    // The helper trims the line, so an unstaged change reads `M f.txt` and a staged one `M  f.txt`.
    assert_eq!(before, "M f.txt", "the change is still unstaged");
}

#[test]
fn a_base_that_changed_the_same_line_conflicts_and_says_where_it_landed() {
    let (repo, one, _two) = two_worktrees();
    edit(&one, "f.txt", 5, "one");
    repo.commit_one("f.txt", &lines().replace("line 5\n", "main's five\n"), "Merge pull request #7");
    let tip = repo.head_str();
    assert_eq!(against(&one, &tip), Ok(Verdict::Conflict(vec!["f.txt".into()])));
    assert_eq!(changed_on(&one, &tip), Ok(vec!["f.txt".into()]));
    assert!(landed_in(&one, &tip, "f.txt").is_some_and(|said| said.ends_with("Merge pull request #7")));
}

#[test]
fn what_a_worktree_changed_is_its_commits_and_its_uncommitted_files_and_none_of_the_bases() {
    let (repo, one, _two) = two_worktrees();
    edit(&one, "f.txt", 5, "one");
    repo.git(&["-C", &one.to_string_lossy(), "commit", "-qam", "one's f"]);
    std::fs::write(one.join("new.txt"), "one\n").expect("written");
    repo.commit_one("g.txt", "g, changed on the base\n", "change g");
    let base = repo.head_str();
    assert_eq!(changed_here(&one, &base), Ok(vec!["f.txt".into(), "new.txt".into()]));
}

#[test]
fn a_base_that_changed_another_file_is_clean() {
    let (repo, one, _two) = two_worktrees();
    edit(&one, "f.txt", 5, "one");
    repo.commit_one("g.txt", "g, changed\n", "change g");
    let tip = repo.head_str();
    assert_eq!(against(&one, &tip), Ok(Verdict::Clean));
    assert_eq!(changed_on(&one, &tip), Ok(vec!["g.txt".into()]));
}

#[test]
fn a_file_is_each_sides_patch_from_the_merge_base_and_neither_carries_the_others() {
    let (repo, one, two) = two_worktrees();
    edit(&one, "f.txt", 5, "one");
    edit(&two, "f.txt", 5, "two");
    edit(&two, "g.txt", 1, "two's g");
    repo.git(&["-C", &two.to_string_lossy(), "commit", "-qam", "two's"]);
    edit(&two, "f.txt", 6, "two again, uncommitted");
    let before = repo.git(&["-C", &one.to_string_lossy(), "status", "--porcelain"]);

    let read = file_between(&one, &two, "f.txt").expect("read");
    assert!(read.ours.starts_with("diff --git a/f.txt b/f.txt\n"), "{}", read.ours);
    assert!(read.ours.contains("-line 5\n+one\n") && !read.ours.contains("two"), "{}", read.ours);
    assert!(read.theirs.contains("-line 5\n-line 6\n+two\n+two again, uncommitted\n"), "{}", read.theirs);
    assert!(!read.theirs.contains("g.txt"), "one file only: {}", read.theirs);
    assert_eq!(repo.git(&["-C", &one.to_string_lossy(), "status", "--porcelain"]), before, "nothing written");
}

#[test]
fn a_file_one_side_left_alone_is_an_empty_patch_and_a_new_file_is_all_added() {
    let (_repo, one, two) = two_worktrees();
    std::fs::write(one.join("new.txt"), "one\n").expect("written");
    std::fs::write(two.join("new.txt"), "two\n").expect("written");
    edit(&one, "f.txt", 5, "one");
    let new = file_between(&one, &two, "new.txt").expect("read");
    assert!(new.ours.contains("new file mode") && new.ours.contains("+one\n"), "{}", new.ours);
    assert!(new.theirs.contains("+two\n"), "{}", new.theirs);
    let untouched = file_between(&one, &two, "f.txt").expect("read");
    assert!(untouched.ours.contains("+one\n"));
    assert_eq!(untouched.theirs, "", "two never touched it");
}

#[test]
fn a_name_with_pattern_characters_is_that_file_and_no_pattern() {
    let (_repo, one, two) = two_worktrees();
    std::fs::write(one.join("a[1].txt"), "bracket\n").expect("written");
    std::fs::write(one.join("a1.txt"), "plain\n").expect("written");
    let read = file_between(&one, &two, "a[1].txt").expect("read");
    assert!(read.ours.contains("+bracket\n") && !read.ours.contains("plain"), "a1.txt is not a[1].txt: {}", read.ours);
}

#[test]
fn a_file_against_the_base_is_the_worktrees_change_and_the_bases_since_they_met() {
    let (repo, one, _two) = two_worktrees();
    edit(&one, "f.txt", 5, "one");
    repo.commit_one("f.txt", &lines().replace("line 5\n", "main's five\n"), "main's");
    let tip = repo.head_str();
    let read = file_against(&one, &tip, "f.txt").expect("read");
    assert!(read.ours.contains("-line 5\n+one\n"), "{}", read.ours);
    assert!(read.theirs.contains("-line 5\n+main's five\n"), "{}", read.theirs);
}

#[test]
fn what_changed_between_two_commits_names_edits_deletions_and_both_sides_of_a_rename() {
    let repo = TempRepo::with_a_commit();
    repo.commit_one("kept.txt", &lines(), "add kept");
    repo.commit_one("gone.txt", "gone\n", "add gone");
    repo.commit_one("old.txt", &lines(), "add old");
    let from = repo.head_str();
    repo.commit_one("kept.txt", &format!("changed\n{}", lines()), "edit kept");
    repo.git(&["rm", "-q", "gone.txt"]);
    repo.git(&["mv", "old.txt", "new.txt"]);
    repo.git(&["-c", "user.name=armada", "-c", "user.email=armada@example.invalid", "commit", "-q", "-m", "delete and rename"]);
    let to = repo.head_str();

    let mut changed = changed_between(repo.root(), &from, &to).expect("git answers");
    changed.sort();
    assert_eq!(changed, ["gone.txt", "kept.txt", "new.txt", "old.txt"]);
    assert_eq!(changed_between(repo.root(), &to, &to), Ok(Vec::new()));
    assert!(changed_between(repo.root(), "0000000000000000000000000000000000000000", &to).is_err());
}
