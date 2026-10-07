//! What a repair Drone is allowed to run, on the launch. `crate::healing` in
//! `fleet` is what grants it.

use adapter_traits::{Grant, Repair, Toolbelt};

use crate::tests::harness::{rendered, value_after};

fn entries(args: &[String], flag: &str) -> Vec<String> {
    value_after(args, flag)
        .expect("the flag is rendered")
        .split(',')
        .map(String::from)
        .collect()
}

/// Staging is allowed and its deny is lifted with it; a commit, a push, a reset
/// and a checkout are neither.
#[test]
fn repairing_the_index_grants_staging_and_nothing_that_writes_history() {
    let args = rendered(
        Toolbelt::evidence_only()
            .and(Grant::ReadTheWorktree)
            .and(Grant::RepairTheWorktree(Repair::TheIndex)),
    );
    let allowed = entries(&args, "--allowedTools");
    let denied = entries(&args, "--disallowedTools");
    for verb in ["add", "rm", "restore"] {
        assert!(
            allowed.contains(&format!("Bash(git {verb}:*)")),
            "{allowed:?}"
        );
        assert!(
            !denied.contains(&format!("Bash(git {verb}:*)")),
            "{denied:?}"
        );
    }
    for verb in ["commit", "reset", "checkout", "merge", "push"] {
        assert!(
            !allowed.contains(&format!("Bash(git {verb}:*)")),
            "{allowed:?}"
        );
        assert!(
            denied.contains(&format!("Bash(git {verb}:*)")),
            "{denied:?}"
        );
    }
}

/// The lift belongs to the grant: a launch without it denies staging as ever.
#[test]
fn a_launch_without_the_repair_still_denies_staging() {
    let args = rendered(Toolbelt::evidence_only().and(Grant::ReadTheRepository));
    let denied = entries(&args, "--disallowedTools");
    for verb in ["add", "rm", "restore"] {
        assert!(
            denied.contains(&format!("Bash(git {verb}:*)")),
            "{denied:?}"
        );
    }
}

/// The install is the repository's own command, as a prefix rule, so its flag
/// for a forced reinstall rides behind it.
#[test]
fn repairing_the_install_grants_the_declared_bootstrap_and_nothing_else() {
    let args = rendered(Toolbelt::evidence_only().and(Grant::RepairTheWorktree(
        Repair::TheInstall(String::from("pnpm install --frozen-lockfile")),
    )));
    let allowed = entries(&args, "--allowedTools");
    assert!(
        allowed.contains(&String::from("Bash(pnpm install --frozen-lockfile:*)")),
        "{allowed:?}"
    );
    assert!(!allowed.iter().any(|rule| rule.starts_with("Bash(git")));
}
