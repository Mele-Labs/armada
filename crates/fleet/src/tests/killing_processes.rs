//! Killing one process of a Job, and every one — `Fleet::kill_process` and
//! `Fleet::kill_processes`, `#1647`.
//!
//! **Every Drone here is a real `sh` with real children**, because what is
//! claimed is about a process tree as `ps` reads it. Every signal these tests
//! cause lands on a process this test spawned: the stranger in the refusal case
//! is the one pid nothing may signal, and the test checks it never was.

use std::process::{Child, Command};
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CallDetail, DroneEvent};
use api::Refusal;
use config::ResolvedWorkflow;
use core_model::{
    EscalationTrigger, Job, JobId, JobStatus, StepId, StepLevelTrigger, StepState, StepVerdict,
};
use ipc::WireValue;
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Sketch};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::process::{holder_of, Holder};
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, fitted_with, one, worktree_directory};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const IMPLEMENT: &str = "implement";

/// A Drone with two children. **The first is in a process group of its own**
/// — `set -m` is job control, which gives a background job its own group —
/// so the Drone's group signal does not reach it, and only a kill that walks
/// the tree does.
const TWO_CHILDREN: &str =
    "set -m; sleep 30 & set +m; sleep 31 & echo BUSY; while IFS= read -r line; do :; done";

/// A Drone with one child, in its own group like any other.
const ONE_CHILD: &str = "sleep 31 & echo BUSY; while IFS= read -r line; do :; done";

fn a_drone(script: &str) -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", script]).reading(
        "BUSY",
        vec![DroneEvent::Called {
            tool: String::from("Bash"),
            call: String::from("a-call"),
            detail: CallDetail::of("cargo build"),
        }],
    )
}

fn one_step() -> ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: IMPLEMENT,
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

fn a_fleet(home: &TempDir, script: &str) -> Fixture {
    let mut fittings = fitted_with(
        home,
        FakeWorkProduct::changed(&["src/parse.rs"]),
        a_drone(script),
    );
    fittings.starting().workflows = one(one_step());
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about this"));
    Fleet::assembled(fittings)
}

/// A running Job, and its tree once the Drone has started every child:
/// the Drone's pid first.
async fn running(fleet: &Fixture, home: &TempDir, processes: usize) -> (JobId, Vec<u32>) {
    let job = fleet
        .propose(a_proposal("make the parser take it"))
        .await
        .expect("proposed");
    worktree_directory(home, &job);
    dispatched(fleet, job.id()).await.expect("dispatched");
    let tree = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let held = fleet.job_resources(&job).await.expect("a reading");
            if held.processes.len() == processes {
                return held.processes.iter().map(|one| one.pid).collect::<Vec<_>>();
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("the Drone started its children");
    (job.id().clone(), tree)
}

/// Where the Job stands, as far as a kill could have moved it.
fn standing(job: &Job) -> (JobStatus, Option<StepState>, Option<StepVerdict>) {
    let row = job.step(&StepId::new(IMPLEMENT));
    (
        job.status(),
        row.map(|row| row.state()),
        row.and_then(|row| row.last_verdict()),
    )
}

/// Wait until nothing holds `pid` as the process that held it at `started`.
async fn gone(pid: u32, started: &Holder) -> bool {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match holder_of(pid) {
                Ok(now) if &now == started => {}
                _ => return,
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .is_ok()
}

fn held(pid: u32) -> Holder {
    holder_of(pid).expect("ps answers")
}

/// A stranger: a process this test owns that is in no Job's tree.
fn a_stranger() -> Child {
    Command::new("sleep").arg("30").spawn().expect("sleep runs")
}

/// **The whole reason the route reads the tree again**: a pid the renderer
/// named is not a grant. One outside the Job's tree is refused, on the wire
/// as a 409 naming it, and nothing is signalled.
#[tokio::test]
async fn a_pid_outside_the_jobs_tree_is_refused_and_nothing_is_signalled() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, ONE_CHILD);
    let (job, _) = running(&fleet, &home, 2).await;
    let before = standing(&fleet.load(&job).await.unwrap());
    let mut stranger = a_stranger();

    let refused = fleet
        .kill_process(&job, stranger.id())
        .await
        .expect_err("a process that is not the Job's");

    assert!(
        matches!(refused, Adrift::NotTheJobsProcess { pid, .. } if pid == stranger.id()),
        "{refused:?}"
    );
    assert!(
        stranger.try_wait().expect("readable").is_none(),
        "the stranger is still running"
    );
    assert_eq!(standing(&fleet.load(&job).await.unwrap()), before);
    let Refusal::IllegalMove(wire) = fleet.killing_refusal(refused) else {
        panic!("a conflict: the row was read and is no longer true");
    };
    assert_eq!(wire.code, "fleet.not_the_jobs_process");
    assert_eq!(
        wire.fields.get("pid"),
        Some(&WireValue::Int(i64::from(stranger.id())))
    );
    stranger.kill().expect("the test ends what it started");
    let _ = stranger.wait();
}

