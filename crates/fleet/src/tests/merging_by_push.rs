//! `merge_by:` decides which road a merge takes: the forge's, or a merge
//! commit Fleet pushes onto the base itself.
//!
//! The push is scripted here as the forge is in `merging`; what `git` does is
//! asserted in `adapters`. What is under test is that the Manifest picks the
//! road, and that everything after a merge is the same on both.

use std::time::Duration;

use adapter_traits::{Landing, NotMerged, RepositoryStanding};
use config::Manifest;
use core_model::{JobStatus, StepId, StepState};
use testkit::{Delivered, FakeWorkProduct, Merging};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::noticing::Noticing;
use crate::tests::daemon::{fittings, one, two_steps_gated_on_a_person};
use crate::tests::merging::{at_the_gate_having_delivered, Fixture, PULL_REQUEST};
use crate::tests::tmp::TempDir;

/// The tip the fast-forward leaves after the push.
const PUSHED: &str = "9c1a5e0000000000000000000000000000000000";

fn manifest(text: &str) -> Manifest {
    Manifest::parse(
        std::path::Path::new("armada.yml"),
        &format!("version: 1\nid: 01FIXTUREMANIFEST\n{text}"),
    )
    .expect("a manifest")
}

/// A Fleet holding a delivered Job for a person, under `armada.yml` `text`.
fn holding_the_work_under(home: &TempDir, text: &str) -> Fixture {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.starting().workflows = one(two_steps_gated_on_a_person(
        "summarise",
        None,
        Some("summarise"),
    ));
    fittings.starting().manifest = manifest(text);
    fittings.noticing = Noticing::every(Duration::ZERO);
    Fleet::assembled(fittings)
}

/// **The default keeps today's road**, so a repository that says nothing is
/// merged by the forge exactly as before the key existed.
#[tokio::test]
async fn a_manifest_that_says_nothing_merges_through_the_forge() {
    let home = TempDir::new();
    let fleet = holding_the_work_under(&home, "");
    let job_id = at_the_gate_having_delivered(&fleet, &home).await;
    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(PULL_REQUEST),
    });

    fleet.merge_pull_request(&job_id).await.expect("it merges");

    assert_eq!(fleet.vcs().times_asked_to_merge(), 1);
    assert_eq!(fleet.vcs().times_pushed_onto_the_base(), 0);
}

#[tokio::test]
async fn merge_by_forge_said_outright_is_the_same_road() {
    let home = TempDir::new();
    let fleet = holding_the_work_under(&home, "merge_by: forge\n");
    let job_id = at_the_gate_having_delivered(&fleet, &home).await;
    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(PULL_REQUEST),
    });

    fleet.merge_pull_request(&job_id).await.expect("it merges");

    assert_eq!(fleet.vcs().times_asked_to_merge(), 1);
    assert_eq!(fleet.vcs().times_pushed_onto_the_base(), 0);
}

/// **`push` never asks the forge to merge**, and the record is written from
/// the push rather than from a forge that may not have caught up yet — the
/// fake's forge still says the pull request is open.
#[tokio::test]
async fn merge_by_push_lands_the_work_without_the_forge_merging_it() {
    let home = TempDir::new();
    let fleet = holding_the_work_under(&home, "merge_by: push\n");
    let job_id = at_the_gate_having_delivered(&fleet, &home).await;
    let handle = fleet.load(&job_id).await.expect("the Job").handle();

    let job = fleet
        .merge_pull_request(&job_id)
        .await
        .expect("the push landed it");

    assert_eq!(job.status(), JobStatus::CompletedSuccess);
    assert_eq!(fleet.vcs().times_asked_to_merge(), 0);
    assert!(
        fleet.vcs().delivered().contains(&Delivered::MergedByPush {
            handle,
            pull_request: Some(1),
        }),
        "the Job's own branch, with its pull request named in the merge"
    );
    let landed = fleet
        .store()
        .lock()
        .await
        .delivery_for(&job_id)
        .expect("the delivery reads back")
        .landed;
    assert_eq!(
        landed,
        Some(Landing::Merged {
            url: String::from(PULL_REQUEST)
        })
    );
}

/// **A base that moved past the branch is refused, not merged**, with its own
/// code: what answers it is the branch brought up and gated again, never a
/// forge setting.
#[tokio::test]
async fn a_base_that_moved_is_refused_with_its_own_code_and_moves_nothing() {
    let home = TempDir::new();
    let fleet = holding_the_work_under(&home, "merge_by: push\n");
    let job_id = at_the_gate_having_delivered(&fleet, &home).await;
    fleet.vcs().merging(Merging::Refuses(NotMerged::BaseMoved {
        said: String::from("the candidate does not hold main"),
    }));

    let refused = fleet
        .merge_pull_request(&job_id)
        .await
        .expect_err("nothing gated that combination");
    assert!(matches!(refused, Adrift::NotMerged { .. }), "{refused}");
    let wire = match fleet.refusal(refused) {
        api::Refusal::IllegalMove(wire) => wire,
        other => panic!("the person's to answer, so a conflict: {other:?}"),
    };
    assert_eq!(wire.code, "fleet.merge_base_moved");

    let held = fleet.load(&job_id).await.expect("the Job is there");
    assert_eq!(held.status(), JobStatus::AwaitingReview);
    assert_eq!(
        held.step(&StepId::new("summarise".to_string()))
            .map(|step| step.state()),
        Some(StepState::AwaitingHuman),
    );
}

/// **What merged is proved on both roads.** The push names the base, the
/// repository is brought up to it, and the after-merge Checks run against the
/// commit that left.
#[tokio::test]
async fn merge_by_push_proves_the_commit_the_merge_left_behind() {
    let home = TempDir::new();
    let fleet = holding_the_work_under(
        &home,
        "merge_by: push\nchecks:\n  green:\n    run: \"true\"\nafter_merge:\n  checks: [green]\n",
    );
    let job_id = at_the_gate_having_delivered(&fleet, &home).await;
    fleet
        .vcs()
        .repository_standing(RepositoryStanding::MovedOn {
            base: String::from("main"),
            commits: 1,
            head: String::from(PUSHED),
        });

    fleet.merge_pull_request(&job_id).await.expect("it merges");

    for _ in 0..200 {
        let turned = fleet.turn().await.unwrap();
        if let Some(run) = turned.proved.first() {
            assert_eq!(run.at_commit, PUSHED);
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("no proof run came back within two seconds");
}
