//! A person's skip and retry on a gate that is running Checks.
//!
//! Real commands, for `crate::tests::underway`'s reason: what is claimed is a
//! process ending and another starting, and a fake runner would be asserting
//! this file's guess at both. Each case drives `crate::checking::ran` and acts
//! through the run's [`Control`], which is the thing `Fleet::skip_check`,
//! `skip_checks` and `retry_check` reach through `Underway::gate_of`.

use std::sync::Arc;
use std::time::Duration;

use core_model::{Attempt, ResolvedCheck, StepId};
use verification::{Exit, Observed, Ran};

use crate::check_acts::{Control, Unactable};
use crate::checking::{ran, Completed};
use crate::places::{ChecksAtOnce, Room};
use crate::tests::gate::Stopped;
use crate::tests::tmp::TempDir;
use crate::underway::{Announcing, Underway};

const JOB: &str = "01JOB";
const STEP: &str = "implement";

fn named(name: &str, run: &str) -> ResolvedCheck {
    ResolvedCheck::ManifestCheck {
        manifest_dir: String::new(),
        name: name.to_string(),
        run: run.to_string(),
        expect_exit_code: 0,
        when: None,
        requires: Vec::new(),
        narrow: None,
        one_test: None,
        runs_at: core_model::RunsAt::Everywhere,
        places: std::num::NonZeroU32::MIN,
        width: None,
        runner: None,
    }
}

/// A script that records each time it starts, then waits `seconds`.
fn recording(repo: &TempDir, name: &str, seconds: u32) -> ResolvedCheck {
    let script = repo.path().join(format!("{name}.sh"));
    std::fs::write(
        &script,
        format!("echo started >> {name}.log\nsleep {seconds}\n"),
    )
    .expect("the script is written");
    named(name, &format!("sh {name}.sh"))
}

fn starts(repo: &TempDir, name: &str) -> usize {
    std::fs::read_to_string(repo.path().join(format!("{name}.log")))
        .map(|log| log.lines().count())
        .unwrap_or(0)
}

