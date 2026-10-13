//! `get_issue`: one issue by number, read from the tracker each time. Closed issues are found;
//! a number the tracker does not have, and a pull request's, are none; a tracker that will not
//! answer is a fault.

use std::sync::Arc;

use api::{Issues, Refusal};
use ipc::{IssueState, ManifestId};
use testkit::{FakeHarness, FakeTracker, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

fn manifest(fleet: &Hosted) -> ManifestId {
    let served = fleet.repositories().first().expect("a served repository");
    ManifestId::carried(served.manifest().id().as_str())
}

fn a_fleet(home: &TempDir, tracker: Arc<FakeTracker>) -> Hosted {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    fittings.tracker = tracker;
    Fleet::assembled(fittings)
}

#[tokio::test]
async fn an_open_issue_is_found_with_its_address() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::listing(vec![FakeTracker::issue(214, "The title")]));
    let fleet = a_fleet(&home, tracker);
    let read = fleet.get_issue(manifest(&fleet), 214).await.expect("found");
    assert_eq!(read.number, 214);
    assert_eq!(read.title, "The title");
    assert_eq!(read.url, "https://forge.invalid/o/r/issues/214");
    assert_eq!(read.state, IssueState::Open);
}

#[tokio::test]
async fn a_closed_issue_is_found_too() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::empty());
    tracker.knows_closed(31, "Long since done");
    let fleet = a_fleet(&home, tracker);
    let read = fleet.get_issue(manifest(&fleet), 31).await.expect("found");
    assert_eq!(read.state, IssueState::Closed);
    assert_eq!(read.title, "Long since done");
}

/// The tracker answers none for a pull request's number, as the real one does by its address.
#[tokio::test]
async fn a_number_the_tracker_does_not_have_is_a_404() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, Arc::new(FakeTracker::empty()));
    match fleet.get_issue(manifest(&fleet), 7).await {
        Err(Refusal::NoSuchJob(error)) => assert_eq!(error.code, "fleet.no_such_issue"),
        other => panic!("not found, not {other:?}"),
    }
}

#[tokio::test]
async fn a_tracker_that_will_not_answer_is_a_fault_and_an_unserved_repository_is_a_422() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::empty());
    tracker.will_not_list("not signed in");
    let fleet = a_fleet(&home, tracker);
    match fleet.get_issue(manifest(&fleet), 7).await {
        Err(Refusal::Fault(error)) => {
            assert_eq!(error.code, "fleet.issues_unreadable");
            assert!(error.message.contains("not signed in"), "{}", error.message);
        }
        other => panic!("a fault, not {other:?}"),
    }
    match fleet.get_issue(ManifestId::carried("not-served"), 7).await {
        Err(Refusal::Unacceptable(_)) => {}
        other => panic!("unacceptable, not {other:?}"),
    }
}
