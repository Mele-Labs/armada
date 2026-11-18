//! The Reviews lists kept: answered at once, across a restart, with the forge read behind them and
//! told as `review_lists.changed`. **No forge is called**: `FakeVcs` answers as told, and a held list
//! read is a forge that has not answered yet.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{ListFailure, ReviewList};
use api::{Next, PullRequestPanel};
use ipc::{Event, ReviewListsChanged};
use store::review_lists_kept::KeptReviewLists;

use super::pull_requesting::{a_fleet, Fixture};
use super::pull_review_lists::{pull, signed_in_as, SLUG};
use crate::tests::tmp::TempDir;

const LISTS: [ReviewList; 4] = [ReviewList::Requested, ReviewList::Team, ReviewList::Assigned, ReviewList::Mine];
const NO_TOKEN: &str = "The forge has no token here. Sign in.";

fn forge_says(fleet: &Fixture, number: u64) {
    for list in LISTS {
        fleet.vcs().now_review_list(list, vec![pull(number, SLUG)]);
    }
}

fn a_signed_in_fleet(home: &TempDir) -> Arc<Fixture> {
    let fleet = a_fleet(home);
    signed_in_as(&fleet, "nick");
    Arc::new(fleet)
}

/// A Fleet that has read the lists once, with pull request 7 on each.
async fn having_read(home: &TempDir) -> Arc<Fixture> {
    let fleet = a_signed_in_fleet(home);
    forge_says(&fleet, 7);
    let first = fleet.clone().get_review_lists(None).await.expect("read");
    assert!(!first.refreshing, "nothing was kept, so the forge was read before the answer");
    fleet
}

/// The next `review_lists.changed`.
pub(super) async fn next_changed(watching: &mut api::Subscription) -> ReviewListsChanged {
    loop {
        match tokio::time::timeout(Duration::from_secs(10), watching.next()).await.expect("an event") {
            Some(Next::Send(delivered)) => {
                if let Event::ReviewListsChanged(told) = delivered.event {
                    return told;
                }
            }
            other => panic!("the stream ended: {other:?}"),
        }
    }
}

