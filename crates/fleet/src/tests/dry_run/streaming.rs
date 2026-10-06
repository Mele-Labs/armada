//! A Drone's run told a result at a time, stopped at its first failure, and
//! started fastest first once the repository has history. #1062.
//!
//! Real commands with real sleeps, for `later`'s reason: every claim here is
//! about which process ends when.

use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::sync::Arc;
use std::time::Duration;

use config::ResolvedWorkflow;
use core_model::{DroneId, JobId};
use testkit::{FakeJudge, FakeWorkProduct, Gate, Sketch};

use crate::converging::StepNorms;
use crate::daemon::Fleet;
use crate::dry_run::DryRuns;
use crate::silence::Liveness;
use crate::tests::daemon::{fitted_with, one};
use crate::tests::dry_run::{
    a_quiet_drone, checks_in, started, the_one_drone, transcript, Fixture, Held,
    A_CHECK_RUN_HAS_LONG_ENOUGH, QUIET_AFTER, WALL_CLOCK,
};
use crate::tests::tmp::TempDir;

/// A Fleet whose step is gated on `workflow`'s Checks, `at_once` at a time.
fn a_fleet_at_once(home: &TempDir, workflow: ResolvedWorkflow, at_once: usize) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]).showing("+    let x = 1;\n"),
        a_quiet_drone(),
    );
    fittings.starting().workflows = one(workflow);
    let clock: Arc<Held> = Arc::new(Held::started());
    fittings.clock = clock;
    fittings.liveness = Liveness::of(QUIET_AFTER, 2);
    fittings.norms = StepNorms::of(60, WALL_CLOCK, Duration::from_secs(120));
    fittings.dry_runs = DryRuns::of(3);
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about a dry run"));
    fittings.checks_at_once = crate::ChecksAtOnce::of(at_once);
    Fleet::assembled(fittings)
}

