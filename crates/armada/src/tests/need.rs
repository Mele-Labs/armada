//! What `armada need` prints from what Fleet answered, and `armada land`'s own
//! hold, which still reads the files: two branches each declare a need on
//! `migrations.rs`; ready first, the second waits in the line and says what it
//! waits behind; once the first lands, the second may land; and a deleted
//! branch's need blocks nobody. **Fleet's side, the ledger and the order a Job
//! and a session share, is `fleet`'s `needs` tests.**

use std::path::Path;
use std::process::Command;

use crate::land::{nonce, write_queue_entry, QueueEntry, StateDir};
use crate::need::{declared_text, status_text, waiting_text, Needs};
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

    let one = needs
        .declare("first", PATH, "a new migration")
        .expect("declared");
    assert!(one.ahead.is_empty());
    needs.took("first", PATH, "V95").expect("took");

    needs
        .declare("second", PATH, "a new migration")
        .expect("declared");

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
    needs
        .declare("gone", PATH, "a new migration")
        .expect("declared");
    needs
        .declare("waiting", PATH, "a new migration")
        .expect("declared");
    assert_eq!(needs.behind("waiting").len(), 1);

    git(repo.path(), &["branch", "-D", "gone"]);
    assert!(needs.behind("waiting").is_empty());
    assert_eq!(needs.standing().len(), 1);
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
