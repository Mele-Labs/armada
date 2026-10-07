//! What `armada need` prints from what Fleet answered. **Fleet's side, the
//! ledger and the order a Job and a session share, is `fleet`'s `needs` tests.**

use std::path::Path;
use std::process::Command;

use crate::need::{declared_text, status_text};
use crate::tests::TempDir;
use ipc::{Holder, HolderKind, Instant, NeedAnswer, NeedLine};

const PATH: &str = "crates/store/src/migrations.rs";

fn git(repo: &Path, args: &[&str]) {
    let run = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .output()
        .expect("git on PATH");
    assert!(run.status.success(), "git {args:?}: {run:?}");
}

/// A repository with `main` and one branch for each name.
fn a_repository_with(branches: &[&str]) -> TempDir {
    let dir = TempDir::new();
    git(
        dir.path(),
        &["-c", "init.defaultBranch=main", "init", "--quiet"],
    );
    git(
        dir.path(),
        &["commit", "--allow-empty", "-m", "start", "--quiet"],
    );
    for branch in branches {
        git(dir.path(), &["branch", branch]);
    }
    dir
}

fn line(held_by: &str, what: &str, took: Option<&str>) -> NeedLine {
    NeedLine {
        holder: Holder {
            kind: HolderKind::Job,
            id: format!("job-{held_by}"),
        },
        held_by: held_by.to_string(),
        path: PATH.to_string(),
        what: what.to_string(),
        took: took.map(str::to_string),
        since: Instant::carried("2026-10-07T09:00:00.000Z"),
    }
}

fn answer(already: bool, ahead: Vec<NeedLine>) -> NeedAnswer {
    NeedAnswer {
        mine: Some(line("second", "a new migration", None)),
        already,
        ahead,
        gave_back: false,
    }
}

#[test]
fn declaring_says_who_is_ahead_and_what_it_took() {
    let told = declared_text(
        &answer(false, vec![line("first", "a new migration", Some("V95"))]),
        "second",
        PATH,
        "a new migration",
        false,
    );
    assert_eq!(
        told,
        format!(
            "need recorded: second on {PATH}: a new migration\n\
             ahead of you on {PATH}:\n  first: a new migration, took V95\n\
             pick the value after theirs, then `armada need --took <path> \"<value>\"`. \
             Fleet holds this branch's merge until they have landed or been given back.\n"
        )
    );
    let alone = declared_text(
        &answer(true, vec![]),
        "second",
        PATH,
        "a new migration",
        false,
    );
    assert_eq!(
        alone,
        format!(
            "need already recorded: second on {PATH}: a new migration\n\
             nothing is ahead of you on {PATH}\n"
        )
    );
}

#[test]
fn the_list_is_by_path_and_numbered_in_order() {
    assert_eq!(status_text(&[]), "no needs standing\n");
    let listed = status_text(&[line("only", "a minor", None), line("next", "a minor", None)]);
    assert!(
        listed.contains("1. only: a minor, took nothing yet")
            && listed.contains("2. next: a minor, took nothing yet"),
        "{listed}"
    );
}

#[test]
fn a_branch_that_already_changed_the_path_is_told_to_search_for_its_old_number() {
    let told = declared_text(
        &answer(false, vec![line("early", "a new migration", None)]),
        "late",
        PATH,
        "a new migration",
        true,
    );
    assert!(told.contains("search comments and docs"), "{told}");
}

/// What a branch that has changed the path reads as, off git.
#[test]
fn a_branch_with_the_path_changed_is_late() {
    let repo = a_repository_with(&[]);
    git(
        repo.path(),
        &["update-ref", "refs/remotes/origin/main", "HEAD"],
    );
    git(repo.path(), &["checkout", "--quiet", "-b", "late"]);
    let file = repo.path().join(PATH);
    std::fs::create_dir_all(file.parent().expect("a parent")).expect("dirs");
    std::fs::write(&file, "V95\n").expect("written");
    git(repo.path(), &["add", PATH]);
    git(repo.path(), &["commit", "--quiet", "-m", "took V95"]);
    assert!(crate::need::changes(repo.path(), PATH));
    assert!(!crate::need::changes(repo.path(), "other.rs"));
}
