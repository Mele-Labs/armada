//! Which Drone was put on which task, what it handed in, and the plan's
//! `handed_in` mark replaying after a reopen. Spike 022, slice 1b.

use core_model::{
    Approach, DroneId, NewTask, PlanChange, Shown, TaskId, TaskState, TaskUpdate, Ulid,
};

use crate::tests::attempt::{on_its_first_run, step_id};
use crate::tests::{at, job_id, open, TempDir};
use crate::{PlanHand, TaskHandIn};

fn drone(id: &str) -> DroneId {
    DroneId::carried(Ulid::carried(id))
}

fn task(id: &str) -> TaskId {
    TaskId::read(id).expect("a task id")
}

/// Fleet's own mark, which `update_task` refuses, is kept and replays.
#[test]
fn a_handed_in_task_and_what_showed_it_survive_a_reopen() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01TASKDRONES");
    let step = step_id();
    let job = job_id("01TASKDRONES");
    let at = at("2026-10-02T10:05:00.000Z");
    let recorded = PlanChange::Recorded {
        approach: Approach::new("Bound the reader").expect("an approach"),
        tasks: vec![NewTask::new("Stop at the end", "", &[], "a test").expect("a title")],
    };
    store
        .change_plan(&job, &recorded, PlanHand::Step(&step), &at)
        .expect("kept");
    let handed_in = PlanChange::Updated {
        task: task("T1"),
        to: TaskUpdate::HandedIn,
        shown: Shown::new("read::stops passes"),
    };
    store
        .change_plan(&job, &handed_in, PlanHand::Step(&step), &at)
        .expect("Fleet's mark is kept");
    drop(store);

    let plan = open(&dir)
        .work_plan(&job)
        .expect("the history replays")
        .expect("a plan");
    let t1 = plan.task(task("T1")).expect("T1");
    assert_eq!(t1.state(), TaskState::HandedIn);
    assert_eq!(t1.shown().map(Shown::as_str), Some("read::stops passes"));
}

/// Each Drone is bound to its task at the spawn, and its hand-in fills the row.
#[test]
fn a_drone_is_bound_to_its_task_and_its_hand_in_is_kept() {
    let dir = TempDir::new();
    let mut store = open(&dir);
    on_its_first_run(&mut store, "01TASKDRONES");
    let step = step_id();
    let job = job_id("01TASKDRONES");
    store
        .record_task_drone(
            &job,
            &drone("01DRONEONE"),
            &step,
            task("T1"),
            &at("2026-10-02T10:00:00.000Z"),
        )
        .expect("bound");
    store
        .record_task_drone(
            &job,
            &drone("01DRONETWO"),
            &step,
            task("T2"),
            &at("2026-10-02T10:10:00.000Z"),
        )
        .expect("bound");
    let hand_in = TaskHandIn {
        claimed: "The reader stops at the end.".to_string(),
        shown_by: "read::stops passes".to_string(),
        not_claimed: String::new(),
        at: at("2026-10-02T10:09:00.000Z"),
    };
    store
        .record_task_hand_in(&job, &drone("01DRONEONE"), &hand_in)
        .expect("kept");
    drop(store);

    let bound = open(&dir).task_drones(&job).expect("reads");
    assert_eq!(
        bound
            .iter()
            .map(|row| (
                row.drone_id.as_str().to_string(),
                row.task,
                row.handed_in.clone()
            ))
            .collect::<Vec<_>>(),
        [
            ("01DRONEONE".to_string(), task("T1"), Some(hand_in)),
            ("01DRONETWO".to_string(), task("T2"), None),
        ]
    );
}
