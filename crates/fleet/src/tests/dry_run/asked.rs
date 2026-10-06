//! An asked run is a record of its own: written when it starts, updated when
//! it ends by any route, returned wherever the Job's record is read, and said
//! in the Job's log at both ends. Never a Check row.

use std::sync::Arc;
use std::time::Duration;

use core_model::{DroneId, JobId};
use ipc::mcp::ChecksAsk;

use crate::gate::CheckBudget;
use crate::tests::dry_run::{
    a_fleet_budgeted, a_fleet_checking, a_quiet_drone, one_step, started, the_one_drone, Held,
};
use crate::tests::tmp::TempDir;

async fn kept(fleet: &super::Fixture, job: &JobId) -> Vec<store::AskedRun> {
    fleet
        .store()
        .lock()
        .await
        .asked_runs(job, fleet.run())
        .expect("the asked runs read")
}

fn the_log(home: &TempDir, fleet: &super::Fixture, job: &JobId) -> String {
    let handle = futures_handle(fleet, job);
    std::fs::read_to_string(crate::transcript::log_of(
        &home.path().to_string_lossy(),
        &handle,
    ))
    .expect("the Job's own log")
}

fn futures_handle(fleet: &super::Fixture, job: &JobId) -> String {
    fleet.name_of(job).expect("the Job has a handle")
}

fn by_the_drone(job: &JobId, drone: &DroneId) -> ipc::Requester {
    ipc::Requester::drone_on_step(
        &ipc::JobId::from(job),
        &ipc::StepId::carried("implement"),
        &ipc::DroneId::from(drone),
    )
}

/// **The row exists while the run goes**, saying who asked and what for, and
/// the live view says the same; it is closed when the run ends, with the log.
#[tokio::test]
async fn an_asked_run_is_a_row_from_its_start_and_closed_when_it_ends() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_checking(
        &home,
        one_step("/bin/sleep 1"),
        Arc::new(Held::started()),
        3,
    ));
    started(&fleet, &home).await;
    let (job, drone) = the_one_drone(&fleet).await.expect("a Drone at work");

    let running = fleet
        .run_checks(&job, ChecksAsk::everything(false))
        .await
        .expect("the run starts");

    let rows = kept(&fleet, &job).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].state, store::AskedState::Running);
    assert_eq!(rows[0].drone, drone);
    assert_eq!(rows[0].step.as_str(), "implement");
    assert_eq!(rows[0].attempt, 1);
    assert_eq!(rows[0].task, None);
    assert!(!rows[0].narrowed);
    assert_eq!(rows[0].only_check, None);
    assert_eq!(rows[0].checks, ["suite", "diff_nonempty"]);
    assert_eq!(rows[0].finished_at, None);

    let live = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(shown) = fleet
                .underway()
                .dry_run_on(&ipc::JobId::from(&job), &ipc::StepId::carried("implement"))
            {
                return shown;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the run is shown");
    assert_eq!(live.requester, by_the_drone(&job, &drone));

    let _ = running.finished().await;
    let rows = kept(&fleet, &job).await;
    assert_eq!(rows.len(), 1, "the same row, not a second");
    assert_eq!(rows[0].state, store::AskedState::Passed);
    assert!(rows[0].finished_at.is_some());
    assert!(
        rows[0].logs.iter().any(|log| log.ends_with("implement.1.dry.0.log")),
        "{:?}",
        rows[0].logs
    );

    let gate_rows = fleet
        .store()
        .lock()
        .await
        .step_checks_every_attempt(&job)
        .expect("the gate's rows");
    assert!(gate_rows.is_empty(), "an asked run is never a Check row");
}

#[tokio::test]
async fn a_run_that_found_a_failure_is_closed_as_failed_and_names_the_checks_it_ran() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_checking(
        &home,
        one_step("/usr/bin/false"),
        Arc::new(Held::started()),
        3,
    ));
    started(&fleet, &home).await;
    let (job, _) = the_one_drone(&fleet).await.expect("a Drone at work");

    let _ = fleet
        .run_checks(&job, ChecksAsk::everything(false))
        .await
        .expect("the run starts")
        .finished()
        .await
        .expect("the run ended");

    let rows = kept(&fleet, &job).await;
    assert_eq!(rows[0].state, store::AskedState::Failed);
}

