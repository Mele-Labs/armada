//! The new Job's backend milestone, spike 022: **a Job's plan is worked by a
//! Drone per task, group by group, and I can see and act on each task, each
//! group and each Drone.** Added to slice by slice; every slice not named
//! below is unasserted, and is its issue's row in the spike's milestone table.
//!
//! What it asserts today is slice 0b's Fleet half: **I can read how many events
//! of each kind Fleet published each minute, and for which Job.** Why the tally
//! is read off the broadcaster's own window is `api::stream`.
//!
//! | Not proved here | Why not |
//! |---|---|
//! | A Drone per task rather than per step | #1762 (slice 1b). Until then the rate is a step's |
//! | Bridge redrawing the pane whole after a `missed` | Nothing here renders; `apps/desktop/src/main/observe.test.ts` |
//! | Fleet writing the line once a minute | A `tokio` interval in `crates/armada/src/serve.rs`; it writes the tally asserted here |

// The bench is shared with every other milestone's test and none uses all of it.
#[allow(dead_code)]
mod bench;

use core_model::{Actor, StepId, Target};
use ipc::{ChangeKind, ChangedFile, DroneExited, DroneSpawned, Event, JobFilesChanged};
use ipc::{JobStateChanged, JobSummary, RepositoryList};
use testkit::FakeWorkProduct;

use bench::focus::{drone, now};
use bench::{Bench, Run};

/// The row a `drone.*` event carries, as Fleet builds it with nothing to add.
fn summary(run: &Run) -> JobSummary {
    JobSummary::of(&run.job, None, None, None, false, None, None, None)
}

/// One footprint reading of the Drone on `step`, which Fleet publishes while a
/// Bridge is listening and never stores.
fn files_changed(bench: &Bench, run: &Run, step: &StepId, n: usize) -> Event {
    let drone = run
        .job
        .assigned_drone()
        .expect("a working Drone's footprint");
    Event::JobFilesChanged(JobFilesChanged {
        job_id: run.job.id().into(),
        step_id: step.into(),
        drone_id: drone.into(),
        plan_declared: false,
        files: (0..n)
            .map(|i| ChangedFile {
                path: format!("crates/store/src/read_{i}.rs"),
                change: ChangeKind::Modified,
                outside_plan: false,
                lines: None,
            })
            .collect(),
        actor: Actor::Fleet.into(),
        at: (&now(bench)).into(),
    })
}

#[test]
fn every_minute_says_how_many_events_of_each_kind_fleet_published_and_for_which_job() {
    let events = api::Broadcaster::new();
    let bench = Bench::with(FakeWorkProduct::changed(&["crates/store/src/read.rs"]));
    let minute = events.cursor();

    // ------------------------------------------------- one Job, two Drones

    let mut run = bench.created("fix the cursor that reads one row past the end");
    let job = run.job.id().as_str().to_string();
    bench.approved_and_dispatched(&mut run);
    for moved in bench.moves.borrow().iter() {
        events.publish(Event::JobStateChanged(JobStateChanged::from(moved)));
    }

    let (first, second) = (bench.step(0), bench.step(1));
    let arrived = run
        .job
        .drone_spawned(&first, drone(1), Actor::Fleet, now(&bench))
        .expect("nothing is on the first step yet");
    run.job = arrived.job;
    events.publish(Event::DroneSpawned(DroneSpawned::of(
        &arrived.event,
        summary(&run),
        None,
    )));
    for n in 1..=3 {
        events.publish(files_changed(&bench, &run, &first, n));
    }
    let left = run
        .job
        .drone_exited(&first, Actor::Fleet, now(&bench))
        .expect("the first Drone is on the first step");
    run.job = left.job;
    events.publish(Event::DroneExited(DroneExited::of(
        &left.event,
        summary(&run),
    )));
    let arrived = run
        .job
        .drone_spawned(&second, drone(2), Actor::Fleet, now(&bench))
        .expect("the first Drone is gone");
    run.job = arrived.job;
    events.publish(Event::DroneSpawned(DroneSpawned::of(
        &arrived.event,
        summary(&run),
        None,
    )));
    events.publish(files_changed(&bench, &run, &second, 1));

    // ----------------------------------- a second Job, and the machine's own

    let other = bench.created("a second Job, queued behind the first");
    let other_job = other.job.id().as_str().to_string();
    let queued = other
        .job
        .transition(Target::Queued, Actor::Human, now(&bench))
        .expect("a created Job can be approved");
    events.publish(Event::JobStateChanged(JobStateChanged::from(&queued.event)));
    events.publish(Event::RepositoriesChanged(RepositoryList {
        repositories: Vec::new(),
    }));

    // ------------------------------------------------------- the minute's tally

    let tally = events.tallied(minute);
    assert_eq!(
        (
            tally.count("job.state_changed"),
            tally.count("drone.spawned"),
            tally.count("drone.exited"),
            tally.count("job.files_changed"),
            tally.count("repositories.changed"),
        ),
        (3, 2, 1, 4, 1),
        "every kind published is counted, the ones never stored included: {tally}"
    );
    assert_eq!(
        (
            tally.count_for(&job, "drone.spawned"),
            tally.count_for(&job, "job.files_changed"),
            tally.count_for(&job, "job.state_changed"),
            tally.count_for(&other_job, "job.state_changed"),
        ),
        (2, 4, 2, 1),
        "and again under the Job each names, so a rate can be read per Job: {tally}"
    );

    let line = tally.to_string();
    for said in [
        "11 events".to_string(),
        "drone.spawned 2".to_string(),
        "job.files_changed 4".to_string(),
        "repositories.changed 1".to_string(),
        format!("{job}: 9"),
        format!("{other_job}: 1"),
        format!("BACKLOG {}", api::BACKLOG),
    ] {
        assert!(line.contains(&said), "the log line says {said:?}: {line}");
    }

    // The next minute starts where this one ended, so nothing is counted twice.
    let next = events.tallied(tally.upto());
    assert!(
        next.is_empty(),
        "a minute nothing crossed counts nothing: {next}"
    );
}
