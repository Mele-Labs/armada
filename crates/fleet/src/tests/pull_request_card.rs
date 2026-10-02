//! A Job's pull request keeps its title and comment count after it merges.
//!
//! Both used to live only in the sweep's memory, which is erased when a pull
//! request settles. A Job whose pull request merged before the rotation reached
//! it open never had a title at all; PR #1750 is the one that showed it.

use std::time::Duration;

use adapter_traits::{Landing, Remark, Rendering, UnderReview, WhatPeopleSaid, WhatTheForgeRan};
use api::Queries;
use testkit::{Delivering, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::noticing::Noticing;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_proposal, diff_evidence, fittings, note_evidence, worktree_directory,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<testkit::FakeHarness, FakeVcs, FakeWorkProduct>;

const PULL_REQUEST: &str = "https://forge.invalid/armada/pull/1";

/// A Fleet that asks the forge on every turn, against the forge `delivering`
/// scripts.
fn a_fleet_asking_every_turn(home: &TempDir, delivering: Delivering) -> Fixture {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.noticing = Noticing::every(Duration::ZERO);
    fittings.vcs = FakeVcs::new().delivering(delivering);
    Fleet::assembled(fittings)
}

/// Run the two-step fixture workflow to the end, which opens the pull request.
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

async fn delivery_of(fleet: &Fixture, job_id: &core_model::JobId) -> ipc::JobDelivery {
    fleet
        .get_job(ipc::JobId::from(job_id))
        .await
        .expect("the Job is served")
        .delivery
        .expect("the Job delivered")
}

fn merged() -> Landing {
    Landing::Merged {
        url: String::from(PULL_REQUEST),
    }
}

/// **The case PR #1750 was.** The forge already says merged on the first ask,
/// so the sweep never reads the pull request open, and the settling read names
/// no title. The title Fleet opened it with is still served.
#[tokio::test]
async fn a_pull_request_merged_before_any_poll_keeps_the_title_fleet_opened_it_with() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(
        &home,
        Delivering {
            landed: merged(),
            title: None,
            ..Delivering::default()
        },
    );
    let job_id = a_finished_job(&fleet, &home).await;
    fleet.turn().await.unwrap();

    assert_eq!(
        fleet.vcs().times_asked_what_is_under_review(),
        0,
        "the case is a pull request no sweep ever read open"
    );
    let delivery = delivery_of(&fleet, &job_id).await;
    assert_eq!(delivery.landed, Some(ipc::Settled::Merged));
    assert_eq!(
        delivery.pull_request_detail, None,
        "the live reading is gone"
    );
    let job = fleet.load(&job_id).await.expect("the Job");
    assert_eq!(
        delivery.pull_request_title.as_deref(),
        Some(job.title().as_str()),
        "the title Fleet opened the pull request with"
    );
    assert_eq!(
        delivery.pull_request_comments, None,
        "never counted, so unknown and not zero"
    );
}

/// A title edited on the forge replaces the one Armada opened with, and the
/// settling read is a read like any other.
#[tokio::test]
async fn the_forge_title_on_the_settling_read_is_the_one_kept() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(
        &home,
        Delivering {
            landed: merged(),
            title: Some(String::from("Retitled by a person on the forge")),
            ..Delivering::default()
        },
    );
    let job_id = a_finished_job(&fleet, &home).await;
    fleet.turn().await.unwrap();

    assert_eq!(
        delivery_of(&fleet, &job_id)
            .await
            .pull_request_title
            .as_deref(),
        Some("Retitled by a person on the forge")
    );
}

/// The count the sweep read while the pull request was open is still served
/// once it has merged, with no forge call on the read.
#[tokio::test]
async fn the_comment_count_reads_on_a_finished_job() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home, Delivering::default());
    let job_id = a_finished_job(&fleet, &home).await;

    fleet.vcs().now_landed(Landing::Open {
        url: String::from(PULL_REQUEST),
        rendering: Rendering::AsWritten,
    });
    fleet.vcs().now_under_review(UnderReview {
        people: WhatPeopleSaid::NobodyHasLooked,
        checks: WhatTheForgeRan::NothingRan,
        remarks: vec![
            Remark::written(
                "IC_1",
                "somebody",
                "2026-10-01T10:00:00Z",
                "why this bound?",
            ),
            Remark::written("IC_2", "somebody", "2026-10-01T10:05:00Z", "never mind"),
        ],
        verdicts: Vec::new(),
    });
    fleet.turn().await.unwrap();
    fleet.vcs().now_landed(merged());
    fleet.turn().await.unwrap();

    let delivery = delivery_of(&fleet, &job_id).await;
    assert_eq!(delivery.landed, Some(ipc::Settled::Merged));
    assert_eq!(delivery.pull_request_comments, Some(2));
    assert_eq!(
        delivery.pull_request_title.as_deref(),
        Some("a job's pull request"),
        "the forge's title, from the reads"
    );
    let asked = fleet.vcs().times_asked_what_became_of_it();
    delivery_of(&fleet, &job_id).await;
    assert_eq!(
        fleet.vcs().times_asked_what_became_of_it(),
        asked,
        "a Job read asks the forge nothing"
    );
}
