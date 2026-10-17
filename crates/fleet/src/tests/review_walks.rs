//! A pull request's change held up for a walk, through a real Fleet, a scripted forge and a real
//! command in a bay. `docs/concepts/pull-review.md`, *Walk*.
//!
//! **The only process is the one the person's command starts**, a `sleep` a close ends, and the only
//! address is a listener the test binds, so nothing outside the test is reached.

use std::time::Duration;

use adapter_traits::slot_path;
use api::{Next, PullRequestPanel};
use ipc::{Event, OpenReviewWalk, ReviewWalk, ReviewWalkState, WalkHow, WalkPlace};
use std::sync::Arc;
use store::HeldReviewWalk;
use tokio::net::TcpListener;

use super::pull_request_panel::a_page;
use super::pull_requesting::{a_fleet, manifest, Fixture};
use crate::tests::tmp::TempDir;

fn open(place: WalkPlace) -> OpenReviewWalk {
    OpenReviewWalk { number: 12, place }
}

fn pulled(command: &str, address: &str) -> WalkPlace {
    WalkPlace { how: WalkHow::Pull, address: address.into(), command: command.into(), api: String::new() }
}

/// A fleet whose forge knows pull request 12, and the directory its first bay will be in.
fn a_fleet_with_a_bay(home: &TempDir) -> (Arc<Fixture>, ipc::ManifestId, String) {
    let fleet = a_fleet(home);
    fleet.vcs().now_pull_request_page(Some(a_page()));
    let (id, root) = manifest(&fleet);
    let bay = slot_path(&root, 1);
    std::fs::create_dir_all(&bay).expect("a bay to run in");
    (Arc::new(fleet), id, bay)
}

/// The next `review_walk.changed` that reaches `state`, and every state it passed through.
async fn until(watching: &mut api::Subscription, state: ReviewWalkState) -> (ReviewWalk, Vec<ReviewWalkState>) {
    let mut passed = Vec::new();
    loop {
        match tokio::time::timeout(Duration::from_secs(10), watching.next()).await.expect("an event") {
            Some(Next::Send(delivered)) => {
                if let Event::ReviewWalkChanged { walk } = delivered.event {
                    passed.push(walk.state);
                    if walk.state == state {
                        return (walk, passed);
                    }
                }
            }
            other => panic!("the stream ended: {other:?}"),
        }
    }
}

async fn held(fleet: &Fixture) -> Vec<HeldReviewWalk> {
    fleet.store().lock().await.review_walks_held().expect("read")
}

#[tokio::test]
async fn a_deployed_place_is_ready_at_once_and_holds_no_bay() {
    let home = TempDir::new();
    let (fleet, id, _) = a_fleet_with_a_bay(&home);
    let place = WalkPlace { how: WalkHow::Deployed, address: "https://pr-12.preview.invalid".into(), command: String::new(), api: String::new() };

    let walk = Arc::clone(&fleet).open_review_walk(id, open(place)).await.expect("opened");

    assert_eq!((walk.state, walk.slot, walk.address.as_str()), (ReviewWalkState::Ready, None, "https://pr-12.preview.invalid"));
    assert!(fleet.vcs().review_bays_leased().is_empty());
    assert!(held(&fleet).await.is_empty());
}

#[tokio::test]
async fn a_pulled_down_walk_holds_a_bay_serves_the_command_and_a_close_gives_everything_back() {
    let home = TempDir::new();
    let (fleet, id, bay) = a_fleet_with_a_bay(&home);
    let listening = TcpListener::bind("127.0.0.1:0").await.expect("a port");
    let address = format!("http://{}", listening.local_addr().expect("bound"));
    let mut watching = fleet.events().subscribe();

    let walk = Arc::clone(&fleet).open_review_walk(id.clone(), open(pulled("sleep 60", &address))).await.expect("opened");
    assert_eq!((walk.state, walk.number), (ReviewWalkState::CheckingOut, 12), "answered at once, before the checkout");
    let (ready, passed) = until(&mut watching, ReviewWalkState::Ready).await;
    assert_eq!(passed, [ReviewWalkState::CheckingOut, ReviewWalkState::Starting, ReviewWalkState::Ready]);
    assert_eq!((ready.id.as_str(), ready.slot, ready.address.as_str()), (walk.id.as_str(), Some(1), address.as_str()));

    // Held: leased for this pull request, not given back, written down, and the command runs in it.
    let leased = fleet.vcs().review_bays_leased();
    assert_eq!(leased.len(), 1);
    assert_eq!(leased[0].0, 12);
    assert!(fleet.vcs().review_bays_released().is_empty());
    let rows = held(&fleet).await;
    assert_eq!((rows.len(), rows[0].id.as_str(), rows[0].slot, rows[0].holder.as_str()), (1, walk.id.as_str(), 1, leased[0].1.as_str()));
    assert_eq!(rows[0].path, bay);
    assert!(fleet.servers().live_ids().len() == 1, "the person's command is up as a server");

    let closed = fleet.close_review_walk(id.clone(), walk.id.clone()).await.expect("closed");
    assert_eq!(closed.state, ReviewWalkState::Closed);
    until(&mut watching, ReviewWalkState::Closed).await;
    assert_eq!(fleet.vcs().review_bays_released(), [(1, leased[0].1.clone())], "the bay went back to the one that took it");
    assert!(held(&fleet).await.is_empty());
    assert!(fleet.servers().live_ids().is_empty(), "the server is stopped");

    // A close of a walk already closed is answered, and says nothing more.
    let again = fleet.close_review_walk(id, walk.id).await.expect("idempotent");
    assert_eq!(again.state, ReviewWalkState::Closed);
    assert_eq!(fleet.vcs().review_bays_released().len(), 1);
}

