//! Where an approved Job lands, set once after its approval. Since 23.20.

use std::sync::Arc;

use api::{Commands, Queries, Refusal};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::daemon::{a_proposal, fittings};
use crate::tests::tmp::TempDir;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

fn a_fleet(home: &TempDir) -> Arc<Fixture> {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/read.rs"]));
    fittings.vcs = FakeVcs::new()
        .with_ref_at("main", "a".repeat(40))
        .with_ref_at("release/2.0", "c".repeat(40));
    Arc::new(Fleet::assembled(fittings))
}

fn code(refusal: &Refusal) -> &str {
    match refusal {
        Refusal::NoSuchJob(e)
        | Refusal::IllegalMove(e)
        | Refusal::Unacceptable(e)
        | Refusal::Fault(e) => &e.code,
    }
}

fn aim(target: &str) -> ipc::SetLandingTarget {
    ipc::SetLandingTarget {
        target: target.to_string(),
    }
}

async fn approved(fleet: &Arc<Fixture>, title: &str) -> core_model::JobId {
    let job = fleet.propose(a_proposal(title)).await.expect("a Job");
    Commands::approve_dispatch(Arc::clone(fleet), job.id().into(), None)
        .await
        .expect("approved as it stands");
    job.id().clone()
}

/// **An approved Job landing in the base is aimed once**, and `get_job` reads
/// it back; a second aim, a branch nobody holds and a blank one are refused.
#[tokio::test]
async fn an_approved_job_with_no_target_is_aimed_once() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let job = approved(&fleet, "the reader").await;
    let set =
        |target: &str| Commands::set_landing_target(Arc::clone(&fleet), (&job).into(), aim(target));

    assert_eq!(
        code(&set("nowhere").await.expect_err("unheld")),
        "fleet.no_such_branch"
    );
    assert_eq!(
        code(&set("  ").await.expect_err("blank")),
        "fleet.landing_target_blank"
    );
    set("release/2.0").await.expect("aimed");
    let detail = Queries::get_job(&*fleet, (&job).into())
        .await
        .expect("the detail");
    assert_eq!(
        detail.landing.and_then(|landing| landing.target).as_deref(),
        Some("release/2.0")
    );
    assert_eq!(
        code(&set("main").await.expect_err("aimed already")),
        "fleet.landing_target_settled"
    );
}

/// **Not at the gate, where the approval sets it, and not once the work went
/// out**, since its pull request opens against the target.
#[tokio::test]
async fn a_target_is_not_set_at_the_gate_or_once_the_work_went_out() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let gated = fleet
        .propose(a_proposal("at the gate"))
        .await
        .expect("a Job");
    let refused =
        Commands::set_landing_target(Arc::clone(&fleet), gated.id().into(), aim("release/2.0"))
            .await
            .expect_err("at its gate");
    assert_eq!(code(&refused), "fleet.landing_target_settled");

    let job = approved(&fleet, "went out").await;
    fleet
        .store()
        .lock()
        .await
        .record_delivery(
            &job,
            &store::Delivery {
                commit: Some("d".repeat(40)),
                ..store::Delivery::default()
            },
        )
        .expect("recorded");
    let refused =
        Commands::set_landing_target(Arc::clone(&fleet), (&job).into(), aim("release/2.0"))
            .await
            .expect_err("the work went out");
    assert_eq!(code(&refused), "fleet.landing_target_settled");
}