#[tokio::test]
async fn a_run_cut_off_by_its_time_box_is_closed_as_stopped() {
    use crate::places::{Asking, ChecksAtOnce};
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_budgeted(
        &home,
        one_step("/bin/true"),
        Arc::new(Held::started()),
        3,
        &["src/parse.rs"],
        a_quiet_drone(),
        CheckBudget::of(Duration::from_millis(200)).waiting(Duration::from_millis(200)),
    ));
    fleet.rechecked(ChecksAtOnce::of(1));
    started(&fleet, &home).await;
    let (job, _) = the_one_drone(&fleet).await.expect("a Drone at work");
    let held = fleet.room(Asking::Gate).place().await;

    let _ = fleet
        .run_checks(&job, ChecksAsk::everything(false))
        .await
        .expect("the run starts")
        .finished()
        .await;
    drop(held);

    let rows = kept(&fleet, &job).await;
    assert_eq!(rows[0].state, store::AskedState::Stopped);
    assert!(rows[0].finished_at.is_some());
}

#[tokio::test]
async fn a_run_whose_task_died_is_closed_as_lost() {
    let home = TempDir::new();
    let clock = Arc::new(Held::started());
    let fleet = Arc::new(a_fleet_checking(
        &home,
        one_step("/bin/true"),
        Arc::clone(&clock),
        3,
    ));
    started(&fleet, &home).await;
    let (job, _) = the_one_drone(&fleet).await.expect("a Drone at work");

    let running = fleet
        .run_checks(&job, ChecksAsk::everything(false))
        .await
        .expect("the run starts");
    clock.doom_next_reading();
    let _ = running.finished().await;

    let rows = kept(&fleet, &job).await;
    assert_eq!(rows[0].state, store::AskedState::Lost);
}

/// **Read wherever the Job's record is read**: the step's detail, and the run
/// list, each with the requester on the row.
#[tokio::test]
async fn the_job_detail_and_the_run_list_return_the_asked_run() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_checking(
        &home,
        one_step("/usr/bin/false"),
        Arc::new(Held::started()),
        3,
    ));
    started(&fleet, &home).await;
    let (job, drone) = the_one_drone(&fleet).await.expect("a Drone at work");
    let _ = fleet
        .run_checks(&job, ChecksAsk::everything(false))
        .await
        .expect("the run starts")
        .finished()
        .await
        .expect("the run ended");

    let detail = fleet
        .job_detail(ipc::JobId::from(&job))
        .await
        .expect("the detail");
    let step = detail
        .steps
        .iter()
        .find(|step| step.step_id.as_str() == "implement")
        .expect("the step");
    assert_eq!(step.asked_runs.len(), 1);
    assert_eq!(step.asked_runs[0].requester, by_the_drone(&job, &drone));
    assert_eq!(step.asked_runs[0].state, ipc::AskedRunState::Failed);
    assert!(step.check_runs.is_empty(), "never among the gate's rows");

    let list = fleet.rehearsal_history(&job).await.expect("the run list");
    assert_eq!(list.asked_runs, step.asked_runs);
    assert!(list.runs.is_empty());
}

/// A drone on a plan task is a `drone_task`, with the task named.
#[test]
fn a_run_on_a_task_names_the_task() {
    let job = JobId::carried(core_model::Ulid::carried("01JOB"));
    let run = store::AskedRun {
        id: 7,
        drone: DroneId::carried(core_model::Ulid::carried("01DRONE")),
        task: core_model::TaskId::read("T3"),
        step: core_model::StepId::new("implement"),
        attempt: 2,
        started_at: core_model::Timestamp::from_rfc3339("2026-10-06T10:00:00.000Z"),
        finished_at: None,
        state: store::AskedState::Running,
        checks: vec!["suite".to_string()],
        narrowed: true,
        only_check: Some("suite".to_string()),
        logs: Vec::new(),
    };
    let wired = crate::dry_run::asked::wired(&job, &run);
    assert_eq!(wired.requester.kind, "drone_task");
    assert_eq!(wired.requester.task_id.as_deref(), Some("T3"));
    assert_eq!(wired.requester.drone_id.as_ref().map(|d| d.as_str()), Some("01DRONE"));
    assert_eq!(wired.state, ipc::AskedRunState::Running);
    assert_eq!(wired.attempt, 2);
    assert_eq!(wired.only_check.as_deref(), Some("suite"));
}

