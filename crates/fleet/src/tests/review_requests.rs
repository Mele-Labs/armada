//! A review asked of him by name, through a real Fleet and a scripted forge: told once a request, across a
//! restart too, standing as a call, and counted until he opens the pull request.

use std::sync::{Arc, Mutex};

use adapter_traits::{LatestReview, LatestReviewSaid, ReviewList};
use api::PullRequestPanel;
use ipc::{CallArrival, CallArrivalKind, CallTone, Event, MarkPullSeen, StandingCall, StandingCallKind};

use super::pull_request_panel::a_page;
use super::pull_requesting::{a_fleet, manifest, Fixture};
use super::pull_review_lists::{pull, signed_in_as, SLUG};
use crate::tests::tmp::TempDir;

/// What `pull` says the head is.
const HEAD: &str = "3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e3f9c1d2e";

/// What the route does for a page Bridge reads: the page, then the open.
async fn opens(fleet: &Fixture, id: &ipc::ManifestId, number: u64) {
    fleet.get_pull_request_detail(id.clone(), number).await.expect("read");
    fleet.pull_request_opened(id.clone(), number).await;
}

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

/// How many times `calls.changed` is sent from now on.
fn changes(fleet: &Fixture) -> Arc<Mutex<usize>> {
    let sent: Arc<Mutex<usize>> = Arc::default();
    let seen = Arc::clone(&sent);
    fleet.events().tap(move |event| {
        if let Event::CallsChanged(_) = event {
            *seen.lock().expect("not poisoned") += 1;
        }
    });
    sent
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
async fn the_reviews_count_falls_when_he_has_opened_the_pull_request() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    signed_in_as(&fleet, "NickMele");
    let mut reviewed = pull(14, SLUG);
    reviewed.my_latest = reviewed_at("2026-01-02T09:00:00Z");
    fleet.vcs().now_review_list(ReviewList::Requested, vec![pull(12, SLUG), pull(13, SLUG), reviewed]);
    fleet.review_requests_looked().await;
    assert_eq!(fleet.calls_read().expect("the calls").reviews_unopened(), 3, "a request asked again after his review is unopened too");

    // Bridge marks it seen when he leaves the pull request: the count falls then, not at the next tick.
    fleet.mark_pull_request_seen(id, MarkPullSeen { number: 12, head: HEAD.into() }).await.expect("kept");
    assert_eq!(fleet.calls_read().expect("the calls").reviews_unopened(), 2);
    fleet.review_requests_looked().await;
    let calls = reviews(&fleet);
    let opened: Vec<(u64, bool)> = calls.iter().map(|one| (one.review.as_ref().expect("named").number, one.opened)).collect();
    assert_eq!(opened, [(12, true), (13, false), (14, false)]);
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

#[tokio::test]
async fn a_page_read_for_his_pull_request_opens_its_review_call_and_one_for_another_does_not() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    signed_in_as(&fleet, "NickMele");
    fleet.vcs().now_review_list(ReviewList::Requested, vec![pull(12, SLUG), pull(13, SLUG)]);
    fleet.vcs().now_pull_request_page(Some(a_page()));
    fleet.review_requests_looked().await;
    assert_eq!(fleet.calls_read().expect("the calls").reviews_unopened(), 2);

    opens(&fleet, &id, 14).await;
    assert_eq!(fleet.calls_read().expect("the calls").reviews_unopened(), 2, "another pull request's page opens nothing");

    // Bridge reads the page when he opens the pull request: the count falls then, not when he leaves it.
    let sent = changes(&fleet);
    opens(&fleet, &id, 12).await;
    assert_eq!(fleet.calls_read().expect("the calls").reviews_unopened(), 1);
    opens(&fleet, &id, 12).await;
    assert_eq!(*sent.lock().expect("not poisoned"), 1, "a page read again changes no call");

    fleet.review_requests_looked().await;
    let opened: Vec<(u64, bool)> = reviews(&fleet).iter().map(|one| (one.review.as_ref().expect("named").number, one.opened)).collect();
    assert_eq!(opened, [(12, true), (13, false)], "the next look keeps it opened");
}

#[test]
fn a_page_read_opens_a_request_only_when_read_after_the_review_before_it() {
    use crate::review_requests::read_since;
    assert!(read_since("", "2026-01-01T09:00:00.000Z"), "a first request: any read");
    assert!(read_since("2026-01-02T09:00:00Z", "2026-01-02T09:00:01.000Z"));
    assert!(!read_since("2026-01-02T09:00:00Z", "2026-01-01T09:00:00.000Z"), "a read before his review was of the request before");
}

#[tokio::test]
async fn a_tick_forgets_the_requests_no_longer_asked_of_him_and_tells_one_asked_again() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    signed_in_as(&fleet, "NickMele");
    let told = told(&fleet);
    fleet.vcs().now_review_list(ReviewList::Requested, vec![pull(12, SLUG), pull(13, SLUG)]);
    fleet.vcs().now_pull_request_page(Some(a_page()));
    fleet.review_requests_looked().await;
    opens(&fleet, &id, 12).await;

    // Taken back, reviewed or closed: the forge lists it no more.
    fleet.vcs().now_review_list(ReviewList::Requested, vec![pull(13, SLUG)]);
    fleet.review_requests_looked().await;
    let kept = || async {
        let store = fleet.store().lock().await;
        let told: Vec<u64> = store.review_requests_told().expect("read").iter().map(|it| it.number).collect();
        let opened: Vec<u64> = store.pull_requests_opened().expect("read").iter().map(|it| it.number).collect();
        (told, opened)
    };
    assert_eq!(kept().await, (vec![13], vec![]));

    fleet.vcs().review_list_silent(ReviewList::Requested);
    fleet.review_requests_looked().await;
    assert_eq!(kept().await, (vec![13], vec![]), "a tick the forge did not answer forgets nothing");

    fleet.vcs().now_review_list(ReviewList::Requested, vec![pull(12, SLUG), pull(13, SLUG)]);
    fleet.review_requests_looked().await;
    assert_eq!(told.lock().expect("not poisoned").len(), 3, "asked again after it was taken back is a new request");
    assert_eq!(fleet.calls_read().expect("the calls").reviews_unopened(), 2, "the read before it was of the request taken back");
}

#[tokio::test]
async fn a_review_asked_again_after_his_review_stays_unopened_until_he_reads_its_page() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    signed_in_as(&fleet, "NickMele");
    fleet.vcs().now_pull_request_page(Some(a_page()));
    // He read the page before his review, and was asked again after it.
    let before = store::KeptPullOpened { manifest_id: id.as_str().to_string(), number: 12, opened_at: "2026-01-01T09:00:00.000Z".into() };
    fleet.store().lock().await.keep_pull_request_opened(&before).expect("kept");
    let mut again = pull(12, SLUG);
    again.my_latest = reviewed_at("2026-01-02T09:00:00Z");
    fleet.vcs().now_review_list(ReviewList::Requested, vec![again]);

    fleet.review_requests_looked().await;
    assert_eq!(fleet.calls_read().expect("the calls").reviews_unopened(), 1, "his review and what he read before it do not open a request made after them");
    opens(&fleet, &id, 12).await;
    assert_eq!(fleet.calls_read().expect("the calls").reviews_unopened(), 0);
    fleet.review_requests_looked().await;
    assert_eq!(fleet.calls_read().expect("the calls").reviews_unopened(), 0, "read since the request, the next look keeps it opened");
}
