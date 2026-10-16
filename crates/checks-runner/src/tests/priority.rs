//! At what priority a Check runs: agent work clamped beneath the merge line.
//!
//! **Read from `ps`, not from the argv.** What matters is the priority the
//! kernel schedules the Check's own processes at, and a wrapper that ran but
//! did not take would pass an argv test.

use std::path::Path;
use std::time::Duration;

use verification::{Exit, NeverRan};

use crate::run::{run_until, Writing};
use crate::Priority;

/// The scheduler priority `ps` reports for a Check's own shell.
const OWN_PRIORITY: &str = "/bin/sh -c 'ps -o pri= -p $$'";

/// Utility QoS's band; normal work is 31.
#[cfg(target_os = "macos")]
const UTILITY: u32 = 20;

async fn priority_of_a_check_at(priority: Priority) -> u32 {
    let ran = run_until(
        OWN_PRIORITY,
        Path::new("/"),
        Duration::from_secs(30),
        Writing::Nowhere,
        &[],
        std::future::pending(),
        priority,
    )
    .await;
    assert_eq!(ran.exit, Exit::Code(0), "{:?}", ran.output);
    ran.output
        .stdout
        .trim()
        .parse()
        .expect("ps printed a number")
}

/// This test's own priority: a suite run under a lowered Check is lowered
/// itself, and a normal child of it is no higher.
fn this_process() -> u32 {
    let printed = std::process::Command::new("ps")
        .args(["-o", "pri=", "-p", &std::process::id().to_string()])
        .output()
        .expect("ps runs");
    String::from_utf8_lossy(&printed.stdout)
        .trim()
        .parse()
        .expect("ps printed a number")
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn an_agents_check_runs_clamped_to_utility() {
    assert!(priority_of_a_check_at(Priority::Low).await <= UTILITY);
}

#[tokio::test]
async fn a_normal_check_runs_at_the_priority_of_whoever_started_it() {
    assert_eq!(
        priority_of_a_check_at(Priority::Normal).await,
        this_process()
    );
}

/// The clamp's wrapper must not turn a missing program into its own exit code.
#[tokio::test]
async fn a_lowered_check_naming_no_program_still_says_so() {
    let ran = run_until(
        "armada-no-such-program-anywhere",
        Path::new("/"),
        Duration::from_secs(10),
        Writing::Nowhere,
        &[],
        std::future::pending(),
        Priority::Low,
    )
    .await;
    assert!(
        matches!(ran.exit, Exit::NeverRan(NeverRan::NoSuchCommand { .. })),
        "{:?}",
        ran.exit
    );
}

/// `ARMADA_CHECK_PRIORITY`: only `normal` turns the lowering off.
#[test]
fn the_lowering_is_off_only_where_the_variable_says_normal() {
    assert_eq!(Priority::named(None), Priority::Low);
    assert_eq!(Priority::named(Some("")), Priority::Low);
    assert_eq!(Priority::named(Some("low")), Priority::Low);
    assert_eq!(Priority::named(Some("normal")), Priority::Normal);
    assert_eq!(Priority::named(Some(" Normal\n")), Priority::Normal);
}

/// Fleet's own `git` and `gh` are started as a command, not through `run`: the
/// clamp has to be on the command a caller then adds its arguments to.
#[cfg(target_os = "macos")]
#[test]
fn a_command_spawned_at_low_priority_runs_clamped_to_utility() {
    let printed = Priority::Low
        .spawning("/bin/sh")
        .args(["-c", "ps -o pri= -p $$"])
        .output()
        .expect("sh runs");
    let own: u32 = String::from_utf8_lossy(&printed.stdout)
        .trim()
        .parse()
        .expect("ps printed a number");
    assert!(own <= UTILITY, "{own}");
}

#[test]
fn a_command_spawned_at_normal_priority_is_the_program_itself() {
    let command = Priority::Normal.spawning("git");
    assert_eq!(command.get_program(), "git");
    assert_eq!(command.get_args().count(), 0);
}

/// Said by the operating system rather than by the wrapper exiting 66.
#[test]
fn a_command_spawned_at_low_priority_for_no_program_still_is_not_found() {
    let spawned = Priority::Low
        .spawning("armada-no-such-program-anywhere")
        .output();
    assert_eq!(
        spawned.expect_err("nothing to run").kind(),
        std::io::ErrorKind::NotFound
    );
}
