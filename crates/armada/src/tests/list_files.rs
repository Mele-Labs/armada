//! Two branches that each append a line to a declared list file merge one
//! after another with no conflict, and both lines are there. #1059, decided
//! 2 Oct 2026.
//!
//! **Written against `armada land`'s own merge**: the candidate worktree merges
//! the base in through `GitVcs::bring_up_to_date` (`land::merge_in` is that call
//! plus a regeneration), which is plain `git merge`, so it reads
//! `.gitattributes`. The repository's own `.gitattributes` is the one under
//! test, not a copy. Real git in a temporary directory, which the acceptance
//! package may not do: it is hermetic by rule.
//!
//! The gate's half, that a declared file holds only entries, is
//! `xtask/src/rules_list_files/tests.rs`.

use std::path::Path;
use std::process::Command;

use adapter_traits::{Base, BroughtUpToDate, Delivery, Worktree};
use adapters::GitVcs;

use crate::tests::{repository, TempDir};

const LIST: &str = "packages/components/src/index.ts";

fn git(at: &Path, args: &[&str]) {
    let run = Command::new("git")
        .arg("-C")
        .arg(at)
        .args(["-c", "user.name=t", "-c", "user.email=t@example.test"])
        .args([
            "-c",
            "init.defaultBranch=main",
            "-c",
            "commit.gpgSign=false",
        ])
        .args(args)
        .output()
        .expect("git on PATH");
    assert!(
        run.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&run.stderr)
    );
}

fn append(at: &Path, line: &str, message: &str) {
    let path = at.join(LIST);
    let mut text = std::fs::read_to_string(&path).expect("the list");
    text.push_str(line);
    std::fs::write(&path, text).expect("the list");
    git(at, &["commit", "-qam", message]);
}

/// `main` holds the list, with or without the repository's `.gitattributes`,
/// and two branches cut from it each append their own line.
fn two_branches(declared: bool) -> TempDir {
    let repo = TempDir::new();
    git(repo.path(), &["init", "-q"]);
    repo.write(LIST, "export * from \"./a/A\";\n");
    if declared {
        let attributes = std::fs::read_to_string(repository().join(".gitattributes"))
            .expect("the repository's .gitattributes");
        repo.write(".gitattributes", &attributes);
        git(repo.path(), &["add", ".gitattributes"]);
    }
    git(repo.path(), &["add", LIST]);
    git(repo.path(), &["commit", "-qm", "base"]);
    for (branch, line) in [
        ("one", "export * from \"./one/One\";\n"),
        ("two", "export * from \"./two/Two\";\n"),
    ] {
        git(repo.path(), &["checkout", "-qb", branch, "main"]);
        append(repo.path(), line, branch);
    }
    git(repo.path(), &["checkout", "-q", "main"]);
    repo
}

fn bring_main_in(repo: &Path, branch: &str) -> BroughtUpToDate {
    git(repo, &["checkout", "-q", branch]);
    let at = Worktree::at(repo.to_string_lossy().to_string(), branch.to_string());
    GitVcs::new()
        .bring_up_to_date(&at, &Base::Inferred("main".to_string()))
        .expect("the merge ran")
}

/// What landing `one`, then `two`, does: each lands on `main` and the next
/// branch has the base merged in.
fn land_both(repo: &Path) -> BroughtUpToDate {
    git(repo, &["merge", "-q", "--no-ff", "-m", "land one", "one"]);
    bring_main_in(repo, "two")
}

#[test]
fn two_appends_to_a_declared_list_file_merge_with_both_lines() {
    let repo = two_branches(true);
    let merged = land_both(repo.path());
    assert!(
        matches!(merged, BroughtUpToDate::Clean { .. }),
        "a declared list file conflicted"
    );
    let text = std::fs::read_to_string(repo.path().join(LIST)).expect("the list");
    for line in ["./a/A", "./one/One", "./two/Two"] {
        assert!(text.contains(line), "{line} is missing from:\n{text}");
    }
}

#[test]
fn the_same_two_appends_conflict_where_the_file_is_not_declared() {
    let repo = two_branches(false);
    let merged = land_both(repo.path());
    assert!(
        matches!(merged, BroughtUpToDate::Conflicted { .. }),
        "the control did not conflict"
    );
}
