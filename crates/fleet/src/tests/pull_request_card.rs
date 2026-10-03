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
use crate::tests::under_review::{published, remarks_changed};

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

fn open() -> Landing {
    Landing::Open {
        url: String::from(PULL_REQUEST),
        rendering: Rendering::AsWritten,
    }
}

/// One conversation comment, as `gh pr view`'s reduction folds it: the
/// forge's node id and its address, and no place in the diff.
fn read_with_one_conversation_comment() -> UnderReview {
    UnderReview {
        people: WhatPeopleSaid::NobodyHasLooked,
        checks: WhatTheForgeRan::NothingRan,
        remarks: vec![Remark::written(
            "IC_kwDOAbc123",
            "somebody",
            "2026-10-02T10:00:00Z",
            "why this bound?",
        )
        .with_url("https://forge.invalid/armada/pull/1#issuecomment-1")],
        verdicts: Vec::new(),
    }
}

/// One comment on a line of the diff, as the review-comments reduction folds
/// it: the REST id, its address, and where in the diff it sits.
fn a_line_comment(id: &str, line: u32, said: &str) -> Remark {
    Remark::written(id, "a-reviewer", "2026-10-02T10:05:00Z", said)
        .with_url(format!(
            "https://forge.invalid/armada/pull/1#discussion_r{id}"
        ))
        .with_inline("src/log.rs", line, "@@ -40,3 +40,3 @@ fn read() {")
}

fn two_line_comments() -> Option<Vec<Remark>> {
    Some(vec![
        a_line_comment("2001", 41, "this leaks a file handle"),
        a_line_comment("2002", 44, "and this one"),
    ])
}

/// **The owner's 2 Oct decision**: the card's count includes the comments on
/// lines of the diff, so it matches what the forge shows. One conversation
/// comment and two line comments read 3, at one extra forge call on the turn
/// that found the pull request open.
#[tokio::test]
async fn line_comments_on_the_diff_are_in_the_comment_count() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home, Delivering::default());
    let job_id = a_finished_job(&fleet, &home).await;

    fleet.vcs().now_landed(open());
    fleet
        .vcs()
        .now_under_review(read_with_one_conversation_comment());
    fleet.vcs().now_inline_remarks(two_line_comments());
    let asked = fleet.vcs().times_asked_for_inline_remarks();
    fleet.turn().await.unwrap();

    assert_eq!(
        fleet.vcs().times_asked_for_inline_remarks(),
        asked + 1,
        "one extra forge call on the open turn"
    );
    assert_eq!(
        delivery_of(&fleet, &job_id).await.pull_request_comments,
        Some(3)
    );
    let asked = fleet.vcs().times_asked_for_inline_remarks();
    delivery_of(&fleet, &job_id).await;
    assert_eq!(
        fleet.vcs().times_asked_for_inline_remarks(),
        asked,
        "a Job read asks the forge nothing"
    );
}

/// A forge that answers the conversation but not the line comments keeps the
/// last count, rather than showing a smaller number or 0.
#[tokio::test]
async fn an_unanswered_line_comment_read_keeps_the_last_count() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home, Delivering::default());
    let job_id = a_finished_job(&fleet, &home).await;

    fleet.vcs().now_landed(open());
    fleet
        .vcs()
        .now_under_review(read_with_one_conversation_comment());
    fleet.vcs().now_inline_remarks(two_line_comments());
    fleet.turn().await.unwrap();
    fleet.vcs().now_inline_remarks(None);
    fleet.turn().await.unwrap();

    assert_eq!(
        delivery_of(&fleet, &job_id).await.pull_request_comments,
        Some(3),
        "the silence is not a count"
    );
}

/// **A new comment on a line of the diff wakes Bridge**, as a new conversation
/// comment does. The sweep fetches them for the count, so the signature that
/// decides `job.remarks_changed` reads them too.
#[tokio::test]
async fn a_new_line_comment_publishes_job_remarks_changed() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home, Delivering::default());
    let job_id = a_finished_job(&fleet, &home).await;
    fleet.vcs().now_landed(open());
    fleet
        .vcs()
        .now_under_review(read_with_one_conversation_comment());
    fleet
        .vcs()
        .now_inline_remarks(Some(vec![a_line_comment("2001", 41, "this leaks")]));
    fleet.turn().await.unwrap();

    let mut subscription = fleet.events().subscribe();
    fleet.vcs().now_inline_remarks(two_line_comments());
    fleet.turn().await.unwrap();

    let seen = published(&mut subscription).await;
    assert_eq!(
        remarks_changed(&seen),
        vec![&ipc::JobId::from(&job_id)],
        "the Job whose pull request gained a line comment, named once: {seen:?}"
    );
}

/// **A silence is not a change.** A line-comment read that goes unanswered,
/// then answers what it answered before, wakes Bridge on neither turn.
#[tokio::test]
async fn an_unanswered_line_comment_read_publishes_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet_asking_every_turn(&home, Delivering::default());
    a_finished_job(&fleet, &home).await;
    fleet.vcs().now_landed(open());
    fleet
        .vcs()
        .now_under_review(read_with_one_conversation_comment());
    fleet.vcs().now_inline_remarks(two_line_comments());
    fleet.turn().await.unwrap();

    let mut subscription = fleet.events().subscribe();
    fleet.vcs().now_inline_remarks(None);
    fleet.turn().await.unwrap();
    fleet.vcs().now_inline_remarks(two_line_comments());
    fleet.turn().await.unwrap();

    let seen = published(&mut subscription).await;
    assert!(
        remarks_changed(&seen).is_empty(),
        "an unanswered read, then the same answer: {seen:?}"
    );
}
