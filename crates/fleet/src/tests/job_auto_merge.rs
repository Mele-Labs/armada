//! Merge pressed on a Job's pull request while the forge's checks run: the ask is recorded, the
//! Job stays at its gate, and the sweep takes the work only for a merge that was asked for.

use adapter_traits::{Landing, PullRequestFacts, PullRequestStanding, UnderReview, WhatTheForgeRan};
use core_model::JobStatus;

use std::time::Duration;

use testkit::{Delivering, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::noticing::Noticing;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_proposal, diff_evidence, fittings, note_evidence, one, two_steps_gated_on_a_person,
    worktree_directory,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<testkit::FakeHarness, FakeVcs, FakeWorkProduct>;

/// A Fleet whose last step holds for a person, asking the forge on every turn.
fn a_fleet_asking_every_turn(home: &TempDir) -> Fixture {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.starting().workflows = one(two_steps_gated_on_a_person(
        "summarise",
        None,
        Some("summarise"),
    ));
    fittings.vcs = FakeVcs::new().delivering(Delivering::default());
    fittings.noticing = Noticing::every(Duration::ZERO);
    Fleet::assembled(fittings)
}

async fn a_finished_job(fleet: &Fixture, home: &TempDir) -> core_model::JobId {
    let job = fleet
        .propose(a_proposal("fix the off-by-one in the log reader"))
        .await
        .unwrap();
    worktree_directory(home, &job);
    dispatched(fleet, job.id()).await.unwrap();
    submitted_by_the_one(fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    submitted_by_the_one(fleet, note_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    job.id().clone()
}

const ADDRESS: &str = "https://forge.invalid/armada/pull/1";

fn checks_running(fleet: &Fixture) {
    fleet.vcs().now_pull_request(Some(PullRequestFacts {
        standing: PullRequestStanding::Open,
        branch: "fleet/fix".into(),
        auto_merge: false,
        title: "Fix".into(),
        url: ADDRESS.into(),
    }));
    fleet.vcs().now_under_review(UnderReview {
        checks: WhatTheForgeRan::StillWaiting { finished: 1, checks: 2 },
        ..UnderReview::unreadable()
    });
}

async fn status(fleet: &Fixture, job: &core_model::JobId) -> JobStatus {
    fleet.load(job).await.unwrap().status()
}

#[tokio::test]
async fn the_ask_is_recorded_and_the_job_stays_at_its_gate() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home);
    let job = a_finished_job(&fleet, &home).await;
    assert_eq!(status(&fleet, &job).await, JobStatus::AwaitingReview);
    checks_running(&fleet);

    let state = fleet.enable_job_auto_merge(&job).await.expect("the forge took it");

    assert!(state.auto_merge);
    assert_eq!(fleet.vcs().times_asked_for_auto_merge(), 1);
    assert!(fleet.store().lock().await.auto_merge_asked(&job).unwrap());
    assert_eq!(status(&fleet, &job).await, JobStatus::AwaitingReview);
}

#[tokio::test]
async fn the_sweep_takes_the_work_once_the_forge_merged_what_was_asked_for() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home);
    let job = a_finished_job(&fleet, &home).await;
    checks_running(&fleet);
    fleet.enable_job_auto_merge(&job).await.unwrap();

    fleet.vcs().now_landed(Landing::Merged { url: ADDRESS.into() });
    fleet.turn().await.unwrap();

    assert_ne!(status(&fleet, &job).await, JobStatus::AwaitingReview);
}

#[tokio::test]
async fn a_merge_nobody_asked_for_leaves_the_job_at_its_gate() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home);
    let job = a_finished_job(&fleet, &home).await;

    fleet.vcs().now_landed(Landing::Merged { url: ADDRESS.into() });
    fleet.turn().await.unwrap();

    assert_eq!(status(&fleet, &job).await, JobStatus::AwaitingReview);
}

#[tokio::test]
async fn a_press_off_the_gate_is_refused() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home);
    let job = a_finished_job(&fleet, &home).await;
    fleet.approve_review(&job).await.expect("taken");
    checks_running(&fleet);

    assert!(fleet.enable_job_auto_merge(&job).await.is_err());
    assert_eq!(fleet.vcs().times_asked_for_auto_merge(), 0);
}
