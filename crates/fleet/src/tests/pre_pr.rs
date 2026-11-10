//! The pre-PR Checks run for an agent that goes to open a pull request, and what it is told.

use checks_runner::CheckWidth;

use crate::session_host::pre_pr::{ran, ran_in, ToRun};

fn check(name: &str, run: &str, expect: i64) -> ToRun {
    ToRun { name: name.to_string(), run: run.to_string(), dir: std::env::temp_dir(), expect, width: CheckWidth::of(1) }
}

/// **A Check's `${width}` is filled before it runs**, as at a gate: nextest refuses a bare
/// `--test-threads ${width}`, and so did every pull request reaching a Check that wrote one.
#[tokio::test]
async fn a_checks_width_is_filled_before_it_runs() {
    let mut wide = check("crates/bridge-native:test", "test \"${width}\" = 3", 0);
    wide.width = CheckWidth::of(3);
    let answered = ran(vec![wide]).await;
    assert!(answered.passed, "{}", answered.says);
}

#[tokio::test]
async fn checks_that_pass_say_nothing_and_let_the_pull_request_open() {
    let answered = ran(vec![check("format_check", "true", 0)]).await;
    assert!(answered.passed);
    assert_eq!(answered.says, "");
    assert_eq!(answered.checks.len(), 1);
}

/// **The agent is told which Check failed and what it printed**, and a Check after the failing one
/// still runs so both are in one answer.
#[tokio::test]
async fn a_failing_check_is_named_with_its_log_tail_and_the_rest_still_run() {
    let answered = ran(vec![
        check("apps/desktop:format_check", "ls /armada-no-such-dir-a", 0),
        check("lint", "true", 0),
        check("typecheck", "ls /armada-no-such-dir-b", 0),
    ])
    .await;
    assert!(!answered.passed);
    assert!(answered.says.contains("apps/desktop:format_check failed:"), "{}", answered.says);
    assert!(answered.says.contains("armada-no-such-dir-a"), "{}", answered.says);
    assert!(answered.says.contains("typecheck failed:"), "{}", answered.says);
    assert!(answered.says.contains("armada-no-such-dir-b"), "{}", answered.says);
    assert!(!answered.says.contains("lint failed"), "{}", answered.says);
    assert_eq!(answered.checks.iter().map(|one| one.passed).collect::<Vec<_>>(), [false, true, false]);
}

/// A Check that expects a non-zero exit passes on it, as at a gate.
#[tokio::test]
async fn the_exit_a_check_expects_is_the_one_that_passes() {
    let answered = ran(vec![check("expects_one", "false", 1)]).await;
    assert!(answered.passed, "{}", answered.says);
}

/// A Check that cannot start has not passed.
#[tokio::test]
async fn a_check_that_never_ran_has_not_passed() {
    let answered = ran(vec![check("missing", "armada-no-such-program-here", 0)]).await;
    assert!(!answered.passed);
}

#[tokio::test]
async fn no_check_reached_is_a_pass() {
    assert!(ran(Vec::new()).await.passed);
}

fn git(root: &std::path::Path, args: &[&str]) {
    let done = std::process::Command::new("git")
        .args(["-c", "user.name=armada", "-c", "user.email=armada@example.invalid"])
        .args(args)
        .current_dir(root)
        .output()
        .expect("git runs");
    assert!(done.status.success(), "{args:?}: {}", String::from_utf8_lossy(&done.stderr));
}

fn repo() -> crate::tests::tmp::TempDir {
    let slot = crate::tests::tmp::TempDir::new();
    git(slot.path(), &["init", "-q", "-b", "main"]);
    std::fs::write(slot.path().join("a.ts"), "x\n").unwrap();
    std::fs::write(slot.path().join(".gitignore"), "ignored.log\n").unwrap();
    git(slot.path(), &["add", "."]);
    git(slot.path(), &["commit", "-q", "-m", "first"]);
    slot
}

/// **A formatted file the agent did not commit would pass the Check and miss the pull request**, so
/// no Check runs and the agent is told which paths to commit.
#[tokio::test]
async fn uncommitted_changes_hold_the_checks_and_name_the_paths() {
    let slot = repo();
    std::fs::write(slot.path().join("a.ts"), "formatted\n").unwrap();
    std::fs::write(slot.path().join("new.ts"), "new\n").unwrap();
    let marker = slot.path().join("ran-marker");
    let touch = format!("touch {}", marker.display());
    let answered = ran_in(slot.path().to_path_buf(), vec![check("format_check", &touch, 0)]).await;

    assert!(!answered.passed);
    assert!(answered.checks.is_empty());
    assert!(answered.says.contains("Commit"), "{}", answered.says);
    assert!(answered.says.contains("a.ts") && answered.says.contains("new.ts"), "{}", answered.says);
    assert!(!marker.exists(), "no Check runs on a dirty slot");
}

#[tokio::test]
async fn no_more_than_ten_dirty_paths_are_listed() {
    let slot = repo();
    for n in 0..13 {
        std::fs::write(slot.path().join(format!("f{n:02}.ts")), "x\n").unwrap();
    }
    let answered = ran_in(slot.path().to_path_buf(), vec![check("lint", "true", 0)]).await;

    assert_eq!(answered.says.matches(".ts\n").count(), 10, "{}", answered.says);
    assert!(answered.says.contains("and 3 more"), "{}", answered.says);
}

#[tokio::test]
async fn a_committed_slot_runs_its_checks_and_an_ignored_file_does_not_hold_them() {
    let slot = repo();
    std::fs::write(slot.path().join("ignored.log"), "noise\n").unwrap();
    let answered = ran_in(slot.path().to_path_buf(), vec![check("lint", "true", 0)]).await;

    assert!(answered.passed, "{}", answered.says);
    assert_eq!(answered.checks.len(), 1);
}

/// With no Check reached there is nothing the uncommitted files could slip past.
#[tokio::test]
async fn a_dirty_slot_with_no_check_reached_still_passes() {
    let slot = repo();
    std::fs::write(slot.path().join("a.ts"), "changed\n").unwrap();

    assert!(ran_in(slot.path().to_path_buf(), Vec::new()).await.passed);
}