/// A Job with no Drone has no tree, so every pid is refused against it.
#[tokio::test]
async fn a_job_with_no_drone_refuses_every_pid() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, ONE_CHILD);
    let job = fleet.propose(a_proposal("nothing runs")).await.unwrap();
    let mut stranger = a_stranger();

    let refused = fleet.kill_process(job.id(), stranger.id()).await;

    assert!(
        matches!(refused, Err(Adrift::NotTheJobsProcess { .. })),
        "{refused:?}"
    );
    assert!(stranger.try_wait().expect("readable").is_none());
    stranger.kill().expect("the test ends what it started");
    let _ = stranger.wait();
}

/// A runaway build under the Drone, killed from Pulse. **It is not the Drone
/// dying**: the child goes, and the Drone, its step and the Job are where they
/// were.
#[tokio::test]
async fn killing_a_child_ends_it_and_leaves_the_drone_and_its_step_as_they_were() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, ONE_CHILD);
    let (job, tree) = running(&fleet, &home, 2).await;
    let (drone, child) = (tree[0], tree[1]);
    let (drone_was, child_was) = (held(drone), held(child));
    let before = standing(&fleet.load(&job).await.unwrap());
    assert_eq!(before.0, JobStatus::Running);

    let answered = fleet
        .kill_process(&job, child)
        .await
        .expect("the child is the Job's");

    assert!(gone(child, &child_was).await, "the child ended");
    assert_eq!(
        standing(&answered),
        before,
        "the answer is the Job as it was"
    );
    // A turn, so anything that would read the child's end as the Drone's has
    // had its chance to.
    fleet.turn().await.expect("a turn");
    assert_eq!(standing(&fleet.load(&job).await.unwrap()), before);
    assert_eq!(
        held(drone),
        drone_was,
        "the Drone is still the same process"
    );
    let after = fleet
        .job_resources(&fleet.load(&job).await.unwrap())
        .await
        .unwrap();
    assert_eq!(after.held, ipc::Held::Running);
    assert!(
        after.processes.iter().all(|one| one.pid != child),
        "the next reading drops the row"
    );
    fleet
        .kill_drone(&job)
        .await
        .expect("the test ends its Drone");
}

/// **The Drone's own pid is `kill_drone`**, and the Job is left exactly as
/// that leaves it: two Fleets, one killed each way.
#[tokio::test]
async fn killing_the_drones_own_pid_leaves_the_job_as_kill_drone_does() {
    let (by_pid_home, by_act_home) = (TempDir::new(), TempDir::new());
    let by_pid = a_fleet(&by_pid_home, ONE_CHILD);
    let by_act = a_fleet(&by_act_home, ONE_CHILD);
    let (pid_job, tree) = running(&by_pid, &by_pid_home, 2).await;
    let (act_job, _) = running(&by_act, &by_act_home, 2).await;
    let drone_was = held(tree[0]);

    let answered = by_pid
        .kill_process(&pid_job, tree[0])
        .await
        .expect("the Drone is the Job's");
    by_act.kill_drone(&act_job).await.expect("the Drone ends");

    let killed = standing(&by_act.load(&act_job).await.unwrap());
    assert_eq!(
        killed.1,
        Some(StepState::Stopped),
        "the comparison is to a stopped step"
    );
    assert_eq!(standing(&by_pid.load(&pid_job).await.unwrap()), killed);
    assert_eq!(standing(&answered), killed);
    assert!(gone(tree[0], &drone_was).await, "and the Drone is gone");
}

/// Kill all reaches every process the Job holds — **the one in a group of its
/// own too**, which `kill_drone`'s group signal would leave running,
/// reparented and out of the Job's tree. The Job is left as `kill_drone`
/// leaves it.
#[tokio::test]
async fn killing_all_ends_every_process_the_job_holds() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, TWO_CHILDREN);
    let (job, tree) = running(&fleet, &home, 3).await;
    let was: Vec<(u32, Holder)> = tree.iter().map(|pid| (*pid, held(*pid))).collect();

    let answered = fleet.kill_processes(&job).await.expect("every process");

    for (pid, started) in &was {
        assert!(gone(*pid, started).await, "pid {pid} ended");
    }
    let (_, step, verdict) = standing(&answered);
    assert_eq!(
        step,
        Some(StepState::Stopped),
        "the Drone went, so its step stopped"
    );
    assert_eq!(
        verdict,
        Some(StepVerdict::Failed(
            StepLevelTrigger::of(EscalationTrigger::DroneKilled).expect("a step-level trigger")
        )),
        "stopped by hand, as kill_drone stops it"
    );
    let after = fleet
        .job_resources(&fleet.load(&job).await.unwrap())
        .await
        .unwrap();
    assert!(after.processes.is_empty(), "{:?}", after.processes);
}
