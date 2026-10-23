//! Which Jobs hold a listed issue comes off the ledger's `issue` rows, so it is the same for every
//! tracker; what the tracker said of the issue rides beside it.

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::FromOutside;
use api::Issues;
use ipc::{IssueReference, IssueState, ManifestId};
use store::Holder;
use testkit::{FakeHarness, FakeTracker, FakeTrackers, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::session_issues::issue_held;
use crate::tests::daemon::fittings;
use crate::tests::tmp::TempDir;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const AT: &str = "2026-10-23T10:00:00.000Z";

fn manifest(fleet: &Hosted) -> ManifestId {
    let served = fleet.repositories().first().expect("a served repository");
    ManifestId::carried(served.manifest().id().as_str())
}

fn a_fleet(home: &TempDir, tracker: Arc<FakeTracker>) -> Hosted {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&[]));
    fittings.trackers = Arc::new(FakeTrackers::empty().tracker(tracker));
    Fleet::assembled(fittings)
}

async fn held_by(fleet: &Hosted, job: &str, issue: &str) {
    let mut store = fleet.store().lock().await;
    let issue = IssueReference::read(issue).expect("an issue");
    issue_held(&mut store, &Holder::session(job), manifest(fleet).as_str(), &issue, &BTreeMap::new(), AT)
        .expect("held");
}

#[tokio::test]
async fn a_listed_issue_names_the_jobs_holding_it_off_the_ledger() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::listing(vec![
        FakeTracker::issue(7, "Worked by two"),
        FakeTracker::issue(8, "Worked by none"),
    ]));
    let fleet = a_fleet(&home, tracker);
    held_by(&fleet, "job-b", "7").await;
    held_by(&fleet, "job-a", "#7").await;
    held_by(&fleet, "job-c", "9").await;

    let list = fleet.list_issues(manifest(&fleet), true).await.expect("listed");

    let holders: Vec<(&str, Vec<&str>)> = list
        .issues
        .iter()
        .map(|one| (one.reference.as_str(), one.holders.iter().map(String::as_str).collect()))
        .collect();
    assert_eq!(holders, [("#7", vec!["job-a", "job-b"]), ("#8", vec![])]);
}

#[tokio::test]
async fn a_job_that_gave_an_issue_back_no_longer_holds_it_and_a_read_issue_names_its_holders() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::listing(vec![FakeTracker::issue(7, "Worked")]));
    let fleet = a_fleet(&home, tracker);
    held_by(&fleet, "job-a", "7").await;
    held_by(&fleet, "job-b", "7").await;
    let read = fleet.get_issue(manifest(&fleet), "7".to_string()).await.expect("read");
    assert_eq!(read.holders, ["job-a", "job-b"]);

    fleet.store().lock().await.give_back(&Holder::session("job-a"), Some("issue"), AT).expect("given back");

    let list = fleet.list_issues(manifest(&fleet), true).await.expect("listed");
    assert_eq!(list.issues[0].holders, ["job-b"]);
}

#[tokio::test]
async fn a_listed_issue_carries_its_author_state_and_the_tracker_s_fields() {
    let home = TempDir::new();
    let mut open = FakeTracker::issue(7, "Open one");
    open.author = FromOutside::verbatim("sam");
    open.fields = vec![adapter_traits::IssueField {
        name: "Labels".to_string(),
        values: vec![FromOutside::verbatim("bug"), FromOutside::verbatim("web")],
    }];
    let mut closed = FakeTracker::issue(8, "Closed one");
    closed.closed = true;
    let fleet = a_fleet(&home, Arc::new(FakeTracker::listing(vec![open, closed])));

    let list = fleet.list_issues(manifest(&fleet), true).await.expect("listed");

    let (first, second) = (&list.issues[0], &list.issues[1]);
    assert_eq!((first.author.as_deref(), first.state), (Some("sam"), IssueState::Open));
    assert_eq!(first.fields.len(), 1);
    assert_eq!((first.fields[0].name.as_str(), first.fields[0].values.clone()), ("Labels", vec!["bug".to_string(), "web".to_string()]));
    assert_eq!((second.author.clone(), second.state, second.fields.len()), (None, IssueState::Closed, 0));
}