/// Until `n` list reads wait on the held forge, so a read is known to be going.
async fn until_waiting(hold: &testkit::Hold, n: usize) {
    for _ in 0..500 {
        if hold.waiting() >= n {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("{} list reads waiting, not {n}", hold.waiting());
}

fn numbers(rows: &[ipc::PullReviewRow]) -> Vec<u64> {
    rows.iter().map(|row| row.number).collect()
}

#[tokio::test]
async fn kept_lists_are_answered_at_once_with_refreshing_while_the_forge_is_held_back() {
    let home = TempDir::new();
    let fleet = having_read(&home).await;
    forge_says(&fleet, 8);
    let hold = fleet.vcs().hold_review_lists();
    let answered = fleet.clone().get_review_lists(None).await.expect("kept lists, at once");
    assert!(answered.refreshing, "the forge is being read behind them");
    assert_eq!(numbers(&answered.requested), [7], "the lists kept, not the forge's new ones");
    until_waiting(&hold, 4).await;
    hold.open();
}

#[tokio::test]
async fn lists_kept_survive_a_restart() {
    let home = TempDir::new();
    drop(having_read(&home).await);
    let restarted = a_signed_in_fleet(&home);
    forge_says(&restarted, 8);
    let hold = restarted.vcs().hold_review_lists();
    let answered = restarted.clone().get_review_lists(None).await.expect("the lists this Fleet's store holds");
    assert_eq!(numbers(&answered.requested), [7], "a Fleet that has read nothing answers what the last one read");
    assert!(answered.refreshing);
    assert_eq!(answered.me, "nick");
    hold.open();
}

#[tokio::test]
async fn the_event_is_published_when_the_read_lands_and_the_fresh_lists_are_kept() {
    let home = TempDir::new();
    let fleet = having_read(&home).await;
    forge_says(&fleet, 8);
    let mut watching = fleet.events().subscribe();
    let answered = fleet.clone().get_review_lists(None).await.expect("kept");
    assert_eq!(numbers(&answered.requested), [7]);
    let told = next_changed(&mut watching).await;
    assert_eq!(told.project, None);
    assert!(told.failed.is_none());
    let fresh = told.lists.expect("the fresh lists, whole");
    assert_eq!(numbers(&fresh.requested), [8]);
    assert!(!fresh.refreshing);
    // What a Bridge that reconnects is answered with: the fresh lists, kept.
    let next = fleet.clone().get_review_lists(None).await.expect("kept");
    assert_eq!(numbers(&next.requested), [8]);
}

#[tokio::test]
async fn a_forge_read_that_fails_leaves_the_kept_rows_and_publishes_why() {
    let home = TempDir::new();
    let fleet = having_read(&home).await;
    for list in LISTS {
        fleet.vcs().review_list_silent(list);
    }
    fleet.vcs().now_review_lists_fail(ListFailure::NoToken(NO_TOKEN.into()));
    let mut watching = fleet.events().subscribe();
    fleet.clone().get_review_lists(None).await.expect("kept");
    let told = next_changed(&mut watching).await;
    assert!(told.lists.is_none());
    let failed = told.failed.expect("why");
    assert_eq!((failed.code.as_str(), failed.said.as_str()), ("fleet.review_lists_unreadable", NO_TOKEN));
    let kept = fleet.store().lock().await.kept_review_lists("").expect("read").expect("the row is still there");
    assert!(kept.lists.contains("Pull request 7"), "{}", kept.lists);
    let next = fleet.clone().get_review_lists(None).await.expect("still kept");
    assert_eq!(numbers(&next.requested), [7]);
}

#[tokio::test]
async fn asked_twice_one_read_of_the_forge_is_in_flight() {
    let home = TempDir::new();
    let fleet = having_read(&home).await;
    forge_says(&fleet, 8);
    let hold = fleet.vcs().hold_review_lists();
    let before = fleet.vcs().review_lists_asked().len();
    let (first, second) = (fleet.clone().get_review_lists(None).await, fleet.clone().get_review_lists(None).await);
    assert!(first.expect("kept").refreshing && second.expect("kept").refreshing, "both are told it is being read");
    until_waiting(&hold, 4).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(fleet.vcs().review_lists_asked().len(), before + 4, "four lists, once");
    hold.open();
}

#[tokio::test]
async fn with_nothing_kept_a_second_ask_joins_the_read_going_rather_than_start_another() {
    let home = TempDir::new();
    let fleet = a_signed_in_fleet(&home);
    forge_says(&fleet, 7);
    let hold = fleet.vcs().hold_review_lists();
    let first = tokio::spawn({
        let fleet = fleet.clone();
        async move { fleet.get_review_lists(None).await }
    });
    until_waiting(&hold, 4).await;
    let second = tokio::spawn({
        let fleet = fleet.clone();
        async move { fleet.get_review_lists(None).await }
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    hold.open();
    let (first, second) = (first.await.expect("task").expect("read"), second.await.expect("task").expect("answered"));
    assert_eq!((numbers(&first.requested), numbers(&second.requested)), (vec![7], vec![7]));
    assert_eq!(fleet.vcs().review_lists_asked().len(), 4, "one read of the four lists answered both");
}

#[tokio::test]
async fn a_stored_row_that_does_not_decode_is_nothing_kept() {
    let home = TempDir::new();
    let fleet = a_signed_in_fleet(&home);
    forge_says(&fleet, 7);
    let older = KeptReviewLists { project: String::new(), fetched_at: "2026-11-01T00:00:00Z".into(), lists: r#"{"the_shape":"an older build wrote"}"#.into() };
    fleet.store().lock().await.keep_review_lists(&older).expect("kept");
    let answered = fleet.clone().get_review_lists(None).await.expect("read, not an error to the person");
    assert!(!answered.refreshing, "nothing was kept, so the forge was read before the answer");
    assert_eq!(numbers(&answered.requested), [7]);
    assert_eq!(fleet.vcs().review_lists_asked().len(), 4);
}
