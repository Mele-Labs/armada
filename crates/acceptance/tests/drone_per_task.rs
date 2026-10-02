//! The new Job's backend milestone, spike 022: **a Job's plan is worked by a
//! Drone per task, group by group, and I can see and act on each task, each
//! group and each Drone.** Added slice by slice; a slice not named is its
//! issue's row in the spike's milestone table. Asserted: 0b's per-minute event
//! tally (`api::stream`), 1a's signers, and 1b's plan worked a Drone per task.
//!
//! | Not proved here | Why not |
//! |---|---|
//! | Fleet spawning, ending and respawning the Drones themselves | Hermetic: nothing here spawns. `crates/fleet/src/tests/drone_per_task.rs` drives a fake harness through every task |
//! | Bridge redrawing the pane whole after a `missed` | Nothing here renders; `apps/desktop/src/main/observe.test.ts` |
//! | A Fleet built before 1a refuses the store | Hermetic; `crates/store/src/tests/signers.rs` |
//! | Fleet writing the line once a minute | A `tokio` interval in `crates/armada/src/serve.rs` |

// The bench is shared with every other milestone's test and none uses all of it.
#[allow(dead_code)]
mod bench;

use core_model::{Actor, JobEvent, JobStatus, StepId, Target, TaskId, TaskState};
use fleet::tasking::{self, HandIn};
use fleet::{briefing, Crossed, Ruling, ThePlan};
use ipc::{ChangeKind, ChangedFile, DroneExited, DroneSpawned, Event, JobFilesChanged};
use ipc::{JobPlanChanged, JobStateChanged, JobSummary, RepositoryList};
use testkit::{FakeJudge, FakeWorkProduct};

use bench::arc::{feature_with_a_drone_per_task, step_signers};
use bench::board::received_detail;
use bench::focus::{drone, now};
use bench::plan::{called, received_event, Planned};
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

const THREE_TASKS: &str = r#"{"approach":"Stop the reader at the end, cover it, then say so",
    "tasks":[{"title":"Stop the reader at the end","note":"","scope":["crates/store/src/read.rs"],"expects":"a test for the last row"},
             {"title":"Cover the last row","note":"","scope":[],"expects":""},
             {"title":"Note the bound in the module","note":"","scope":[],"expects":""}]}"#;

/// What each task's Drone hands in: its task, what it claims, what shows it.
const HANDED_IN: [(&str, &str, &str); 3] = [
    (
        "T1",
        "The reader stops at the last row.",
        "store::read::the_last_row_is_the_last passes",
    ),
    (
        "T2",
        "The last row is covered by a test.",
        "the new case in crates/store/src/tests/read.rs",
    ),
    (
        "T3",
        "The module says where the bound is.",
        "the doc comment on read_to",
    ),
];

fn task(id: &str) -> TaskId {
    TaskId::read(id).expect("a task id")
}

fn at(second: u32) -> core_model::Timestamp {
    core_model::Timestamp::from_rfc3339(format!("2026-10-02T10:00:{second:02}.000Z"))
}

/// Each task's id and state, in plan order, as `get_job` serves them.
fn served_states(planned: &Planned) -> Vec<(String, &'static str)> {
    received_detail(&planned.detail())
        .work_plan
        .expect("the plan crossed")
        .tasks
        .iter()
        .map(|task| (task.id.clone(), task.state.as_wire()))
        .collect()
}

