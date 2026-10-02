//! A path held off a Drone because another Job is fixing a test that lives in
//! it, denied on the launch. #1673.

use adapter_traits::{AgentHarness, Grant, Toolbelt};

use crate::harness::{HarnessRefused, HeadlessAgent};
use crate::tests::harness::{config, rendered, value_after};

/// The file, and everything under it in case it is a directory, beside the
/// git rules every launch carries; a Drone holding nothing gets none.
#[test]
fn a_held_off_path_is_denied_to_every_tool_that_edits() {
    let args = rendered(
        Toolbelt::evidence_only()
            .and(Grant::ChangeTheWorktree)
            .holding_off("src/parse.rs"),
    );
    let denied = value_after(&args, "--disallowedTools").expect("a deny list is rendered");
    let entries: Vec<&str> = denied.split(',').collect();
    for rule in [
        "Edit(./src/parse.rs)",
        "Edit(./src/parse.rs/**)",
        "Bash(git push:*)",
    ] {
        assert!(entries.contains(&rule), "missing `{rule}`: {denied}");
    }

    let free = rendered(Toolbelt::evidence_only().and(Grant::ChangeTheWorktree));
    let denied = value_after(&free, "--disallowedTools").expect("a deny list is rendered");
    assert!(!denied.contains("Edit("), "{denied}");
}

/// A path the rule cannot carry stops the launch rather than leaving it out.
#[test]
fn a_held_off_path_the_rule_cannot_carry_is_refused_not_dropped() {
    for path in ["src/a,b.rs", "src/(gen).rs", "/etc/hosts", ""] {
        let refused = HeadlessAgent::at("/usr/local/bin/agent")
            .render(&config(Toolbelt::evidence_only().holding_off(path)));
        assert!(
            matches!(
                refused,
                Err(HarnessRefused::HeldOffNotExpressibleAsARule { .. })
            ),
            "`{path}` was not refused: {refused:?}"
        );
    }
}
