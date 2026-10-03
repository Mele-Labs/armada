//! Slice 5: **tasks the planner marked as safe together run at once, within
//! this Job's cap and the machine's; I can watch, message or stop any one of
//! their Drones without touching the others; and where two of them edited one
//! file, the group does not commit.**
//!
//! A module of `drone_per_task` rather than more of it, because the file is
//! near the gate's line limit and the claim is one milestone's either way.
//!
//! | Not proved here | Why not |
//! |---|---|
//! | Fleet spawning a second Drone beside the first, and ending one while the other goes on | Hermetic: nothing here spawns. `crates/fleet/src/tests/at_once.rs` drives the fake harness |
//! | A redirect or a kill refused for a Drone that is not live | Needs a Fleet holding Drones; `crates/fleet/src/tests/at_once.rs` |
//! | Reading a path off an edit call in a real Drone's stream | The vendor's tool names are `adapters`' alone; `crates/adapters/src/tests/edited.rs` |

use std::collections::BTreeSet;

use core_model::{GroupId, GroupRuns, TaskId, TaskState, Timestamp};
use fleet::crew::{self, Asking, ExtraRoom};
use fleet::edit_calls::{self, Edited};
use fleet::tasking;
use ipc::{Instant, Saw, TranscriptRow};

use crate::bench::arc::feature_with_a_drone_per_task;
use crate::bench::board::received_detail;
use crate::bench::focus::drone;
use crate::bench::plan::{called, Planned};

/// Group 1 runs T1 and T2 at once; T3 is in it too and runs alone. Group 2 is
/// T4, which comes back to a file T1 wrote.
const AT_ONCE: &str = r#"{"approach":"Bound the reader and its session together, then say so",
    "tasks":[{"title":"Bound the reader","note":"","scope":["crates/store/src/read.rs"],"expects":"","group":1,"concurrent_with":[2]},
             {"title":"Bound the session","note":"","scope":["crates/store/src/session.rs"],"expects":"","group":1},
             {"title":"Cover both","note":"","scope":[],"expects":"","group":1},
             {"title":"Note the bound","note":"","scope":["crates/store/src/read.rs"],"expects":"","group":2}]}"#;

fn task(id: &str) -> TaskId {
    TaskId::read(id).expect("a task id")
}

fn at(second: u32) -> Timestamp {
    Timestamp::from_rfc3339(format!("2026-10-02T11:00:{second:02}.000Z"))
}

fn said(by: u32, what: &str) -> TranscriptRow {
    TranscriptRow {
        ts: Instant::carried("2026-10-02T11:00:00.000Z"),
        step: None,
        by: ipc::Voice::Drone,
        drone_id: Some((&drone(by)).into()),
        saw: Saw::Said {
            text: what.to_string(),
        },
    }
}

/// What a viewer heard next, as `(drone, words)`.
async fn heard(watch: &mut api::Watch) -> (String, String) {
    let seen = tokio::time::timeout(std::time::Duration::from_secs(5), watch.next())
        .await
        .expect("the channel answers")
        .expect("the Job's channel is open");
    let api::Seen::Row(row) = seen else {
        panic!("a row, not {seen:?}");
    };
    let Saw::Said { text } = &row.saw else {
        panic!("prose");
    };
    let by = row
        .drone_id
        .map(|id| id.as_str().to_string())
        .unwrap_or_default();
    (by, text.clone())
}

fn edited(task_id: &str, by: u32, from: u32, to: u32, paths: &[&str]) -> Edited {
    Edited {
        task: task(task_id),
        drone: drone(by),
        from: at(from),
        to: at(to),
        paths: paths.iter().map(|path| path.to_string()).collect(),
    }
}

