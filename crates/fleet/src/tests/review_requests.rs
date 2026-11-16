//! A review asked of him by name, through a real Fleet and a scripted forge: told once a request, across a
//! restart too, standing as a call, and counted until he opens the pull request.

use std::sync::{Arc, Mutex};

use adapter_traits::{LatestReview, LatestReviewSaid, ReviewList};
use api::PullRequestPanel;
use ipc::{CallArrival, CallArrivalKind, CallTone, Event, MarkPullSeen, StandingCall, StandingCallKind};

use super::pull_requesting::{a_fleet, manifest, Fixture};
use super::pull_review_lists::{pull, signed_in_as, SLUG};
use crate::tests::tmp::TempDir;

/// What `pull` says the head is.
const HEAD: &str = "3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e";

/// Every arrival `calls.changed` tells from now on.
fn told(fleet: &Fixture) -> Arc<Mutex<Vec<CallArrival>>> {
    let told: Arc<Mutex<Vec<CallArrival>>> = Arc::default();
    let seen = Arc::clone(&told);
    fleet.events().tap(move |event| {
        if let Event::CallsChanged(changed) = event {
            seen.lock().expect("not poisoned").extend(changed.arrived.clone());
        }
    });
    told
}

fn reviews(fleet: &Fixture) -> Vec<StandingCall> {
    let calls = fleet.calls_read().expect("the calls");
    calls.calls.into_iter().filter(|one| one.kind == StandingCallKind::ReviewRequested).collect()
}

fn reviewed_at(at: &str) -> Option<LatestReview> {
    Some(LatestReview { said: LatestReviewSaid::Commented, commit: Some(HEAD.into()), submitted_at: Some(at.into()) })
}

#[tokio::test]
async fn a_review_asked_of_him_by_name_is_told_once_and_stands_as_a_call() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    signed_in_as(&fleet, "NickMele");
    let told = told(&fleet);
    fleet.vcs().now_review_list(ReviewList::Requested, vec![pull(12, SLUG)]);

    assert_eq!(fleet.review_requests_looked().await, None, "a tick the forge answered says nothing");
    fleet.review_requests_looked().await;

    let arrived = told.lock().expect("not poisoned").clone();
    let key = format!("review:{SLUG}#12");
    assert_eq!(arrived.len(), 1, "told once: {arrived:?}");
    let one = &arrived[0];
    assert_eq!(
        (one.kind, one.key.as_deref(), one.session_id.as_str(), one.title.as_str(), one.text.as_str(), one.tone),
        (CallArrivalKind::ReviewRequested, Some(key.as_str()), key.as_str(), "Pull request 12", "Review requested on mele-labs/armada#12, opened by ada", CallTone::Waiting)
    );
    let calls = reviews(&fleet);
    assert_eq!(calls.len(), 1);
    let (call, review) = (&calls[0], calls[0].review.as_ref().expect("names the pull request"));
    assert_eq!((call.key.as_str(), call.owner.as_str(), call.session_id.as_ref(), call.opened), (key.as_str(), key.as_str(), None, false));
    assert_eq!(
        (&review.manifest_id, review.repository.as_str(), review.number, review.title.as_str(), review.author.as_str()),
        (&id, SLUG, 12, "Pull request 12", "ada")
    );
    assert_eq!(fleet.calls_read().expect("the calls").reviews_unopened(), 1);
    let asked: Vec<ReviewList> = fleet.vcs().review_lists_asked().iter().map(|(list, ..)| *list).collect();
    assert_eq!(asked, [ReviewList::Requested, ReviewList::Requested], "only the list that asks him by name; a team's is not his");
}

#[tokio::test]
async fn a_review_told_is_not_told_again_after_a_restart() {
    let home = TempDir::new();
    {
        let fleet = a_fleet(&home);
        signed_in_as(&fleet, "NickMele");
        fleet.vcs().now_review_list(ReviewList::Requested, vec![pull(12, SLUG)]);
        fleet.review_requests_looked().await;
    }
    let fleet = a_fleet(&home);
    signed_in_as(&fleet, "NickMele");
    let told = told(&fleet);
    fleet.vcs().now_review_list(ReviewList::Requested, vec![pull(12, SLUG), pull(13, SLUG)]);
    fleet.review_requests_looked().await;
    let keys: Vec<Option<String>> = told.lock().expect("not poisoned").iter().map(|one| one.key.clone()).collect();
    assert_eq!(keys, [Some(format!("review:{SLUG}#13"))], "only the request Fleet had not told");
    assert_eq!(reviews(&fleet).len(), 2, "both stand");
}

