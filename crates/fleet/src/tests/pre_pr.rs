//! The pre-PR Checks run for an agent that goes to open a pull request, and what it is told.

use crate::session_host::pre_pr::{ran, ToRun};

fn check(name: &str, run: &str, expect: i64) -> ToRun {
    ToRun { name: name.to_string(), run: run.to_string(), dir: std::env::temp_dir(), expect }
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