#[tokio::test]
async fn a_command_that_ends_before_the_address_answers_is_a_failed_walk_with_its_bay_already_back() {
    let home = TempDir::new();
    let (fleet, id, _) = a_fleet_with_a_bay(&home);
    let silent = TcpListener::bind("127.0.0.1:0").await.expect("a port");
    let address = format!("http://{}", silent.local_addr().expect("bound"));
    drop(silent);
    let mut watching = fleet.events().subscribe();

    Arc::clone(&fleet).open_review_walk(id, open(pulled("exit 3", &address))).await.expect("opened");
    let (failed, _) = until(&mut watching, ReviewWalkState::Failed).await;

    assert!(failed.failed.as_deref().is_some_and(|said| said.contains("ended before")), "{:?}", failed.failed);
    assert_eq!(failed.slot, None);
    // The bay is back by the time the event says so.
    assert_eq!(fleet.vcs().review_bays_released().len(), 1);
    assert!(held(&fleet).await.is_empty());
}

#[tokio::test]
async fn a_bay_a_dead_fleet_held_is_given_back_on_the_next_walk_call() {
    let home = TempDir::new();
    let (fleet, id, bay) = a_fleet_with_a_bay(&home);
    // What the last Fleet left: a leased bay and its row, with nothing here serving it.
    let (_, root) = manifest(&fleet);
    let pool = crate::leasing::pool_of(&fleet.repositories().first().expect("served"));
    let adapter_traits::ReviewBay::Took { slot, path, .. } =
        adapter_traits::Vcs::lease_review_bay(fleet.vcs().as_ref(), &pool, 12, "review-walk-lost").expect("leased")
    else {
        panic!("a free pool gives a bay");
    };
    assert_eq!((slot, path.as_str()), (1, bay.as_str()));
    fleet
        .store()
        .lock()
        .await
        .keep_review_walk(&HeldReviewWalk {
            id: "lost".into(),
            manifest_id: id.as_str().into(),
            number: 12,
            slot,
            holder: "review-walk-lost".into(),
            path,
            opened_at: fleet.now().as_str().into(),
        })
        .expect("kept");
    assert!(fleet.vcs().slot_holders(&root).iter().any(Option::is_some));
    let mut watching = fleet.events().subscribe();

    // Any review walk call finds it: here, closing a walk that was never held.
    fleet.close_review_walk(id, "never-held".into()).await.expect("closed");

    assert!(held(&fleet).await.is_empty());
    assert_eq!(fleet.vcs().review_bays_released(), [(1, "review-walk-lost".to_string())]);
    assert!(fleet.vcs().slot_holders(&root).iter().all(Option::is_none));
    let (closed, _) = until(&mut watching, ReviewWalkState::Closed).await;
    assert_eq!((closed.id.as_str(), closed.number), ("lost", 12));
}

#[tokio::test]
async fn a_place_that_cannot_be_walked_and_a_forge_that_will_not_give_the_head_are_refused_before_a_bay_is_taken() {
    let home = TempDir::new();
    let (fleet, id, _) = a_fleet_with_a_bay(&home);

    let no_command = Arc::clone(&fleet).open_review_walk(id.clone(), open(pulled("", "http://localhost:3000"))).await.unwrap_err();
    assert_eq!((no_command.status(), no_command.error().code.as_str()), (422, "fleet.walk_place_invalid"));

    fleet.vcs().now_pull_request_page(None);
    let silent = Arc::clone(&fleet).open_review_walk(id, open(pulled("sleep 60", "http://localhost:3000"))).await.unwrap_err();
    assert_eq!((silent.status(), silent.error().code.as_str()), (500, "fleet.review_pull_request_unreadable"));
    assert!(fleet.vcs().review_bays_leased().is_empty());
}