/// **A line when it starts and a line when it ends**, the same fields on
/// both, and who asked on each.
#[tokio::test]
async fn the_log_says_when_a_run_started_and_when_it_ended_and_who_asked() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_checking(
        &home,
        one_step("/usr/bin/false"),
        Arc::new(Held::started()),
        3,
    ));
    started(&fleet, &home).await;
    let (job, drone) = the_one_drone(&fleet).await.expect("a Drone at work");
    let _ = fleet
        .run_checks(&job, ChecksAsk::everything(false))
        .await
        .expect("the run starts")
        .finished()
        .await
        .expect("the run ended");

    let log = the_log(&home, &fleet, &job);
    let started_line = log
        .lines()
        .find(|line| line.contains(crate::retro::lines::A_DRONE_STARTED_CHECKS))
        .expect("a line for the start");
    let ended_line = log
        .lines()
        .find(|line| line.contains(crate::retro::lines::A_DRONE_RAN_CHECKS))
        .expect("a line for the end");
    for line in [started_line, ended_line] {
        for field in ["ran", "failed", "narrowed", "failed_checks", "drone", "task", "attempt"] {
            assert!(line.contains(&format!("\"{field}\"")), "{field} missing: {line}");
        }
        assert!(line.contains(drone.as_str()), "{line}");
    }
}

/// **A refusal says how long the run has been going and which Check it is on**,
/// still tells the Drone to wait, and never tells it to look again.
#[tokio::test]
async fn a_second_ask_is_told_how_long_the_run_has_gone_and_where_it_is() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_checking(
        &home,
        one_step("/bin/sleep 2"),
        Arc::new(Held::started()),
        3,
    ));
    started(&fleet, &home).await;
    let (job, _) = the_one_drone(&fleet).await.expect("a Drone at work");
    let running = fleet
        .run_checks(&job, ChecksAsk::everything(false))
        .await
        .expect("the run starts");

    let refused = loop {
        let refused = fleet
            .run_checks(&job, ChecksAsk::everything(false))
            .await
            .expect_err("one is going");
        if refused.to_string().contains("`suite`") {
            break refused.to_string();
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    };
    assert!(refused.contains("going for"), "{refused}");
    assert!(refused.contains("Wait for their report"), "{refused}");
    assert!(refused.contains("later turn"), "{refused}");
    for never in ["poll", "check again", "ask again", "status"] {
        assert!(!refused.to_lowercase().contains(never), "{never}: {refused}");
    }
    let _ = running.finished().await;
}

#[tokio::test]
async fn a_second_ask_says_when_the_run_is_waiting_for_a_check_slot() {
    use crate::places::{Asking, ChecksAtOnce};
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet_budgeted(
        &home,
        one_step("/bin/true"),
        Arc::new(Held::started()),
        3,
        &["src/parse.rs"],
        a_quiet_drone(),
        CheckBudget::of(Duration::from_secs(5)).waiting(Duration::from_secs(30)),
    ));
    fleet.rechecked(ChecksAtOnce::of(1));
    started(&fleet, &home).await;
    let (job, _) = the_one_drone(&fleet).await.expect("a Drone at work");
    let held = fleet.room(Asking::Gate).place().await;
    fleet
        .run_checks(&job, ChecksAsk::everything(false))
        .await
        .expect("the run starts");

    let refused = tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let refused = fleet
                .run_checks(&job, ChecksAsk::everything(false))
                .await
                .expect_err("one is going")
                .to_string();
            if refused.contains("waiting for a Check slot") {
                return refused;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("the refusal never said it was waiting for a slot");
    assert!(refused.contains("later turn"), "{refused}");
    drop(held);
}
