//! An asked run is kept as its own row, from the moment it starts, and a run
//! the last Fleet left "running" reads as lost.

use core_model::{DroneId, TaskId, Ulid};

use crate::tests::attempt::{on_its_first_run, step_id};
use crate::tests::{at, job_id, open, TempDir};
use crate::{AskedRunBegun, AskedState};

fn begun(task: Option<&str>) -> AskedRunBegun {
    AskedRunBegun {
        drone: DroneId::carried(Ulid::carried("01DRONE")),
        task: task.map(|task| TaskId::read(task).expect("a task id")),
        step: step_id(),
        attempt: 1,
        fleet: Ulid::carried("01FLEETONE"),
        at: at("2026-10-06T10:00:00.000Z"),
        checks: vec!["suite".to_string(), "lint".to_string()],
        narrowed: true,
        only_check: Some("suite".to_string()),
    }
}

#[test]
fn a_run_is_written_when_it_starts_and_updated_when_it_ends() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01ASKED");
    let job = job_id("01ASKED");
    let fleet = Ulid::carried("01FLEETONE");

    let id = store
        .begin_asked_run(&job, &begun(Some("T2")))
        .expect("the start is kept");
    let started = store.asked_runs(&job, &fleet).expect("it reads back");
    assert_eq!(started.len(), 1);
    assert_eq!(started[0].state, AskedState::Running);
    assert_eq!(started[0].finished_at, None);
    assert_eq!(started[0].drone.as_str(), "01DRONE");
    assert_eq!(started[0].task.map(|task| task.to_string()), Some("T2".to_string()));
    assert_eq!(started[0].checks, ["suite", "lint"]);
    assert!(started[0].narrowed);
    assert_eq!(started[0].only_check.as_deref(), Some("suite"));

    store
        .end_asked_run(
            id,
            AskedState::Failed,
            &at("2026-10-06T10:01:30.000Z"),
            &["records/implement.1.dry.0.log".to_string()],
        )
        .expect("the end is kept");
    drop(store);

    let store = open(&dir);
    let ended = store.asked_runs(&job, &fleet).expect("it reads back");
    assert_eq!(ended[0].state, AskedState::Failed);
    assert_eq!(
        ended[0].finished_at,
        Some(at("2026-10-06T10:01:30.000Z"))
    );
    assert_eq!(ended[0].logs, ["records/implement.1.dry.0.log"]);
}

#[test]
fn a_run_still_running_for_a_fleet_that_is_gone_reads_as_lost() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01ASKED");
    let job = job_id("01ASKED");
    store
        .begin_asked_run(&job, &begun(None))
        .expect("the start is kept");
    drop(store);

    let store = open(&dir);
    let same = store
        .asked_runs(&job, &Ulid::carried("01FLEETONE"))
        .expect("it reads back");
    assert_eq!(same[0].state, AskedState::Running, "its own Fleet still runs it");
    let later = store
        .asked_runs(&job, &Ulid::carried("01FLEETTWO"))
        .expect("it reads back");
    assert_eq!(later[0].state, AskedState::Lost);
    assert_eq!(later[0].finished_at, None, "nobody saw it end");
}
