//! Fleet answers while it is still reconciling, and a command waits for it.
//!
//! A real Check and a real child, for `crate::tests::reading_a_gate`'s reason:
//! the claim is about one request landing while a ruling is genuinely running.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::http::StatusCode;
use core_model::{JobStatus, StepId, StepState};
use ipc::RunId;
use testkit::{FakeWorkProduct, Gate, Sketch};

use crate::daemon::Fleet;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, diff_evidence, fittings, one, worktree_directory};
use crate::tests::http::call;
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

/// The Check's length, which is the ruling's.
const THE_RULING: Duration = Duration::from_secs(2);

fn a_step_whose_check_takes_a_while() -> config::ResolvedWorkflow {
    testkit::resolved(&[Sketch {
        id: "implement",
        label: "Implement",
        evidence_type: Some("diff"),
        gates: &[Gate::Check {
            name: "slow",
            run: "/bin/sleep 2",
            expect_exit_code: 0,
            when: &[],
        }],
        judged_on: &[],
        scope: None,
        gaming: None,
    }])
}

fn a_fleet(home: &TempDir) -> Fleet<testkit::FakeHarness, testkit::FakeVcs, FakeWorkProduct> {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.starting().workflows = one(a_step_whose_check_takes_a_while());
    Fleet::assembled(fittings)
}

/// A Job whose Drone submitted and whose Fleet went away before the gate ran.
async fn left_pending(home: &TempDir) -> core_model::JobId {
    let fleet = a_fleet(home);
    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .expect("a Job to work");
    worktree_directory(home, &job);
    dispatched(&fleet, job.id()).await.expect("it dispatches");
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the Drone reports its diff");
    job.id().clone()
}

#[tokio::test(flavor = "multi_thread")]
async fn health_answers_while_the_boot_ruling_runs_and_a_command_waits_for_it() {
    let home = TempDir::new();
    let job_id = left_pending(&home).await;

    let fleet = Arc::new(a_fleet(&home));
    let reconciliation = api::Reconciliation::begun();
    let app = api::router(
        api::Served::sharing(Arc::clone(&fleet), RunId::carried("01RUN"), fleet.events())
            .reconciling(&reconciliation),
    );
    let started = Instant::now();
    let reconciling = tokio::spawn({
        let fleet = Arc::clone(&fleet);
        let reconciliation = reconciliation.clone();
        async move {
            let reconciled = fleet.reconcile().await;
            reconciliation.finished();
            reconciled
        }
    });

    let (status, _) = call(&app, "GET", "/health", "").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        started.elapsed() < THE_RULING,
        "health took {:?}, which is the length of the ruling",
        started.elapsed()
    );
    assert!(
        !reconciling.is_finished(),
        "the ruling was over before health asked"
    );

    let waiting = tokio::time::timeout(
        Duration::from_millis(300),
        call(&app, "POST", "/preferences/save", "{}"),
    )
    .await;
    assert!(
        waiting.is_err(),
        "a command was answered mid-reconciliation"
    );

    let reconciled = reconciling.await.expect("the task").expect("the boot read");
    assert_eq!(reconciled.recovered_evidence, vec![job_id.clone()]);
    assert!(
        reconciled.interrupted.is_empty()
            && reconciled.restarted.is_empty()
            && reconciled.adopted.is_empty(),
        "the ruled Job was also interrupted, restarted or adopted: {reconciled:?}"
    );

    let (status, _) = call(&app, "POST", "/preferences/save", "{}").await;
    assert_ne!(status, StatusCode::SERVICE_UNAVAILABLE);

    let ruled = fleet.load(&job_id).await.unwrap();
    assert_eq!(
        ruled
            .step(&StepId::new("implement"))
            .expect("the step")
            .state(),
        StepState::Advanced
    );
    assert_ne!(ruled.status(), JobStatus::Escalated);
    assert!(fleet
        .store()
        .lock()
        .await
        .pending_evidence()
        .expect("the table reads")
        .is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn with_nothing_pending_a_command_passes_once_reconciliation_is_over() {
    let home = TempDir::new();
    let fleet = Arc::new(a_fleet(&home));
    let reconciliation = api::Reconciliation::begun();
    let app = api::router(
        api::Served::sharing(Arc::clone(&fleet), RunId::carried("01RUN"), fleet.events())
            .reconciling(&reconciliation),
    );
    let reconciled = fleet.reconcile().await.expect("the boot read");
    assert!(reconciled.recovered_evidence.is_empty());
    reconciliation.finished();
    let answered = tokio::time::timeout(
        Duration::from_secs(1),
        call(&app, "POST", "/preferences/save", "{}"),
    )
    .await;
    assert!(
        answered.is_ok(),
        "a command still waits after reconciliation"
    );
}