#[tokio::test]
async fn a_review_asked_again_after_his_review_is_a_new_request() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    signed_in_as(&fleet, "NickMele");
    let told = told(&fleet);
    fleet.vcs().now_review_list(ReviewList::Requested, vec![pull(12, SLUG)]);
    fleet.review_requests_looked().await;

    // The forge drops him from the request when he reviews, so listed again after a review he was asked again.
    let mut again = pull(12, SLUG);
    again.my_latest = reviewed_at("2026-01-02T09:00:00Z");
    fleet.vcs().now_review_list(ReviewList::Requested, vec![again.clone()]);
    fleet.review_requests_looked().await;
    fleet.review_requests_looked().await;
    assert_eq!(told.lock().expect("not poisoned").len(), 2, "told once for each request");
}

#[tokio::test]
async fn a_review_he_submitted_moments_ago_is_not_read_as_asked_again() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    signed_in_as(&fleet, "NickMele");
    let told = told(&fleet);
    fleet.vcs().now_review_list(ReviewList::Requested, vec![pull(12, SLUG)]);
    fleet.review_requests_looked().await;

    // The forge's search can list him a while after he reviewed: a review newer than a tick waits for the next.
    let mut lagging = pull(12, SLUG);
    lagging.my_latest = reviewed_at(fleet.clock().now().as_str());
    fleet.vcs().now_review_list(ReviewList::Requested, vec![lagging]);
    fleet.review_requests_looked().await;
    assert_eq!(told.lock().expect("not poisoned").len(), 1, "not told again while the review is fresh");
}

#[tokio::test]
async fn the_reviews_count_falls_when_he_has_opened_the_pull_request_or_reviewed_it() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    signed_in_as(&fleet, "NickMele");
    let mut reviewed = pull(14, SLUG);
    reviewed.my_latest = reviewed_at("2026-01-02T09:00:00Z");
    fleet.vcs().now_review_list(ReviewList::Requested, vec![pull(12, SLUG), pull(13, SLUG), reviewed]);
    fleet.review_requests_looked().await;
    assert_eq!(fleet.calls_read().expect("the calls").reviews_unopened(), 2, "a request he has reviewed before is opened");

    // Bridge marks it seen when he leaves the pull request: the count falls then, not at the next tick.
    fleet.mark_pull_request_seen(id, MarkPullSeen { number: 12, head: HEAD.into() }).await.expect("kept");
    assert_eq!(fleet.calls_read().expect("the calls").reviews_unopened(), 1);
    fleet.review_requests_looked().await;
    let calls = reviews(&fleet);
    let opened: Vec<(u64, bool)> = calls.iter().map(|one| (one.review.as_ref().expect("named").number, one.opened)).collect();
    assert_eq!(opened, [(12, true), (13, false), (14, true)]);
}

#[tokio::test]
async fn a_forge_that_does_not_answer_skips_the_tick_and_tells_nothing() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    signed_in_as(&fleet, "NickMele");
    fleet.vcs().now_review_list(ReviewList::Requested, vec![pull(12, SLUG)]);
    fleet.review_requests_looked().await;
    let told = told(&fleet);

    fleet.vcs().review_list_silent(ReviewList::Requested);
    let note = fleet.review_requests_looked().await.expect("a note for the log");
    assert!(note.contains("did not answer"), "{note}");
    assert!(told.lock().expect("not poisoned").is_empty());
    assert_eq!(reviews(&fleet).len(), 1, "what stood stands through a forge that did not answer");
}

#[tokio::test]
async fn nobody_signed_in_skips_the_tick() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    fleet.vcs().now_review_list(ReviewList::Requested, vec![pull(12, SLUG)]);
    let note = fleet.review_requests_looked().await.expect("a note for the log");
    assert!(note.contains("signed in"), "{note}");
    assert!(reviews(&fleet).is_empty());
}
