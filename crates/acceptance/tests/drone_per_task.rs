//! The new Job's backend milestone, spike 022: **a Job's plan is worked by a
//! Drone per task, group by group, and I can see and act on each task, each
//! group and each Drone.** Added to slice by slice; every slice not named
//! below is unasserted, and is its issue's row in the spike's milestone table.
//!
//! It asserts slice 0b's Fleet half, **how many events of each kind Fleet
//! published each minute, and for which Job** (why: `api::stream`), and slice
//! 1a's, **a Judge's refusal and a Check's failure signed by them**.
//!
//! | Not proved here | Why not |
//! |---|---|
//! | A Drone per task rather than per step | #1762 (slice 1b). Until then the rate is a step's |
//! | Bridge redrawing the pane whole after a `missed` | Nothing here renders; `apps/desktop/src/main/observe.test.ts` |
//! | A Fleet built before 1a refuses the store | Hermetic; `crates/store/src/tests/signers.rs` |
//! | Fleet writing the line once a minute | A `tokio` interval in `crates/armada/src/serve.rs` |

// The bench is shared with every other milestone's test and none uses all of it.
#[allow(dead_code)]
mod bench;

use core_model::{Actor, JobEvent, JobStatus, StepId, Target};
use fleet::Ruling;
use ipc::{ChangeKind, ChangedFile, DroneExited, DroneSpawned, Event, JobFilesChanged};
use ipc::{JobStateChanged, JobSummary, RepositoryList};
use testkit::{FakeJudge, FakeWorkProduct};

use bench::arc::step_signers;
use bench::focus::{drone, now};
use bench::{a_fix_diff, a_root_cause_note, bug_workflow_with_the_fix_judged, Bench, Run};

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

/// The actor on a Job's move, after a round trip through the wire.
fn served(event: &JobEvent) -> ipc::Actor {
    let wire = ipc::JobStateChanged::from(event);
    let text = ipc::encode(&wire).expect("a Job's move encodes");
    let back: ipc::JobStateChanged =
        ipc::decode("a Job's move", text.as_bytes()).expect("and decodes");
    back.actor
}

/// A Judge refuses the fix, and both rows the refusal writes are the Judge's.
#[tokio::test]
async fn a_judges_refusal_is_signed_by_the_judge() {
    let bench = Bench::judged_by(
        FakeWorkProduct::changed(&["crates/store/src/read.rs"]),
        bug_workflow_with_the_fix_judged(),
        FakeJudge::refusing(
            "a fix addressing the cause the note named",
            "a change to an unrelated bound",
            "the reported symptom still occurs",
        ),
    );
    let mut run = bench.created("fix the cursor that reads one row past the end");
    bench.approved_and_dispatched(&mut run);
    let ruling = bench.gate(&run, &bench.step(0), &a_root_cause_note()).await;
    bench.settled(&mut run, &bench.step(0), &ruling);

    let ruling = bench.gate(&run, &bench.step(1), &a_fix_diff()).await;
    assert!(
        matches!(ruling, Ruling::Refused { .. }),
        "the Judge refuses the fix, and got {ruling:?}"
    );
    bench.settled(&mut run, &bench.step(1), &ruling);

    assert_eq!(run.job.status(), JobStatus::Escalated);
    assert_eq!(
        step_signers(&bench).last(),
        Some(&Actor::Judge),
        "the step the Judge stopped says the Judge stopped it"
    );
    assert_eq!(
        bench.actors(),
        vec![Actor::Human, Actor::Fleet, Actor::Judge],
        "approval is a person's, dispatch Fleet's, and the escalation the Judge's"
    );
    let moves = bench.moves.borrow();
    let escalated = moves.last().expect("the refusal moved the Job");
    assert_eq!(
        served(escalated),
        ipc::Actor::from(Actor::Judge),
        "and Bridge is told the Judge signed it, not Fleet"
    );
}

/// A Check fails with no retry left, and both rows the failure writes are the
/// Check's.
#[tokio::test]
async fn a_failed_check_is_signed_by_the_check() {
    // Nothing changed, so `diff_nonempty` fails.
    let bench = Bench::with(FakeWorkProduct::untouched());
    let mut run = bench.created("change nothing");
    bench.approved_and_dispatched(&mut run);
    let ruling = bench.gate(&run, &bench.step(0), &a_root_cause_note()).await;
    bench.settled(&mut run, &bench.step(0), &ruling);

    let ruling = bench.gate(&run, &bench.step(1), &a_fix_diff()).await;
    assert!(
        matches!(ruling, Ruling::Failed { .. }),
        "an empty diff is a failed Check, and got {ruling:?}"
    );
    bench.settled(&mut run, &bench.step(1), &ruling);

    assert_eq!(run.job.status(), JobStatus::AwaitingRepair);
    assert_eq!(
        step_signers(&bench).last(),
        Some(&Actor::Check),
        "the step the Check stopped says the Check stopped it"
    );
    assert_eq!(
        bench.actors(),
        vec![Actor::Human, Actor::Fleet, Actor::Check],
        "approval is a person's, dispatch Fleet's, and the hold the Check's"
    );
    let moves = bench.moves.borrow();
    let held = moves.last().expect("the failure moved the Job");
    assert_eq!(
        served(held),
        ipc::Actor::from(Actor::Check),
        "and Bridge is told the Check signed it, not Fleet"
    );
}
