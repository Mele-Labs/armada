//! An asked run is a record of its own: written when it starts, updated when
//! it ends by any route, returned wherever the Job's record is read, and said
//! in the Job's log at both ends. Never a Check row.

use std::sync::Arc;
use std::time::Duration;

use core_model::{DroneId, JobId};
use ipc::mcp::ChecksAsk;

use crate::gate::CheckBudget;
use crate::tests::asked_run::{
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

fn by_the_drone(fleet: &super::Fixture, job: &JobId, drone: &DroneId) -> ipc::Requester {
    ipc::Requester::drone_on_step(
        &ipc::JobId::from(job),
        &ipc::StepId::carried("implement"),
        &ipc::DroneId::from(drone),
    )
    .with_handle(&futures_handle(fleet, job))
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
                .asked_run_on(&ipc::JobId::from(&job), &ipc::StepId::carried("implement"))
            {
                return shown;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the run is shown");
    assert_eq!(live.requester, by_the_drone(&fleet, &job, &drone));

    let _ = running.finished().await;
    let rows = kept(&fleet, &job).await;
    assert_eq!(rows.len(), 1, "the same row, not a second");
    assert_eq!(rows[0].state, store::AskedState::Passed);
    assert!(rows[0].finished_at.is_some());
    assert!(
        rows[0]
            .logs
            .iter()
            .any(|log| log.ends_with("implement.1.dry.0.log")),
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

    clock.doom_the_run_spawned_next();
    let running = fleet
        .run_checks(&job, ChecksAsk::everything(false))
        .await
        .expect("the run starts");
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
    assert_eq!(
        step.asked_runs[0].requester,
        by_the_drone(&fleet, &job, &drone)
    );
    assert_eq!(step.asked_runs[0].state, ipc::AskedRunState::Failed);
    assert!(step.check_runs.is_empty(), "never among the gate's rows");

    let list = fleet.rehearsal_history(&job).await.expect("the run list");
    assert_eq!(list.asked_runs, step.asked_runs);
    assert!(list.runs.is_empty());
}

/// A drone on a plan task is a `drone_task`, with the task and the Job's
/// handle named.
#[test]
fn a_run_on_a_task_names_the_task_and_the_handle() {
    let job = JobId::carried(core_model::Ulid::carried("01JOB"));
    let asked = crate::asked_run::asked::requester(
        &job,
        "7-a-job",
        &core_model::StepId::new("implement"),
        &DroneId::carried(core_model::Ulid::carried("01DRONE")),
        core_model::TaskId::read("T3"),
    );
    assert_eq!(asked.kind, "drone_task");
    assert_eq!(asked.task_id.as_deref(), Some("T3"));
    assert_eq!(asked.handle.as_deref(), Some("7-a-job"));
    assert_eq!(asked.drone_id.as_ref().map(|d| d.as_str()), Some("01DRONE"));
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
        for field in [
            "ran",
            "failed",
            "narrowed",
            "failed_checks",
            "drone",
            "task",
            "attempt",
        ] {
            assert!(
                line.contains(&format!("\"{field}\"")),
                "{field} missing: {line}"
            );
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
        assert!(
            !refused.to_lowercase().contains(never),
            "{never}: {refused}"
        );
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

/// **One read across the repository's Jobs**: the gate's rows and the Drone's
/// asked runs together, each with who asked, newest first, and a log the
/// existing reader opens — the asked run's included.
#[tokio::test]
async fn the_manifest_wide_read_carries_gate_rows_and_asked_runs_with_openable_logs() {
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
        .await;
    // A gate's row, written the way a ruling writes it.
    fleet
        .store()
        .lock()
        .await
        .record_step_checks(
            &job,
            &core_model::StepId::new("implement"),
            &[core_model::StepCheck {
                name: "suite".to_string(),
                outcome: core_model::CheckOutcome::Passed,
                expected: None,
                produced: None,
                output_path: Some(".armada/checks/the-job/implement.1.0.log".to_string()),
                reused_from_asked_run: None,
            }],
            &core_model::Timestamp::from_rfc3339("2099-01-01T00:00:00.000Z"),
        )
        .expect("the gate's row");

    let read = fleet.manifest_checks(None).await.expect("the read");
    assert_eq!(read.total, 2);
    assert!(!read.truncated);
    assert_eq!(read.rows[0].source, "gate", "newest first: {:?}", read.rows);
    assert_eq!(read.rows[0].requester.kind, "gate");
    assert_eq!(read.rows[0].state, "passed");
    assert_eq!(read.rows[0].took_ms, None, "a gate keeps no duration");
    assert_eq!(read.rows[0].logs[0].kept, "implement.1.0.log");

    let asked = &read.rows[1];
    assert_eq!(asked.source, "asked_run");
    assert_eq!(asked.requester, by_the_drone(&fleet, &job, &drone));
    assert_eq!(asked.state, "failed");
    assert_eq!(asked.name, "suite, diff_nonempty");
    assert_eq!(asked.job_id.as_str(), job.as_str());
    assert!(asked.started_at.is_some() && asked.ended_at.is_some());
    let log = asked
        .logs
        .iter()
        .find(|log| log.check == "suite")
        .expect("the Check's log is named");
    assert_eq!(log.kept, "implement.1.dry.0.log");

    use api::Queries;
    // **The existing reader opens it**, with the Check's own name on it.
    let opened = fleet
        .get_check_output(asked.job_id.clone(), log.kept.clone())
        .await
        .expect("the asked run's log opens");
    assert_eq!(opened.name, "suite");
    assert_eq!(opened.attempt, 1);
}

#[test]
fn an_answer_is_cut_to_the_newest_and_says_so() {
    let row = |at: &str| ipc::ManifestCheckRow {
        source: "gate".to_string(),
        requester: ipc::Requester::outside(),
        job_id: ipc::JobId::carried("01JOB"),
        job_handle: "1-a".to_string(),
        job_title: "a".to_string(),
        step: ipc::StepId::carried("implement"),
        attempt: 1,
        group: None,
        name: "suite".to_string(),
        state: "passed".to_string(),
        started_at: None,
        ended_at: Some(ipc::Instant::carried(at)),
        took_ms: None,
        logs: Vec::new(),
        asked_run_id: None,
    };
    let rows = vec![
        row("2026-10-01T00:00:00Z"),
        row("2026-10-03T00:00:00Z"),
        row("2026-10-02T00:00:00Z"),
    ];
    let read = crate::manifest_checks::newest(rows, 2);
    assert_eq!(read.total, 3);
    assert!(read.truncated);
    let at: Vec<&str> = read
        .rows
        .iter()
        .map(|row| row.ended_at.as_ref().expect("an end").as_str())
        .collect();
    assert_eq!(at, ["2026-10-03T00:00:00Z", "2026-10-02T00:00:00Z"]);
}
