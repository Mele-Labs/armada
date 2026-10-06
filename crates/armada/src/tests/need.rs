//! The claim: two branches each declare a need on `migrations.rs`; the second
//! is told the first is ahead; ready first, it waits in the line and says what
//! it waits behind; once the first lands, the second may land; and a deleted
//! branch's need blocks nobody.

use std::path::Path;
use std::process::Command;

use crate::land::{nonce, write_queue_entry, QueueEntry, StateDir};
use crate::need::{declare, declared_text, status_text, waiting_text, Needs};
use crate::tests::TempDir;

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

fn in_line(state: &StateDir, branch: &str, place: i64) {
    write_queue_entry(
        state,
        &QueueEntry {
            branch: branch.to_string(),
            pr: None,
            head: "a".repeat(40),
            tree: "b".repeat(40),
            place,
            worktree: "/tmp/somewhere".to_string(),
            nonce: nonce(),
        },
    )
    .expect("queued");
}

fn names(line: &[QueueEntry]) -> Vec<&str> {
    line.iter().map(|entry| entry.branch.as_str()).collect()
}

#[test]
fn the_second_declarer_is_told_who_is_ahead_and_waits_for_it_to_land() {
    let repo = a_repository_with(&["first", "second"]);
    let needs = Needs::of(repo.path()).expect("needs");
    let state = StateDir::resolve(repo.path()).expect("state");

    let one = declare(&needs, repo.path(), "first", PATH, "a new migration").expect("declared");
    assert!(one.ahead.is_empty());
    needs.took("first", PATH, "V95").expect("took");

    let two = declare(&needs, repo.path(), "second", PATH, "a new migration").expect("declared");
    let told = declared_text(&two);
    assert!(told.contains("first: a new migration, took V95"), "{told}");

    // The second is ready first, and the line holds it.
    in_line(&state, "second", 1);
    let held = crate::land::hold::unheld(&state, crate::land::queued(&state).expect("line"));
    assert!(held.is_empty(), "{:?}", names(&held));
    let said = waiting_text(&needs.behind("second"));
    assert!(said.contains("waiting behind a new migration on"), "{said}");
    assert!(said.contains("first") && said.contains("V95"), "{said}");

    // The first is ready and goes; once it lands, the second may.
    in_line(&state, "first", 2);
    let line = crate::land::queued(&state).expect("line");
    assert_eq!(names(&crate::land::hold::unheld(&state, line)), ["first"]);
    needs.spend("first");
    let line = crate::land::queued(&state).expect("line");
    assert_eq!(
        names(&crate::land::hold::unheld(&state, line)),
        ["second", "first"]
    );
}

#[test]
fn a_deleted_branch_s_need_blocks_nobody() {
    let repo = a_repository_with(&["gone", "waiting"]);
    let needs = Needs::of(repo.path()).expect("needs");
    declare(&needs, repo.path(), "gone", PATH, "a new migration").expect("declared");
    declare(&needs, repo.path(), "waiting", PATH, "a new migration").expect("declared");
    assert_eq!(needs.behind("waiting").len(), 1);

    git(repo.path(), &["branch", "-D", "gone"]);
    assert!(needs.behind("waiting").is_empty());
    assert_eq!(needs.standing().len(), 1);
}

#[test]
fn declaring_again_records_nothing_and_release_gives_back() {
    let repo = a_repository_with(&["only", "next"]);
    let needs = Needs::of(repo.path()).expect("needs");
    let first = declare(&needs, repo.path(), "only", PATH, "a minor").expect("declared");
    let again = declare(&needs, repo.path(), "only", PATH, "a minor").expect("declared");
    assert!(again.already && !first.already);
    assert_eq!(first.mine.place, again.mine.place);
    assert_eq!(needs.standing().len(), 1);

    declare(&needs, repo.path(), "next", PATH, "a minor").expect("declared");
    let listed = status_text(&needs.standing());
    assert!(
        listed.contains("1. only: a minor") && listed.contains("2. next: a minor"),
        "{listed}"
    );

    assert!(needs.release("only", PATH));
    assert!(!needs.release("only", PATH));
    assert!(needs.behind("next").is_empty());
}

#[test]
fn a_branch_that_already_changed_the_path_is_told_to_search_for_its_old_number() {
    let repo = a_repository_with(&["early"]);
    git(
        repo.path(),
        &["update-ref", "refs/remotes/origin/main", "HEAD"],
    );
    let needs = Needs::of(repo.path()).expect("needs");
    declare(&needs, repo.path(), "early", PATH, "a new migration").expect("declared");

    git(repo.path(), &["checkout", "--quiet", "-b", "late"]);
    let file = repo.path().join(PATH);
    std::fs::create_dir_all(file.parent().expect("a parent")).expect("dirs");
    std::fs::write(&file, "V95\n").expect("written");
    git(repo.path(), &["add", PATH]);
    git(repo.path(), &["commit", "--quiet", "-m", "took V95"]);

    let late = declare(&needs, repo.path(), "late", PATH, "a new migration").expect("declared");
    assert!(late.late);
    assert!(declared_text(&late).contains("search comments and docs"));
}
