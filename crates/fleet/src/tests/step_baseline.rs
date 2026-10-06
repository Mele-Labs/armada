//! A step's baseline survives being put back to work.
//!
//! `diff_nonempty` compares the worktree at the gate against what it held when
//! the step began. A step that wrote its fix, lost its Drone and was restarted
//! used to be measured from the restart, so its own uncommitted work read as
//! inherited and no run of it could pass. Job 12, 6 Oct: five attempts, every
//! other Check green. `crate::dispatch::Fleet::marked` is where the baseline is
//! kept and loaded.
//!
//! The fake work product models what the real one now guarantees: a footprint
//! is the same in a second process, so one Fleet's reading is comparable
//! against the next one's. `crate::tests::restarting` holds the rebase half.

use std::sync::Arc;

use adapter_traits::Change;
use core_model::{JobId, JobStatus, StepId, StepState};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct, Gate, Sketch};

use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::session::LiveSession;
use crate::tests::admitted::{dispatched, started};
use crate::tests::daemon::{a_proposal, diff_evidence, fitted_with, one, worktree_directory};
use crate::tests::planted::the_drone_it_holds_is_gone;
use crate::tests::restarting::stopped;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const IMPLEMENT: &str = "implement";

/// One step gated on `diff_nonempty` and on nothing else, so the only thing that
/// can fail it is the baseline.
fn gated() -> config::ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: IMPLEMENT,
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[Gate::DiffNonempty],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

/// A Fleet over `home`'s store. Called twice with one `home`, the second is a
/// Fleet that restarted.
fn a_fleet_over(home: &TempDir, work: FakeWorkProduct, harness: FakeHarness) -> Fixture {
    let mut fittings = fitted_with(home, work, harness);
    fittings.starting().workflows = one(gated());
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about a baseline"));
    Fleet::assembled(fittings)
}

/// A Drone that stays up until a case ends it, so a verdict can still be told
/// to it and what it submitted is waiting when the Fleet stops.
fn a_drone_that_stays() -> FakeHarness {
    FakeHarness::running("/bin/sh", &["-c", "echo BUSY; sleep 30"])
}