/// Wait until `name`'s script has written its `n`th line: a row reads started
/// a moment before the process does anything, and an act in that gap ends a
/// command that never printed.
async fn until_started(repo: &TempDir, name: &str, n: usize) {
    for _ in 0..500 {
        if starts(repo, name) >= n {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("{name} never started {n} time(s)");
}

/// The name of the one Check whose row reads started.
fn rows_running(underway: &Underway) -> String {
    let step = ipc::StepId::from(&StepId::new(STEP));
    underway
        .on(&job(), &step)
        .and_then(|rows| {
            rows.checks
                .iter()
                .find(|check| check.started_at.is_some() && check.ran.is_none())
                .map(|check| check.name.clone())
        })
        .expect("one Check is running")
}

fn job() -> ipc::JobId {
    ipc::JobId::carried(JOB)
}

/// The gate's own writer over `underway`, with nobody listening.
fn announcing(underway: &Underway, repo: &TempDir) -> Announcing {
    Announcing::on(
        job(),
        StepId::new(STEP),
        Attempt::FIRST,
        underway.clone(),
        api::Broadcaster::new(),
        Arc::new(Stopped),
        &repo.path().display().to_string(),
        JOB,
    )
}

async fn gate(
    checks: &[ResolvedCheck],
    repo: &TempDir,
    announcing: &Announcing,
    at_once: usize,
) -> Vec<Completed> {
    ran(
        checks,
        &[],
        false,
        crate::checking::Reading::Whole,
        repo.path(),
        Duration::from_secs(60),
        &Room::ignoring_the_machine(ChecksAtOnce::of(at_once)),
        announcing,
        &std::collections::BTreeMap::new(),
        &[],
        None,
        &crate::checking::Stop::never(),
        None,
        Attempt::FIRST,
        None,
        None,
    )
    .await
}

/// Wait until `ready` holds of the gate's rows, then hand back its control.
async fn control_when(
    underway: &Underway,
    ready: impl Fn(&ipc::ChecksUnderway) -> bool,
) -> (Arc<Control>, Vec<bool>) {
    for _ in 0..500 {
        let step = StepId::new(STEP);
        if let Some(rows) = underway.on(&job(), &ipc::StepId::from(&step)) {
            if ready(&rows) {
                return underway.gate_of(&job()).expect("a gate is running");
            }
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the gate never reached the state the case waits for");
}

fn recorded(checks: &[ResolvedCheck], done: &[Completed]) -> Vec<core_model::CheckOutcome> {
    let observed: Vec<Observed> = done.iter().map(|one| one.observed.clone()).collect();
    Ran::against(checks, &observed)
        .expect("one observation per Check")
        .recorded()
        .iter()
        .map(|row| row.outcome)
        .collect()
}

#[tokio::test]
async fn skipping_a_running_check_ends_it_as_skipped_and_the_gate_rules_without_it() {
    let repo = TempDir::new();
    let underway = Underway::default();
    let announcing = announcing(&underway, &repo);
    let checks = [recording(&repo, "slow", 30), named("quick", "true")];

    let acting = async {
        let (control, finished) = control_when(&underway, |rows| {
            rows.checks[0].started_at.is_some() && rows.checks[1].ran.is_some()
        })
        .await;
        until_started(&repo, "slow", 1).await;
        control.skip(&finished, "slow").expect("a running Check skips");
    };
    let (done, ()) = tokio::join!(gate(&checks, &repo, &announcing, 4), acting);

    assert_eq!(done[0].observed, Observed::Waived);
    assert_eq!(done[1].observed, Observed::Command(Exit::Code(0)));
    let outcomes = recorded(&checks, &done);
    assert_eq!(outcomes[0], core_model::CheckOutcome::Skipped);
    assert_eq!(outcomes[1], core_model::CheckOutcome::Passed);
    let ruled = Ran::against(
        &checks,
        &done.iter().map(|one| one.observed.clone()).collect::<Vec<_>>(),
    )
    .expect("every Check observed");
    assert!(ruled.advances(), "a skip lets the step through");
    assert!(!ruled.all_passed(), "and is not a pass");
    assert_eq!(starts(&repo, "slow"), 1, "it was never started again");
}

#[tokio::test]
async fn skipping_every_check_ends_the_running_one_and_the_ones_still_waiting() {
    let repo = TempDir::new();
    let underway = Underway::default();
    let announcing = announcing(&underway, &repo);
    let checks = [
        recording(&repo, "first", 30),
        recording(&repo, "second", 30),
        recording(&repo, "third", 30),
    ];

    let acting = async {
        let (control, finished) = control_when(&underway, |rows| {
            rows.checks.iter().any(|check| check.started_at.is_some())
        })
        .await;
        let running = rows_running(&underway);
        until_started(&repo, &running, 1).await;
        control.skip_all(&finished).expect("Checks are left to skip");
    };
    // One place: one runs and the other two wait for it.
    let (done, ()) = tokio::join!(gate(&checks, &repo, &announcing, 1), acting);

    assert!(done.iter().all(|one| one.observed == Observed::Waived));
    let started: usize = ["first", "second", "third"]
        .iter()
        .map(|name| starts(&repo, name))
        .sum();
    assert_eq!(started, 1, "only the one that had a place ever ran");
    assert!(recorded(&checks, &done)
        .iter()
        .all(|outcome| *outcome == core_model::CheckOutcome::Skipped));
}

#[tokio::test]
async fn retrying_a_finished_check_runs_that_one_again_and_the_ruling_waits_for_it() {
    let repo = TempDir::new();
    let underway = Underway::default();
    let announcing = announcing(&underway, &repo);
    std::fs::write(
        repo.path().join("once.sh"),
        "echo started >> once.log\nsleep 1\n",
    )
    .expect("the script is written");
    let checks = [named("once", "sh once.sh"), recording(&repo, "hold", 30)];

    let acting = async {
        let (control, _) = control_when(&underway, |rows| rows.checks[0].ran.is_some()).await;
        control.retry("once").expect("a finished Check runs again");
        // The retried Check starts over, so its row reads waiting or running.
        for _ in 0..500 {
            if starts(&repo, "once") == 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let (control, finished) =
            control_when(&underway, |rows| rows.checks[0].ran.is_some()).await;
        control.skip(&finished, "hold").expect("the other still runs");
    };
    let (done, ()) = tokio::join!(gate(&checks, &repo, &announcing, 4), acting);

    assert_eq!(starts(&repo, "once"), 2, "the retried Check ran twice");
    assert_eq!(starts(&repo, "hold"), 1, "the other was left as it was");
    assert_eq!(done[0].observed, Observed::Command(Exit::Code(0)));
    assert_eq!(done[1].observed, Observed::Waived);
}

#[tokio::test]
async fn retrying_a_running_check_ends_it_and_starts_it_again() {
    let repo = TempDir::new();
    let underway = Underway::default();
    let announcing = announcing(&underway, &repo);
    let checks = [recording(&repo, "slow", 30)];

    let acting = async {
        let (control, _) =
            control_when(&underway, |rows| rows.checks[0].started_at.is_some()).await;
        until_started(&repo, "slow", 1).await;
        control.retry("slow").expect("a running Check runs again");
        for _ in 0..500 {
            if starts(&repo, "slow") == 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let (control, finished) = underway.gate_of(&job()).expect("still running");
        control.skip(&finished, "slow").expect("the second run skips");
    };
    let (done, ()) = tokio::join!(gate(&checks, &repo, &announcing, 4), acting);

    assert_eq!(starts(&repo, "slow"), 2);
    assert_eq!(done[0].observed, Observed::Waived);
}

#[tokio::test]
async fn an_act_is_refused_where_no_gate_runs_no_check_is_named_or_the_ruling_is_being_written() {
    let repo = TempDir::new();
    let underway = Underway::default();
    assert!(
        underway.gate_of(&job()).is_none(),
        "no gate is running Checks for a Job nobody started one for"
    );

    let announcing = announcing(&underway, &repo);
    let checks = [named("quick", "true")];
    let done = gate(&checks, &repo, &announcing, 4).await;
    assert_eq!(done[0].observed, Observed::Command(Exit::Code(0)));

    // The entry stands across the Judge's calls, and the loop is over.
    let (control, finished) = underway.gate_of(&job()).expect("held until the ruling");
    assert_eq!(
        control.skip(&finished, "quick"),
        Err(Unactable::NoGate),
        "the ruling no longer waits for anything"
    );
    assert_eq!(control.retry("quick"), Err(Unactable::NoGate));
    assert_eq!(control.skip_all(&finished), Err(Unactable::NoGate));

    drop(announcing);
    assert!(underway.gate_of(&job()).is_none(), "and gone once it is written");
}

#[tokio::test]
async fn a_name_the_gate_does_not_run_and_a_check_already_finished_are_refused_while_it_is_open() {
    let repo = TempDir::new();
    let underway = Underway::default();
    let announcing = announcing(&underway, &repo);
    let checks = [named("quick", "true"), recording(&repo, "slow", 30)];

    let acting = async {
        let (control, finished) = control_when(&underway, |rows| rows.checks[0].ran.is_some()).await;
        assert_eq!(
            control.skip(&finished, "nothing"),
            Err(Unactable::NoSuchCheck("nothing".into()))
        );
        assert_eq!(
            control.retry("nothing"),
            Err(Unactable::NoSuchCheck("nothing".into()))
        );
        assert_eq!(
            control.skip(&finished, "quick"),
            Err(Unactable::Finished("quick".into()))
        );
        control.skip_all(&finished).expect("the slow one is left");
    };
    let (done, ()) = tokio::join!(gate(&checks, &repo, &announcing, 4), acting);
    assert_eq!(done[0].observed, Observed::Command(Exit::Code(0)));
    assert_eq!(done[1].observed, Observed::Waived);
}

#[tokio::test]
async fn a_job_whose_gate_is_not_running_checks_is_refused_each_act_as_a_409() {
    use testkit::{FakeHarness, FakeWorkProduct};

    use crate::adrift::Adrift;
    use crate::daemon::Fleet;
    use crate::tests::daemon::{a_proposal_for, fitted_with, one};
    use crate::tests::drone_per_task::a_drone_per_task;

    let home = TempDir::new();
    let mut fittings = fitted_with(
        &home,
        FakeWorkProduct::changed(&["src/read.rs"]),
        FakeHarness::that_listens(),
    );
    fittings.starting().workflows = one(a_drone_per_task());
    let fleet = Fleet::assembled(fittings);
    let job = fleet
        .propose(a_proposal_for("bound the reader", "fixture-per-task"))
        .await
        .expect("a Job at the approval gate");
    let id = job.id();

    for refused in [
        fleet.skip_check(id, "test").await.map(|_| ()),
        fleet.skip_checks(id).await.map(|_| ()),
        fleet.retry_check(id, "test").await.map(|_| ()),
    ] {
        let Err(why @ Adrift::NoCheckToAct { .. }) = refused else {
            panic!("a Job with no gate running Checks is refused: {refused:?}");
        };
        let api::Refusal::IllegalMove(wire) = fleet.refusal(why) else {
            panic!("a refusal the machine does not admit is a 409");
        };
        assert_eq!(wire.code, "fleet.no_check_to_act");
    }
}
