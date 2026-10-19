//! `get_issue`: one issue by reference, read from the tracker each time. Closed issues are found;
//! a number the tracker does not have, and a pull request's, are none; a tracker that will not
//! answer is a fault.

use std::sync::Arc;

use adapter_traits::{
    FromOutside, IssueComment, LinkedPullRequest, LinkedPullRequestState, ReadIssue,
};
use api::{Issues, Refusal};
use ipc::{IssueState, ManifestId};
use testkit::{FakeHarness, FakeTracker, FakeTrackers, FakeVcs, FakeWorkProduct};

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
    fittings.trackers = Arc::new(FakeTrackers::empty().tracker(tracker));
    Fleet::assembled(fittings)
}

#[tokio::test]
async fn an_open_issue_is_found_with_its_address() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::listing(vec![FakeTracker::issue(214, "The title")]));
    let fleet = a_fleet(&home, tracker);
    let read = fleet.get_issue(manifest(&fleet), "214".to_string()).await.expect("found");
    assert_eq!(read.reference, "#214");
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
    let read = fleet.get_issue(manifest(&fleet), "#31".to_string()).await.expect("found");
    assert_eq!(read.state, IssueState::Closed);
    assert_eq!(read.title, "Long since done");
}

/// A closed issue the tracker holds whole reaches the wire with every field, comments as the
/// tracker ordered them and each linked pull request's state in the wire's words.
#[tokio::test]
async fn an_issue_read_whole_carries_its_body_comments_and_linked_pull_requests() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::empty());
    let mut issue = ReadIssue::bare(
        "#31".to_string(),
        FromOutside::verbatim("Long since done"),
        FromOutside::verbatim("https://forge.invalid/o/r/issues/31"),
        true,
    );
    issue.body = FromOutside::verbatim("## Why\n\nIt breaks.");
    issue.author = FromOutside::verbatim("nick");
    issue.opened_at = "2026-10-01T09:00:00Z".to_string();
    issue.labels = vec![FromOutside::verbatim("bug")];
    issue.assignees = vec![FromOutside::verbatim("sam")];
    issue.comments = ["first", "second"]
        .iter()
        .enumerate()
        .map(|(at, said)| IssueComment {
            author: FromOutside::verbatim("sam"),
            at: format!("2026-10-0{}T10:00:00Z", at + 2),
            body: FromOutside::verbatim(*said),
        })
        .collect();
    issue.linked_pull_requests = vec![
        (40, LinkedPullRequestState::Merged),
        (41, LinkedPullRequestState::Closed),
        (42, LinkedPullRequestState::Open),
    ]
    .into_iter()
    .map(|(number, state)| LinkedPullRequest {
        number,
        title: FromOutside::verbatim(format!("Change {number}")),
        url: FromOutside::verbatim(format!("https://forge.invalid/o/r/pull/{number}")),
        state,
    })
    .collect();
    tracker.knows(issue);
    let fleet = a_fleet(&home, tracker);
    let read = fleet.get_issue(manifest(&fleet), "#31".to_string()).await.expect("found");
    assert_eq!(read.state, IssueState::Closed);
    assert_eq!(read.body, "## Why\n\nIt breaks.");
    assert_eq!((read.author.as_str(), read.opened_at.as_str()), ("nick", "2026-10-01T09:00:00Z"));
    assert_eq!((read.labels, read.assignees), (vec!["bug".to_string()], vec!["sam".to_string()]));
    let said: Vec<(&str, &str)> = read.comments.iter().map(|one| (one.at.as_str(), one.body.as_str())).collect();
    assert_eq!(said, [("2026-10-02T10:00:00Z", "first"), ("2026-10-03T10:00:00Z", "second")]);
    let linked: Vec<(u64, ipc::LinkedPullRequestState)> =
        read.linked_pull_requests.iter().map(|one| (one.number, one.state)).collect();
    assert_eq!(
        linked,
        [
            (40, ipc::LinkedPullRequestState::Merged),
            (41, ipc::LinkedPullRequestState::Closed),
            (42, ipc::LinkedPullRequestState::Open),
        ]
    );
    assert_eq!(read.linked_pull_requests[2].url, "https://forge.invalid/o/r/pull/42");
}

/// The tracker answers none for a pull request's number, as the real one does by its address.
#[tokio::test]
async fn a_number_the_tracker_does_not_have_is_a_404() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, Arc::new(FakeTracker::empty()));
    match fleet.get_issue(manifest(&fleet), "7".to_string()).await {
        Err(Refusal::NotFound(error)) => assert_eq!(error.code, "fleet.no_such_issue"),
        other => panic!("not found, not {other:?}"),
    }
}

#[tokio::test]
async fn a_tracker_that_will_not_answer_is_a_fault_and_an_unserved_repository_is_a_422() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::empty());
    tracker.will_not_list("not signed in");
    let fleet = a_fleet(&home, tracker);
    match fleet.get_issue(manifest(&fleet), "7".to_string()).await {
        Err(Refusal::Fault(error)) => {
            assert_eq!(error.code, "fleet.issues_unreadable");
            assert!(error.message.contains("not signed in"), "{}", error.message);
        }
        other => panic!("a fault, not {other:?}"),
    }
    match fleet.get_issue(ManifestId::carried("not-served"), "7".to_string()).await {
        Err(Refusal::Unacceptable(_)) => {}
        other => panic!("unacceptable, not {other:?}"),
    }
}
