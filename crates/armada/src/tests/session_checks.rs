//! What `armada check` tells Fleet of a run as it ends. **A run killed for its budget reads timed out, never failed**:
//! its log may hold only a compiler warning, and an agent told "failed" goes hunting for a failing test that is not there.

use std::time::Duration;

use checks_runner::{Attempt, Output};
use ipc::SessionCheckState;
use verification::Exit;

use crate::declared::Ran;
use crate::session_checks::ending;

fn ran(exit: Exit, stderr: &str) -> Ran {
    Ran {
        name: "test".into(),
        command: "cargo nextest run".into(),
        test: None,
        destructive: false,
        required: Vec::new(),
        attempt: Attempt { exit, output: Output { stdout: String::new(), stderr: stderr.into(), truncated: false } },
        narrowed: None,
        target: None,
        mapped: crate::test_map::Mapped::default(),
    }
}

#[test]
fn a_run_killed_for_its_budget_is_told_as_timed_out_and_its_log_says_so() {
    let warning = "warning: unused import: `PinAppRunPort`\n";
    let (outcome, log) = ending(&ran(Exit::TimedOut { after: Duration::from_secs(900) }, warning));
    assert_eq!(outcome, SessionCheckState::TimedOut);
    assert!(log.contains(warning), "what it printed is kept: {log}");
    assert!(log.contains("ran past its 900s budget and was killed"), "the log says why it ended: {log}");
}

#[test]
fn a_run_that_exits_nonzero_still_fails_and_one_that_exits_zero_passes() {
    assert_eq!(ending(&ran(Exit::Code(101), "")).0, SessionCheckState::Failed);
    assert_eq!(ending(&ran(Exit::Code(0), "")).0, SessionCheckState::Passed);
    assert!(!ending(&ran(Exit::Code(101), "")).1.contains("budget"), "a plain failure says nothing of a budget");
}