#[tokio::test]
async fn tasks_marked_safe_together_run_at_once_one_can_be_stopped_and_an_overlap_does_not_commit()
{
    let mut planned = Planned::created_with("bound the reader", feature_with_a_drone_per_task());
    let mut plan = planned.kept(called("record_plan", AT_ONCE), "plan", 1);
    let runs = GroupRuns::default();
    let g1 = GroupId::read("G1").expect("a group id");

    // ------------------------------------ the planner says which run together
    let served = received_detail(&planned.detail())
        .work_plan
        .expect("the plan crossed");
    let beside: Vec<(&str, Vec<&str>)> = served
        .tasks
        .iter()
        .map(|t| {
            (
                t.id.as_str(),
                t.concurrent_with.iter().map(String::as_str).collect(),
            )
        })
        .collect();
    assert_eq!(
        beside,
        [
            ("T1", vec!["T2"]),
            ("T2", vec!["T1"]),
            ("T3", vec![]),
            ("T4", vec![]),
        ],
        "T1 named T2, so each runs beside the other, and T3 runs alone"
    );

    // ------------------------------------------- two Drones, one per task
    let first = tasking::next_task_beside(&plan, &runs, &[]).expect("G1 has a task open");
    assert_eq!(first.id(), task("T1"));
    plan = planned.marked(tasking::started(task("T1")), "implement", 1);
    let second = tasking::next_task_beside(&plan, &runs, &[task("T1")]).expect("T2 runs beside T1");
    assert_eq!(second.id(), task("T2"), "the task marked safe beside T1");
    plan = planned.marked(tasking::started(task("T2")), "implement", 1);
    assert!(
        tasking::next_task_beside(&plan, &runs, &[task("T1"), task("T2")]).is_none(),
        "T3 was not marked safe beside them, so it waits"
    );

    // ------------------------------- inside this Job's cap and the machine's
    let asking = Asking {
        drones: 1,
        machine_cap: 3,
        job_drones: 1,
        job_cap: None,
        a_job_waits: false,
    };
    assert_eq!(crew::extra_room(&asking), ExtraRoom::Yes);
    assert_eq!(
        crew::extra_room(&Asking {
            drones: 3,
            ..asking
        }),
        ExtraRoom::MachineFull,
        "the machine's cap counts Drones, not Jobs"
    );
    assert_eq!(
        crew::extra_room(&Asking {
            job_cap: Some(1),
            ..asking
        }),
        ExtraRoom::JobFull,
        "the Job's own cap sits inside the machine's"
    );
    assert_eq!(
        crew::extra_room(&Asking {
            a_job_waits: true,
            ..asking
        }),
        ExtraRoom::YieldsToAWaitingJob,
        "a Job's extra Drone gives way to a Job waiting to start"
    );

    // ------------------------ one channel per Job, every Drone heard on it
    let turns = api::Turns::new();
    let job = ipc::JobId::from(planned.job.id());
    let channel = turns.opening(&job);
    let t1 = turns.feeding(&job);
    let t2 = turns.feeding(&job);
    let mut early = turns.watching(&job).expect("the Job's channel is open");
    t1.offer(said(1, "T1 reads the bound"));
    t2.offer(said(2, "T2 reads the session"));
    let drone_1 = drone(1).as_str().to_string();
    let drone_2 = drone(2).as_str().to_string();
    assert_eq!(
        heard(&mut early).await,
        (drone_1.clone(), "T1 reads the bound".to_string())
    );
    assert_eq!(
        heard(&mut early).await,
        (drone_2.clone(), "T2 reads the session".to_string()),
        "one viewer hears both Drones"
    );
    // One Drone stops. The other goes on, and a viewer arriving now hears it.
    drop(t1);
    let mut late = turns.watching(&job).expect("still open: T2 goes on");
    t2.offer(said(2, "T2 carries on"));
    assert_eq!(
        heard(&mut early).await,
        (drone_2.clone(), "T2 carries on".to_string())
    );
    assert_eq!(
        heard(&mut late).await,
        (drone_2.clone(), "T2 carries on".to_string()),
        "a viewer hears every live Drone from whenever it joins"
    );
    drop(t2);
    drop(channel);
    assert!(
        early.next().await.is_none(),
        "the channel goes at the Job's end, not at a Drone's"
    );

    // ------------------- message or stop one Drone: routes that name a Drone
    for (operation, path) in [
        (
            "redirect_one_drone",
            "/jobs/:job_id/drones/:drone_id/redirect",
        ),
        ("kill_one_drone", "/jobs/:job_id/drones/:drone_id/kill"),
    ] {
        assert!(
            api::SERVED
                .iter()
                .any(|route| route.operation == operation && route.path == path),
            "{operation} is served at {path}"
        );
    }
    let process = ipc::JobProcess {
        pid: 4242,
        command: "node".to_string(),
        cpu_percent: 1.0,
        memory_bytes: 1024,
        running_for: "00:12".to_string(),
        recorded: false,
        drone_id: Some((&drone(2)).into()),
    };
    let body = ipc::encode(&process).expect("a process encodes");
    let back: ipc::JobProcess = ipc::decode("a process", body.as_bytes()).expect("and decodes");
    assert_eq!(
        back.drone_id.as_ref().map(|id| id.as_str()),
        Some(drone_2.as_str()),
        "a process names the Drone whose it is"
    );

    // --------------------------------- two at once wrote one file: no commit
    planned.marked(tasking::handed_in(task("T1"), "the diff"), "implement", 1);
    plan = planned.marked(tasking::handed_in(task("T2"), "the diff"), "implement", 1);
    let wrote = [
        edited("T1", 1, 10, 30, &["crates/store/src/read.rs"]),
        edited(
            "T2",
            2,
            12,
            28,
            &["crates/store/src/session.rs", "crates/store/src/read.rs"],
        ),
    ];
    let overlaps = edit_calls::overlapping(&plan, g1, &wrote);
    assert_eq!(overlaps.len(), 1, "{overlaps:?}");
    assert_eq!(overlaps[0].tasks, (task("T1"), task("T2")));
    assert_eq!(overlaps[0].paths, ["crates/store/src/read.rs"]);
    let (reopened, apart) = edit_calls::run_apart(g1, &overlaps, at(31));
    for change in reopened {
        plan = planned.marked(change, "implement", 1);
    }
    let runs = GroupRuns::fold(&apart);
    for id in ["T1", "T2"] {
        assert_eq!(
            plan.task(task(id)).map(|t| t.state()),
            Some(TaskState::Open),
            "{id} runs again"
        );
    }
    assert!(
        tasking::together(&plan, &runs, &[]).is_none(),
        "the group does not reach its gate, so it does not commit"
    );
    let again = tasking::next_task_beside(&plan, &runs, &[]).expect("T1 runs again");
    assert_eq!(again.id(), task("T1"));
    plan = planned.marked(tasking::started(task("T1")), "implement", 1);
    assert!(
        tasking::next_task_beside(&plan, &runs, &[task("T1")]).is_none(),
        "and T2 waits for it: the two run one after the other"
    );
    let served = received_detail(&planned.detail_with(&runs))
        .work_plan
        .expect("the plan crossed");
    let t1_served = served.tasks.iter().find(|t| t.id == "T1").expect("T1");
    assert!(
        t1_served.concurrent_with.is_empty(),
        "the plan no longer says T1 runs beside T2: {:?}",
        t1_served.concurrent_with
    );

    // The same file, written one after the other, is no overlap.
    let in_turn = [
        edited("T1", 1, 10, 20, &["crates/store/src/read.rs"]),
        edited("T2", 2, 21, 30, &["crates/store/src/read.rs"]),
    ];
    assert!(edit_calls::overlapping(&plan, g1, &in_turn).is_empty());

    // ----------------------------- a later task's edit touches a done task
    for id in ["T1", "T2", "T3"] {
        plan = planned.marked(tasking::done(task(id)), "implement", 1);
    }
    let later = [
        edited("T1", 3, 40, 50, &["crates/store/src/read.rs"]),
        edited("T2", 4, 52, 58, &["crates/store/src/session.rs"]),
        edited("T4", 5, 60, 70, &["crates/store/src/read.rs"]),
    ];
    let touched: BTreeSet<TaskId> = edit_calls::touched_after_done(&plan, &later);
    assert_eq!(
        touched,
        [task("T1")].into_iter().collect(),
        "T4 edited a file T1 wrote after T1 was done, and nothing T2 wrote"
    );
    let mut row = served.tasks[0].clone();
    row.touched_after_done = true;
    let body = ipc::encode(&row).expect("a task encodes");
    let back: ipc::PlanTask = ipc::decode("a task", body.as_bytes()).expect("and decodes");
    assert!(back.touched_after_done, "and the plan says so");
}