/// Until the ended Drone's slot is empty, ending it again where the first kill
/// missed: a signal sent before the process group exists is lost, and the Drone
/// then lives out its `sleep 30`.
async fn until_reaped(fleet: &Fixture) {
    for turn in 0..4000 {
        fleet.turn().await.expect("a turn");
        let slot = fleet.the_only_slot().await;
        let held = slot.lock().await;
        let Some(working) = held.as_ref() else {
            return;
        };
        if turn % 40 == 39 {
            let _ = working.session().terminate().await;
        }
        drop(held);
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    panic!("the Drone never left");
}

fn a_fix() -> [(&'static str, Change); 1] {
    [("src/parse.rs", Change::Modified)]
}

/// Restart the stopped step and submit, as the Drone that took it over would,
/// without writing anything.
async fn restarted_and_submitted(fleet: &Fixture, job: &JobId) -> Result<(), String> {
    fleet.restart_step(job, None).await.expect("a restart");
    started(fleet, job).await.expect("the fresh Drone is on");
    submitted_by_the_one(fleet, diff_evidence())
        .await
        .expect("the tool took it");
    let turned = fleet.turn().await.expect("the gate ruled");
    match turned.ruled() {
        Some(Ruling::Advanced { .. } | Ruling::Finished { .. }) => Ok(()),
        Some(Ruling::Failed { failures, .. }) => Err(format!("failed on {failures:?}")),
        other => Err(format!("{other:?}")),
    }
}

/// **Job 12's second attempt.** The first run wrote the fix and was stopped; the
/// restart's Drone writes nothing, and the step passes because the work it is
/// measured on is the first run's.
#[tokio::test]
async fn a_restarted_step_keeps_the_work_its_first_run_did() {
    let home = TempDir::new();
    let fleet = a_fleet_over(&home, FakeWorkProduct::inherited(&[]), a_drone_that_stays());
    let job = stopped(&fleet, &home).await;
    fleet.work().wrote(&a_fix());
    the_drone_it_holds_is_gone(&fleet).await;
    until_reaped(&fleet).await;

    let ruled = restarted_and_submitted(&fleet, &job).await;
    assert_eq!(
        ruled,
        Ok(()),
        "the first run's edit stopped counting toward the step"
    );
}

/// **Job 12's third attempt.** The same, across a Fleet that stopped and a new
/// one that read the baseline back off the store.
#[tokio::test]
async fn a_step_restarted_after_a_fleet_restart_keeps_the_work_its_first_run_did() {
    let home = TempDir::new();
    let job = {
        let fleet = a_fleet_over(&home, FakeWorkProduct::inherited(&[]), a_drone_that_stays());
        let job = stopped(&fleet, &home).await;
        fleet.work().wrote(&a_fix());
        the_drone_it_holds_is_gone(&fleet).await;
        until_reaped(&fleet).await;
        job
    };

    let work = FakeWorkProduct::inherited(&[]);
    work.wrote(&a_fix());
    let second = a_fleet_over(&home, work, a_drone_that_stays());
    let ruled = restarted_and_submitted(&second, &job).await;
    assert_eq!(
        ruled,
        Ok(()),
        "the restarted Fleet measured the step from where it was picked up"
    );
}

/// **Job 12's fifth attempt.** Evidence the Drone submitted and the Fleet died
/// before ruling on is ruled on at boot against the step's own start, where it
/// used to be ruled on as though Fleet had never seen the step begin.
#[tokio::test]
async fn evidence_submitted_before_a_restart_is_ruled_on_against_the_step_start() {
    let home = TempDir::new();
    let job = {
        let fleet = a_fleet_over(&home, FakeWorkProduct::inherited(&[]), a_drone_that_stays());
        let job = fleet
            .propose(a_proposal("fix the off-by-one"))
            .await
            .expect("a Job");
        worktree_directory(&home, &job);
        dispatched(&fleet, job.id()).await.expect("released to run");
        fleet.work().wrote(&a_fix());
        submitted_by_the_one(&fleet, diff_evidence())
            .await
            .expect("the tool took it");
        job.id().clone()
    };

    let work = FakeWorkProduct::inherited(&[]);
    work.wrote(&a_fix());
    let second = a_fleet_over(&home, work, a_drone_that_stays());
    let reconciled = second.reconcile().await.expect("the boot read");
    assert_eq!(reconciled.recovered_evidence, vec![job.clone()]);

    let carried = second.load(&job).await.expect("the Job");
    assert_eq!(
        carried
            .step(&StepId::new(IMPLEMENT))
            .expect("the step")
            .state(),
        StepState::Advanced,
        "the boot ruling read the step as one nothing moved in"
    );
    assert_eq!(
        carried.status(),
        JobStatus::CompletedSuccess,
        "the only step delivers, and the ruling finished the Job"
    );
}

/// **The rule the fix must not loosen.** A step whose first run edited nothing
/// still fails, whether it is put back to work in the same Fleet or in the next.
#[tokio::test]
async fn a_restarted_step_whose_first_run_wrote_nothing_still_fails() {
    for across_a_restart in [false, true] {
        let home = TempDir::new();
        let started_with = || FakeWorkProduct::inherited(&["SCOPE.md"]);
        let first = a_fleet_over(&home, started_with(), a_drone_that_stays());
        let job = stopped(&first, &home).await;
        the_drone_it_holds_is_gone(&first).await;
        until_reaped(&first).await;

        let fleet = match across_a_restart {
            true => {
                drop(first);
                a_fleet_over(&home, started_with(), a_drone_that_stays())
            }
            false => first,
        };
        let ruled = restarted_and_submitted(&fleet, &job).await;
        assert_eq!(
            ruled,
            Err(format!(
                "failed on {:?}",
                [verification::CheckFailed::DiffEmpty]
            )),
            "a restart that wrote nothing (across a restart: {across_a_restart})"
        );
    }
}

/// **Main moves every few minutes, so most resumes rebase.** The first run
/// edited one file; the requeue's catch-up is clean and writes a different one.
/// The rebase's file is inherited and the step's own stays counted, so the
/// restarted step passes with no new edit.
#[tokio::test]
async fn a_clean_catch_up_into_another_file_does_not_take_the_step_s_work_with_it() {
    let home = TempDir::new();
    let work = FakeWorkProduct::inherited(&[]);
    let mut fittings = fitted_with(&home, FakeWorkProduct::inherited(&[]), a_drone_that_stays());
    fittings.starting().workflows = one(gated());
    fittings.vcs = FakeVcs::new()
        .delivering(testkit::Delivering {
            standing: adapter_traits::Standing::Behind { commits: 1 },
            rebase: Some(adapter_traits::BroughtUpToDate::Clean {
                base: String::from("main"),
                commits: 1,
            }),
            ..testkit::Delivering::default()
        })
        .writing_into(work.holding(), &["src/other.rs"]);
    fittings.work = work;
    fittings.judge = Arc::new(FakeJudge::that_fails("no model is asked about a baseline"));
    let fleet = Fleet::assembled(fittings);

    let job = stopped(&fleet, &home).await;
    fleet.work().wrote(&a_fix());
    the_drone_it_holds_is_gone(&fleet).await;
    until_reaped(&fleet).await;

    let ruled = restarted_and_submitted(&fleet, &job).await;
    assert_eq!(
        ruled,
        Ok(()),
        "the rebase's own file took the step's work into the baseline"
    );
}
