//! His own latest review on a pull request's page and its rows, and what came since it, through a
//! real Fleet and a scripted forge. `docs/concepts/pull-review.md`, *Your review*.
//!
//! **The forge here lags on purpose**: after the write it still answers with the review before,
//! as a forge read just after a write can, and the page must not.

use adapter_traits::{
    CommitLine, DiffSide, LatestReview, LatestReviewSaid, PullRequestPageFacts, ReviewList, ReviewThreadFacts, ThreadSaid,
};
use api::PullRequestPanel;
use ipc::{MyReviewSaid, PullInbox, PullRequestWriteKind, PullSinceMyReview, WritePullRequest};

use super::pull_request_panel::a_page;
use super::pull_requesting::{a_fleet, manifest};
use super::pull_review_lists::{pull, signed_in_as, SLUG};
use crate::tests::tmp::TempDir;

const HEAD: &str = "3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e";
const OLD: &str = "1a2b3c4d1a2b3c4d1a2b3c4d1a2b3c4d1a2b3c4d";
/// Before the test clock's first reading, `2026-08-26T09:00:00Z`.
const REVIEWED_AT: &str = "2026-08-20T10:00:00Z";

fn said(id: &str, author: &str, at: &str) -> ThreadSaid {
    ThreadSaid { id: id.into(), author: author.into(), bot: false, at: at.into(), edited_at: None, body: "a word".into() }
}

fn thread(id: &str, comments: Vec<ThreadSaid>) -> ReviewThreadFacts {
    ReviewThreadFacts { id: id.into(), path: "a.rs".into(), line: Some(4), side: DiffSide::Right, resolved: false, comments }
}

fn commit(sha: &str) -> CommitLine {
    CommitLine { sha: sha.into(), headline: "A change".into(), author: None, at: None }
}

fn latest(said: LatestReviewSaid, commit: &str, at: &str) -> LatestReview {
    LatestReview { said, commit: Some(commit.into()), submitted_at: Some(at.into()) }
}

/// His comment review at `OLD`, two commits pushed after it, and a thread he wrote on that two
/// people answered since, beside one he never wrote on.
fn a_page_he_reviewed() -> PullRequestPageFacts {
    let mut page = a_page();
    page.commits = vec![commit("1a2b3c4"), commit("b42a108"), commit("3f9c1d2")];
    page.latest_reviews = [
        ("rowan-ashe".to_string(), latest(LatestReviewSaid::Commented, OLD, REVIEWED_AT)),
        ("ada".to_string(), latest(LatestReviewSaid::Approved, HEAD, "2026-08-21T10:00:00Z")),
    ]
    .into();
    page.threads = vec![
        thread(
            "PRRT_1",
            vec![
                said("C1", "Rowan-Ashe", "2026-08-19T10:00:00Z"),
                said("C2", "ada", "2026-08-19T12:00:00Z"),
                said("C3", "ada", "2026-08-21T09:00:00Z"),
                said("C4", "Rowan-Ashe", "2026-08-22T09:00:00Z"),
                said("C5", "grace", "2026-08-23T09:00:00Z"),
            ],
        ),
        thread("PRRT_2", vec![said("C6", "ada", "2026-08-21T09:00:00Z")]),
    ];
    page
}

fn approving() -> WritePullRequest {
    WritePullRequest {
        pull_request: 12,
        kind: PullRequestWriteKind::Approve,
        body: None,
        comments: Vec::new(),
        commit: None,
        session_id: None,
    }
}

#[tokio::test]
async fn the_page_carries_his_latest_review_and_the_commits_and_replies_since_it() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    signed_in_as(&fleet, "Rowan-Ashe");
    fleet.vcs().now_pull_request_page(Some(a_page_he_reviewed()));

    let page = fleet.get_pull_request_detail(id, 12).await.expect("read");
    let mine = page.my_review.expect("his review");
    assert_eq!(mine.said, MyReviewSaid::Commented, "his, not ada's");
    assert_eq!((mine.commit.as_deref(), mine.submitted_at.as_deref()), (Some(OLD), Some(REVIEWED_AT)));
    assert_eq!(
        page.since_my_review,
        Some(PullSinceMyReview { commits: Some(2), replies: Some(2) }),
        "two pushes after OLD; ada and grace answered his thread since, and the thread he never wrote on is not his"
    );
}

#[tokio::test]
async fn a_page_for_somebody_fleet_cannot_name_has_no_review_bar() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    signed_in_as(&fleet, "grace-hopper");
    fleet.vcs().now_pull_request_page(Some(a_page_he_reviewed()));
    let page = fleet.get_pull_request_detail(id, 12).await.expect("read");
    assert_eq!((page.my_review, page.since_my_review), (None, None), "grace-hopper has reviewed nothing");
}

#[tokio::test]
async fn after_he_approves_the_page_and_his_row_show_it_while_the_forge_still_answers_with_the_review_before() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    signed_in_as(&fleet, "Rowan-Ashe");
    // The forge lags: every read below still says his latest is the comment at OLD.
    fleet.vcs().now_pull_request_page(Some(a_page_he_reviewed()));
    let mut row = pull(12, SLUG);
    row.my_latest = Some(latest(LatestReviewSaid::Commented, OLD, REVIEWED_AT));
    fleet.vcs().now_review_list(ReviewList::Requested, vec![row]);

    let answered = fleet.write_pull_request(id.clone(), approving()).await.expect("written");
    let mine = answered.my_review.expect("his review");
    assert_eq!((mine.said, mine.commit.as_deref()), (MyReviewSaid::Approved, Some(HEAD)), "the branch as it stood");
    assert_eq!(answered.since_my_review, Some(PullSinceMyReview { commits: Some(0), replies: Some(0) }));

    let next = fleet.get_pull_request_detail(id.clone(), 12).await.expect("read");
    let mine = next.my_review.expect("his review");
    assert_eq!((mine.said, mine.commit.as_deref()), (MyReviewSaid::Approved, Some(HEAD)), "the next page is not the stale read");
    assert_eq!(next.since_my_review.and_then(|it| it.commits), Some(0));

    let lists = fleet.get_review_lists(None).await.expect("read");
    assert_eq!(lists.requested[0].my_review.as_ref().map(|it| it.said), Some(MyReviewSaid::Approved));
    assert_eq!(lists.requested[0].inbox, PullInbox::Done);

    // The forge catches up with a review newer than Fleet's: the forge's wins.
    let mut caught_up = a_page_he_reviewed();
    caught_up.latest_reviews.insert(
        "rowan-ashe".into(),
        latest(LatestReviewSaid::ChangesRequested, HEAD, "2026-09-01T00:00:00Z"),
    );
    fleet.vcs().now_pull_request_page(Some(caught_up));
    let later = fleet.get_pull_request_detail(id, 12).await.expect("read");
    assert_eq!(later.my_review.map(|it| it.said), Some(MyReviewSaid::ChangesRequested));
}
