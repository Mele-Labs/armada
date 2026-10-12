//! A catch-up fetches the remote's base first, and a fetch that fails stops it.
//!
//! **Job 14 (12 Oct 2026)**: the catch-up merged the local `main`, which sits
//! hours behind the remote's, and logged "behind its base by 0 commit(s)" over
//! a pull request the forge showed as conflicting. What the adapter merges is
//! asserted against real git in `adapters`; what is asserted here is that Fleet
//! asks for the fetch before every catch-up, and that a fetch that fails never
//! falls back to a base it holds an old copy of.

use adapter_traits::{BroughtUpToDate, Landing, Rendering, Standing, Worktree};
use testkit::Delivered;

use crate::adrift::Adrift;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{a_proposal, worktree_directory};
use crate::tests::noticing::{a_finished_job, a_fleet_asking_every_turn};
use crate::tests::tmp::TempDir;

fn open_against_a_base_that_moved() -> Landing {
    Landing::Open {
        url: String::from("https://forge.invalid/armada/pull/1"),
        rendering: Rendering::FromASupersededBase {
            pinned: String::from("67cb1b9e"),
            written_on: String::from("8c2ce681"),
        },
    }
}

fn log_of(fleet: &crate::tests::noticing::Fixture, job: &core_model::JobId) -> String {
    let handle = fleet.name_of(job).expect("the Job has a name");
    let served = fleet.served_by_id(job).expect("served");
    std::fs::read_to_string(crate::transcript::log_of(served.records_root(), &handle))
        .unwrap_or_default()
}

fn caught_up(fleet: &crate::tests::noticing::Fixture) -> bool {
    fleet.vcs().delivered_with_fetches().iter().any(|it| {
        matches!(
            it,
            Delivered::BroughtUpToDate { .. } | Delivered::KeptCurrent { .. }
        )
    })
}

/// **The order is the rule.** Reading the base before fetching it is the bug.
#[tokio::test]
async fn the_sweeps_catch_up_is_asked_after_the_base_is_fetched() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home);
    a_finished_job(&fleet, &home).await;

    fleet.vcs().now_landed(open_against_a_base_that_moved());
    fleet.turn().await.unwrap();

    let did = fleet.vcs().delivered_with_fetches();
    let fetched = did
        .iter()
        .position(|it| matches!(it, Delivered::FetchedBase { .. }))
        .expect("the base was fetched");
    let kept = did
        .iter()
        .position(|it| matches!(it, Delivered::KeptCurrent { .. }))
        .expect("the branch was kept current");
    assert!(fetched < kept, "{did:?}");
}

/// A fetch that failed means nothing was merged, and the Job's log says which
/// remote, which base and what git said.
#[tokio::test]
async fn a_sweep_whose_fetch_fails_merges_nothing_and_says_why() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home);
    let job = a_finished_job(&fleet, &home).await;
    let before = fleet.vcs().times_kept_current();

    fleet
        .vcs()
        .now_unable_to_fetch("origin", "Permission denied (publickey).");
    fleet.vcs().now_landed(open_against_a_base_that_moved());
    fleet.turn().await.unwrap();

    assert_eq!(
        fleet.vcs().times_kept_current(),
        before,
        "a base this machine holds an old copy of is not merged"
    );
    let log = log_of(&fleet, &job);
    for named in ["\"level\":\"warn\"", "origin", "main", "Permission denied (publickey)"] {
        assert!(log.contains(named), "{named} missing from:\n{log}");
    }
}

/// **Nothing is recorded for a fetch that failed**, so the next sweep tries the
/// same base again rather than believing it was attempted.
#[tokio::test]
async fn a_failed_fetch_is_retried_by_the_next_sweep() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home);
    let job = a_finished_job(&fleet, &home).await;
    fleet.vcs().move_ref_to("main", "8c2ce681000000000000000000000000000000");

    fleet.vcs().now_unable_to_fetch("origin", "could not resolve host");
    fleet.vcs().now_landed(open_against_a_base_that_moved());
    fleet.turn().await.unwrap();
    assert!(fleet.store().lock().await.kept_current_for(&job).unwrap().onto.is_none());

    let fetches = |fleet: &crate::tests::noticing::Fixture| {
        fleet
            .vcs()
            .delivered_with_fetches()
            .iter()
            .filter(|it| matches!(it, Delivered::FetchedBase { .. }))
            .count()
    };
    let first = fetches(&fleet);
    fleet.turn().await.unwrap();
    assert!(fetches(&fleet) > first, "asked again");
}

/// The spawn's catch-up: a fetch that fails stops the step before a Drone is
/// put on a tree, and nothing is merged.
#[tokio::test]
async fn a_spawn_whose_fetch_fails_is_stopped_before_anything_is_merged() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home);
    fleet.vcs().now_behind(
        Standing::Behind { commits: 3 },
        Some(BroughtUpToDate::Clean {
            base: String::from("main"),
            commits: 3,
        }),
    );
    fleet.vcs().now_unable_to_fetch("origin", "Permission denied (publickey).");
    let job = fleet
        .propose(a_proposal("fix the off-by-one in the log reader"))
        .await
        .unwrap();
    worktree_directory(&home, &job);

    assert!(
        dispatched(&fleet, job.id()).await.is_err(),
        "the Job is not started on a base that could not be brought up to date"
    );
    assert!(!caught_up(&fleet), "{:?}", fleet.vcs().delivered_with_fetches());
    let log = log_of(&fleet, job.id());
    assert!(
        log.contains("origin") && log.contains("Permission denied (publickey)"),
        "{log}"
    );
}

/// The delivery's own catch-up fetches too, so the push after it is of a branch
/// that holds the base the forge holds.
#[tokio::test]
async fn a_delivery_whose_fetch_fails_pushes_and_opens_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home);
    let id = a_finished_job(&fleet, &home).await;
    let job = fleet.load(&id).await.unwrap();
    let worktree = Worktree::at("/nowhere", "armada/a-branch");
    fleet.vcs().now_behind(Standing::Behind { commits: 2 }, None);
    fleet.vcs().now_unable_to_fetch("origin", "Permission denied (publickey).");
    let before = fleet.vcs().delivered_with_fetches().len();

    let delivered = fleet.deliver(&job, &worktree).await;

    assert!(
        matches!(delivered, Err(Adrift::NotDelivered { .. })),
        "{delivered:?}"
    );
    let since = &fleet.vcs().delivered_with_fetches()[before..];
    assert!(
        since
            .iter()
            .all(|it| matches!(it, Delivered::FetchedBase { .. })),
        "only the fetch was attempted: {since:?}"
    );
}
