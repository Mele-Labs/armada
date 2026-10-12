//! What `get_job` says a quiet Job is waiting on.
//!
//! The wait is worked out from the board as it stands, so these drive the real
//! read, as `queued` does for the Board's label: a Job is put behind a peer,
//! then behind the slot, and its detail is asked for. The Job that is working,
//! and the step between states, are `silence/waiting.rs`.

use api::Queries;
use ipc::JobWaiting;
use testkit::{FakeJudge, FakeWorkProduct};

use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_fleet, a_fleet_proposing_through, a_proposal, worktree_directory};
use crate::tests::planning::A_PLAN;
use crate::tests::proposing::a_catalogue;
use crate::tests::tmp::TempDir;

async fn waiting(fleet: &impl Queries, job: &core_model::JobId) -> Vec<JobWaiting> {
    fleet
        .get_job(ipc::JobId::from(job))
        .await
        .expect("the Job reads")
        .waiting
}

/// A Job held behind a peer names that peer, and what it is called.
#[tokio::test]
async fn a_job_queued_behind_a_dependency_names_the_job() {
    let home = TempDir::new();
    let fleet = a_fleet_proposing_through(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        a_catalogue(),
        FakeJudge::saying(A_PLAN),
    );
    let made = fleet
        .propose_from("two coupled changes", None)
        .await
        .expect("a plan");
    worktree_directory(&home, &made[1]);
    dispatched(&fleet, made[1].id())
        .await
        .expect("approval lands");

    assert_eq!(
        waiting(&fleet, made[1].id()).await,
        [JobWaiting::Job {
            job_id: ipc::JobId::from(made[0].id()),
            title: Some(made[0].title().as_str().to_string()),
        }]
    );
}

/// A Job held only by the slot names the resource, spelled as its row spells it.
#[tokio::test]
async fn a_job_queued_on_capacity_names_the_resource() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    let first = fleet.propose(a_proposal("the first")).await.unwrap();
    worktree_directory(&home, &first);
    dispatched(&fleet, first.id()).await.unwrap();
    let second = fleet.propose(a_proposal("the second")).await.unwrap();
    worktree_directory(&home, &second);
    dispatched(&fleet, second.id()).await.unwrap();

    assert_eq!(
        waiting(&fleet, second.id()).await,
        [JobWaiting::Resource {
            name: "waiting_on_resources".to_string()
        }]
    );
    assert_eq!(
        waiting(&fleet, first.id()).await,
        [],
        "the Job in the slot is waiting on nothing"
    );
}