/// Slice 1b: **a Job's plan is worked one task at a time, each by a Drone of
/// its own, and the plan tells me which task is being worked, which are done,
/// and what showed each one done.** Driven through what Fleet calls at each
/// spawn and hand-in, against a workflow read by `config`'s own parser.
#[test]
fn a_plan_is_worked_one_task_at_a_time_each_by_a_drone_of_its_own() {
    let mut planned = Planned::created_with("bound the reader", feature_with_a_drone_per_task());
    let implement = StepId::new("implement");
    let workflow = planned.job.workflow().clone();
    let step = workflow.step(&implement).expect("implement is a step");
    assert!(
        step.drone_per_task(),
        "implement works its tasks a Drone each"
    );
    let tests = workflow
        .step(&StepId::new("tests"))
        .expect("tests is a step");
    assert!(
        !tests.drone_per_task() && tests.follows_plan(),
        "Write tests keeps one Drone and keeps follows_plan"
    );

    let mut plan = planned.kept(called("record_plan", THREE_TASKS), "plan", 1);
    let mut hand_ins: Vec<HandIn> = Vec::new();
    for (n, (id, claimed, shown)) in HANDED_IN.iter().enumerate() {
        // ------------------------------------------------ the spawn, in order
        let next = tasking::next_task(&plan).expect("a task is still open");
        assert_eq!(next.id(), task(id), "the next spawn takes plan order");
        assert!(
            tasking::together(&plan, &hand_ins).is_none(),
            "the step's gate waits while a task is open or working"
        );

        // The Drone is told which task is its own, and that handing in ends it.
        let crossed = Crossed::nothing().and_the_plan(Some(ThePlan::for_task(&plan, next)));
        let brief = briefing::first_turn(&planned.job, &workflow, &implement, &crossed)
            .expect("a brief assembles");
        let said = brief.as_str();
        assert!(said.contains("YOUR TASK"), "{said}");
        assert!(
            said.contains(&format!("yours is {id}: {}", next.title())),
            "the brief names {id} as this Drone's: {said}"
        );
        assert!(
            said.contains("ends your work on this Job"),
            "and says the hand-in ends it: {said}"
        );
        assert!(
            !said.contains("update_task"),
            "a task's Drone marks nothing itself: {said}"
        );

        // Fleet marks it working at the spawn, and says which task moved.
        plan = planned.marked(tasking::started(task(id)), "implement", 1);
        let working: Vec<_> = served_states(&planned)
            .into_iter()
            .filter(|(_, state)| *state == "working")
            .collect();
        assert_eq!(
            working,
            [(id.to_string(), "working")],
            "one task is being worked, and the plan says which"
        );
        let moved = Event::JobPlanChanged(JobPlanChanged::task_moved(
            planned.job.id(),
            &plan,
            task(id),
            Actor::Fleet,
            &at(n as u32),
        ));
        let Event::JobPlanChanged(heard) = received_event(&moved) else {
            panic!("a plan change crosses as one");
        };
        assert_eq!(
            (
                heard.task.as_deref(),
                heard.state.map(|state| state.as_wire())
            ),
            (Some(*id), Some("working")),
            "the event names the task and its new state"
        );

        // --------------------------------------------------------- the hand-in
        // `submit_evidence`, kept as the task's: handed in, with what showed it.
        plan = planned.marked(tasking::handed_in(task(id), shown), "implement", 1);
        hand_ins.push(HandIn {
            task: task(id),
            claimed: claimed.to_string(),
            shown_by: shown.to_string(),
            not_claimed: String::new(),
        });
        assert_eq!(
            plan.task(task(id)).map(|done| done.state()),
            Some(TaskState::HandedIn),
            "handed in, and not done until the step's Checks answer"
        );
    }

    // ---------------------------------------------- the step's one submission
    assert!(
        tasking::next_task(&plan).is_none(),
        "no task is left to spawn a Drone for"
    );
    let together = tasking::together(&plan, &hand_ins).expect("the step's gate fires");
    for (id, claimed, shown) in HANDED_IN {
        assert!(
            together.claimed.contains(&format!("{id}: {claimed}")),
            "{id}'s claim is in the step's submission: {}",
            together.claimed
        );
        assert!(
            together.shown_by.contains(&format!("{id}: {shown}")),
            "and what showed it: {}",
            together.shown_by
        );
    }
    assert_eq!((plan.counts().handed_in, plan.counts().done), (3, 0));

    // ------------------------------------------------ the step's Checks pass
    for (id, _, _) in HANDED_IN {
        plan = planned.marked(tasking::done(task(id)), "implement", 1);
    }
    let served = received_detail(&planned.detail())
        .work_plan
        .expect("the plan crossed");
    for ((id, _, shown), served) in HANDED_IN.iter().zip(&served.tasks) {
        assert_eq!((served.id.as_str(), served.state.as_wire()), (*id, "done"));
        assert_eq!(
            served.shown.as_deref(),
            Some(*shown),
            "{id} reads done with what showed it done"
        );
    }
    assert_eq!(plan.counts().done, 3);

    // ---------------------------------------------- each by a Drone of its own
    let row = ipc::JobDrone {
        drone_id: (&drone(2)).into(),
        step_id: (&implement).into(),
        task: Some("T2".to_string()),
        state: ipc::DroneState::Done,
        since: (&at(10)).into(),
        ended_at: Some((&at(20)).into()),
        turns: None,
        cost_micros: None,
    };
    let body = ipc::encode(&row).expect("a Drone row encodes");
    let back: ipc::JobDrone = ipc::decode("a Drone row", body.as_bytes()).expect("and decodes");
    assert_eq!(
        back.task.as_deref(),
        Some("T2"),
        "a Drone's row names the task it was put on"
    );
}
