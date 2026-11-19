//! The forge's rate limit on the Reviews lists: said as the limit with its time, and the lists
//! last read kept while Fleet waits it out.

use adapter_traits::{ListFailure, ReviewList};
use api::PullRequestPanel;
use ipc::WireValue;

use super::pull_requesting::{a_fleet, code};
use super::pull_review_kept::next_changed;
use super::pull_review_lists::{pull, signed_in_as, SLUG};
use crate::tests::tmp::TempDir;

/// A time well ahead of any clock a test runs on, in seconds since the epoch: 2100-01-01.
const FAR: u64 = 4_102_444_800;
const LIMIT: &str = "The forge is limiting us";
const NO_TOKEN: &str = "The forge has no token here. Sign in.";
const SILENT: &str = "The forge did not answer.";
const LISTS: [ReviewList; 4] = [ReviewList::Requested, ReviewList::Team, ReviewList::Assigned, ReviewList::Mine];

#[tokio::test]
async fn every_list_rate_limited_with_nothing_kept_says_the_limit_and_when() {
    let home = TempDir::new();
    let fleet = std::sync::Arc::new(a_fleet(&home));
    signed_in_as(&fleet, "nick");
    fleet.vcs().now_review_lists_fail(ListFailure::RateLimited { until: Some(FAR), said: LIMIT.into() });
    let refused = fleet.clone().get_review_lists(None).await.unwrap_err();
    assert_eq!(code(&refused), ipc::REVIEW_LISTS_RATE_LIMITED);
    assert_eq!(refused.error().message, LIMIT);
    assert_eq!(refused.error().fields.get("until"), Some(&WireValue::Str("2100-01-01T00:00:00.000Z".into())));
}

#[tokio::test]
async fn a_rate_limit_that_names_no_time_carries_no_until() {
    let home = TempDir::new();
    let fleet = std::sync::Arc::new(a_fleet(&home));
    signed_in_as(&fleet, "nick");
    fleet.vcs().now_review_lists_fail(ListFailure::RateLimited { until: None, said: LIMIT.into() });
    let refused = fleet.clone().get_review_lists(None).await.unwrap_err();
    assert_eq!(code(&refused), ipc::REVIEW_LISTS_RATE_LIMITED);
    assert!(!refused.error().fields.contains_key("until"));
}

#[tokio::test]
async fn no_token_and_an_unanswering_forge_are_said_in_their_own_sentences() {
    let home = TempDir::new();
    let fleet = std::sync::Arc::new(a_fleet(&home));
    signed_in_as(&fleet, "nick");
    fleet.vcs().now_review_lists_fail(ListFailure::NoToken(NO_TOKEN.into()));
    let refused = fleet.clone().get_review_lists(None).await.unwrap_err();
    assert_eq!(code(&refused), "fleet.review_lists_unreadable");
    assert_eq!(refused.error().message, NO_TOKEN);

    fleet.vcs().now_review_lists_fail(ListFailure::Unreachable(SILENT.into()));
    let refused = fleet.clone().get_review_lists(None).await.unwrap_err();
    assert_eq!(refused.error().message, SILENT);
}

#[tokio::test]
async fn while_held_fleet_asks_nothing_and_shows_the_lists_it_last_read_marked() {
    let home = TempDir::new();
    let fleet = std::sync::Arc::new(a_fleet(&home));
    signed_in_as(&fleet, "nick");
    for list in LISTS {
        fleet.vcs().now_review_list(list, vec![pull(7, SLUG)]);
    }
    let live = fleet.clone().get_review_lists(None).await.expect("read");
    assert!(live.rate_limited.is_none());

    for list in LISTS {
        fleet.vcs().review_list_silent(list);
    }
    fleet.vcs().now_review_lists_fail(ListFailure::RateLimited { until: Some(FAR), said: LIMIT.into() });
    // The kept lists answer at once. The limit is found by the read behind them, and told.
    let mut watching = fleet.events().subscribe();
    let at_once = fleet.clone().get_review_lists(None).await.expect("kept lists");
    assert!(at_once.refreshing && at_once.rate_limited.is_none());
    let kept = next_changed(&mut watching).await.lists.expect("the lists last read, marked");
    assert_eq!(kept.fetched_at, live.fetched_at, "the time they were read, for the banner");
    assert_eq!(kept.requested.len(), 1);
    let limited = kept.rate_limited.expect("marked");
    assert_eq!(limited.until.expect("its time").as_str(), "2100-01-01T00:00:00.000Z");
    assert_eq!(limited.said, LIMIT, "the forge's sentence rides with the mark");

    let asked = fleet.vcs().review_lists_asked().len();
    let again = fleet.clone().get_review_lists(None).await.expect("still kept");
    assert!(again.rate_limited.is_some() && !again.refreshing, "held: nothing is read behind them");
    assert_eq!(fleet.vcs().review_lists_asked().len(), asked, "held: no list was asked");
}

#[tokio::test]
async fn when_the_hold_lifts_fleet_asks_again() {
    let home = TempDir::new();
    let fleet = std::sync::Arc::new(a_fleet(&home));
    signed_in_as(&fleet, "nick");
    for list in LISTS {
        fleet.vcs().now_review_list(list, vec![pull(7, SLUG)]);
    }
    fleet.clone().get_review_lists(None).await.expect("read");
    for list in LISTS {
        fleet.vcs().review_list_silent(list);
    }
    // A reset already past: the wait is over as soon as it is made.
    fleet.vcs().now_review_lists_fail(ListFailure::RateLimited { until: Some(1), said: LIMIT.into() });
    let mut watching = fleet.events().subscribe();
    fleet.clone().get_review_lists(None).await.expect("kept");
    assert!(next_changed(&mut watching).await.lists.expect("kept").rate_limited.is_some());

    fleet.vcs().now_review_list(ReviewList::Mine, vec![pull(9, SLUG)]);
    let asked = fleet.vcs().review_lists_asked().len();
    fleet.clone().get_review_lists(None).await.expect("kept");
    let live = next_changed(&mut watching).await.lists.expect("read");
    assert!(live.rate_limited.is_none());
    assert_eq!(live.mine.len(), 1);
    assert_eq!(fleet.vcs().review_lists_asked().len(), asked + 4);
}

#[tokio::test]
async fn one_list_rate_limited_while_others_answer_stays_a_silent_list() {
    let home = TempDir::new();
    let fleet = std::sync::Arc::new(a_fleet(&home));
    signed_in_as(&fleet, "nick");
    fleet.vcs().now_review_lists_fail(ListFailure::RateLimited { until: Some(FAR), said: LIMIT.into() });
    fleet.vcs().now_review_list(ReviewList::Requested, vec![pull(1, SLUG)]);
    let lists = fleet.clone().get_review_lists(None).await.expect("read");
    assert_eq!(lists.silent.len(), 3);
    assert!(lists.rate_limited.is_none());
}