fn checked_by(gates: &[Gate<'_>]) -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates,
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

fn check<'a>(name: &'a str, run: &'a str) -> Gate<'a> {
    Gate::Check {
        name,
        run,
        expect_exit_code: 0,
        when: &[],
    }
}

/// An executable script under `home`, so how long a Check takes is the case's.
fn script(home: &TempDir, name: &str, body: &str) -> String {
    let path = home.path().join(name);
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("the script writes");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .expect("the script can run");
    path.display().to_string()
}

/// Every Checks turn this Drone has been told, once `runs` runs have ended.
async fn told_until_over(
    fleet: &Fixture,
    home: &TempDir,
    job: &JobId,
    drone: &DroneId,
    runs: usize,
) -> Vec<String> {
    let over = |said: &str| {
        checks_in(said)
            .iter()
            .filter(|turn| turn.contains("The run is over"))
            .count()
            >= runs
    };
    let said = transcript(fleet, home, job, drone)
        .await
        .until(A_CHECK_RUN_HAS_LONG_ENOUGH, over)
        .await
        .expect("the run never said it was over");
    checks_in(&said)
}

/// **The first definition of done.** The build fails and the Drone hears it
/// then: `slow` never ends on its own, so a run that is over did not wait for
/// it. It is stopped, and nothing it never finished is kept as a pass for the
/// gate to reuse (#1014).
///
/// **Ordered by the processes, not the clock.** Two places, so `build` starts
/// only once `quick` is told passed; `build` fails only once `slow` holds the
/// FIFO open; and an open with no reader left is `ENXIO`, so the end proves
/// nothing of `slow` survived the stop.
#[tokio::test]
async fn a_build_that_fails_reaches_the_drone_at_once_and_stops_the_slower_checks() {
    let home = TempDir::new();
    let held = home.path().join("slow-holds");
    let made = std::process::Command::new("/usr/bin/mkfifo")
        .arg(&held)
        .status()
        .expect("mkfifo runs");
    assert!(made.success(), "the FIFO is made");
    let slow = script(
        &home,
        "slow",
        &format!("exec 3<>\"{}\"\nread line <&3", held.display()),
    );
    // Opening a FIFO to write waits for a reader, so this fails only once `slow` is running.
    let build = script(
        &home,
        "build",
        &format!("/usr/bin/true > \"{}\" || exit 0\nexit 1", held.display()),
    );
    let gates = [
        check("slow", &slow),
        check("quick", "/usr/bin/true"),
        check("build", &build),
    ];
    let fleet = Arc::new(a_fleet_at_once(&home, checked_by(&gates), 2));
    started(&fleet, &home).await;
    let (job, drone) = the_one_drone(&fleet).await.expect("a Drone at work");

    let underway = fleet
        .run_checks(&job, ipc::mcp::ChecksAsk::everything(false))
        .await
        .expect("the run starts");
    let told = told_until_over(&fleet, &home, &job, &drone, 1).await;
    let last = told.last().expect("a last turn");
    for part in [
        "`build` did not pass",
        "stopped when `build` did not pass",
        "were stopped before they finished",
    ] {
        assert!(last.contains(part), "the last turn has no {part}: {last}");
    }
    assert!(matches!(underway.finished().await, Some(Ok(_))));

    let kept = fleet
        .the_only_slot()
        .await
        .lock()
        .await
        .as_ref()
        .and_then(|at_work| at_work.dry_run_kept().cloned())
        .expect("a finished run is kept");
    assert!(
        kept.passed("slow").is_none(),
        "a Check the failure stopped was kept as a pass"
    );
    assert!(kept.passed("build").is_none());
    assert!(
        kept.passed("quick").is_some(),
        "a pass before the failure is still a pass"
    );

    let still_read = std::fs::OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(&held);
    assert_eq!(
        still_read.err().and_then(|error| error.raw_os_error()),
        Some(libc::ENXIO),
        "the slow Check ran on after the build failed"
    );
}

/// A result that lands while another still runs is its own turn, and the last
/// turn is the report.
#[tokio::test]
async fn each_result_that_lands_while_others_run_is_its_own_turn() {
    let home = TempDir::new();
    let gates = [
        check("quick", "/usr/bin/true"),
        check("slow", "/bin/sleep 1"),
    ];
    let fleet = Arc::new(a_fleet_at_once(&home, checked_by(&gates), 2));
    started(&fleet, &home).await;
    let (job, drone) = the_one_drone(&fleet).await.expect("a Drone at work");

    let underway = fleet
        .run_checks(&job, ipc::mcp::ChecksAsk::everything(false))
        .await
        .expect("the run starts");
    assert!(matches!(underway.finished().await, Some(Ok(_))));
    let told = told_until_over(&fleet, &home, &job, &drone, 1).await;
    assert_eq!(
        told.len(),
        2,
        "one turn per result, then the report: {told:?}"
    );
    assert!(
        told[0].contains("`quick` passed") && told[0].contains("Still going: `slow`"),
        "{}",
        told[0]
    );
    assert!(
        told[1].contains("The run is over") && told[1].contains("2 of 2 passed"),
        "{}",
        told[1]
    );
}

/// **The third definition of done.** A run keeps how long each Check took, and
/// the next run starts the one that took least first — here one slot, so the
/// first result told is whichever started first.
#[tokio::test]
async fn once_the_repository_has_history_the_fastest_check_starts_first() {
    let home = TempDir::new();
    let gates = [
        check("slow", "/bin/sleep 0.5"),
        check("quick", "/usr/bin/true"),
    ];
    let fleet = Arc::new(a_fleet_at_once(&home, checked_by(&gates), 1));
    started(&fleet, &home).await;
    let (job, drone) = the_one_drone(&fleet).await.expect("a Drone at work");

    let first = fleet
        .run_checks(&job, ipc::mcp::ChecksAsk::everything(false))
        .await
        .expect("the first run");
    assert!(matches!(first.finished().await, Some(Ok(_))));
    let told = told_until_over(&fleet, &home, &job, &drone, 1).await;
    assert!(
        told[0].contains("`slow` passed"),
        "with no history the Manifest's order holds: {told:?}"
    );

    let record = fleet.load(&job).await.expect("the Job");
    let timed = fleet
        .store()
        .lock()
        .await
        .check_timings(record.owner_manifest_id())
        .expect("the timings read");
    assert!(
        timed.get("slow") > timed.get("quick"),
        "the run's durations were not kept: {timed:?}"
    );

    let second = fleet
        .run_checks(&job, ipc::mcp::ChecksAsk::everything(false))
        .await
        .expect("the second run");
    assert!(matches!(second.finished().await, Some(Ok(_))));
    let told = told_until_over(&fleet, &home, &job, &drone, 2).await;
    assert!(
        told[2].contains("`quick` passed") && told[2].contains("Still going: `slow`"),
        "the fastest Check did not start first: {told:?}"
    );
}

/// **The history is a record, not a dependency.** A run appends one row per
/// Check that reached a code, and a store that will not take the row leaves the
/// run's answer and its timings as they were.
#[tokio::test]
async fn a_run_appends_to_the_check_history_and_a_store_that_will_not_fails_nothing() {
    let home = TempDir::new();
    let gates = [
        check("slow", "/bin/sleep 0.2"),
        check("quick", "/usr/bin/true"),
    ];
    let fleet = Arc::new(a_fleet_at_once(&home, checked_by(&gates), 1));
    started(&fleet, &home).await;
    let (job, _drone) = the_one_drone(&fleet).await.expect("a Drone at work");
    let record = fleet.load(&job).await.expect("the Job");
    let repository = record.owner_manifest_id();

    let first = fleet
        .run_checks(&job, ipc::mcp::ChecksAsk::everything(false))
        .await
        .expect("the first run");
    assert!(matches!(first.finished().await, Some(Ok(_))));
    assert_eq!(
        fleet
            .store()
            .lock()
            .await
            .check_runs_kept(repository)
            .expect("counts"),
        2,
        "one row per Check that ran"
    );

    fleet
        .store()
        .lock()
        .await
        .without_the_check_run_history()
        .expect("dropped");
    let second = fleet
        .run_checks(&job, ipc::mcp::ChecksAsk::everything(false))
        .await
        .expect("the second run");
    assert!(
        matches!(second.finished().await, Some(Ok(_))),
        "a history that would not write failed the run"
    );
    let timed = fleet
        .store()
        .lock()
        .await
        .check_timings(repository)
        .expect("the timings read");
    assert_eq!(timed.len(), 2, "the timings were lost with the history");
}
