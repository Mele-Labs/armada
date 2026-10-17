//! What the session gate runs in the background, refuses, or leaves alone. #2415.

use crate::hosted_session::{check_line, CheckLine};

#[test]
fn a_check_beside_only_reads_runs_alone() {
    for alone in [
        "armada check test build_notices",
        "cd /repo/.armada/slots/slot-5 && armada check test x 2>&1 | tail -25",
        "git diff --name-only | armada check test --changed",
        "taskpolicy -c utility armada check test x",
        "./target/debug/armada check build",
        "armada manifest check",
    ] {
        assert_eq!(check_line(alone), CheckLine::Alone, "{alone}");
    }
}

/// Backgrounded, the commit would run unseen.
#[test]
fn a_check_beside_a_write_is_chained() {
    for chained in [
        "armada check test x && git commit -m y",
        "armada check build; cargo fmt",
    ] {
        assert_eq!(check_line(chained), CheckLine::Chained, "{chained}");
    }
}

#[test]
fn a_line_with_no_check_or_one_it_cannot_read_is_left_alone() {
    for none in [
        "armada worktree --status",
        "cargo test",
        "echo armada check",
        "armada check test x > out.txt",
    ] {
        assert_eq!(check_line(none), CheckLine::NoCheck, "{none}");
    }
}
