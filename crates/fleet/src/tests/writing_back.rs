//! What Fleet writes on the issue a Job came from, and the list of open issues
//! Bridge chooses from. #91, `docs/concepts/tracker.md`.

use std::sync::Arc;

use adapter_traits::Review;
use api::{Issues, Refusal};
use core_model::{IssueSource, JobId, JobStatus, Timestamp, Ulid};
use ipc::ManifestId;
use store::{WriteBackOwed, WrittenBack};
use testkit::{
    FakeHarness, FakeJudge, FakeLinkLookup, FakeTracker, FakeTrackers, FakeVcs, FakeWorkProduct,
    TrackerWrite,
};

use crate::daemon::Fleet;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::fittings;
use crate::tests::proposing::a_catalogue;
use crate::tests::tmp::TempDir;
use crate::writing_back::{owed, Label, IN_PROGRESS, SET_ASIDE};

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const ISSUE: &str = "https://example.test/x/y/issues/9";

/// A Fleet whose proposer reads issue 9, writing back through `tracker`.
fn a_fleet(home: &TempDir, tracker: Arc<FakeTracker>) -> Fixture {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/read.rs"]));
    fittings.starting().workflows = a_catalogue()
        .into_iter()
        .map(|workflow| (workflow.id().clone(), workflow))
        .collect();
    fittings.judge = Arc::new(FakeJudge::saying(
        "workflow: bug\ntitle: The reader drops the last row\ndone_when: the last row is read",
    ));
    fittings.trackers = Arc::new(
        FakeTrackers::empty()
            .links(Arc::new(
                FakeLinkLookup::resolving("example.test/issues/9", "the reader drops the last row")
                    .linking("y#9", ISSUE),
            ))
            .tracker(tracker),
    );
    Fleet::assembled(fittings)
}

async fn proposed_from_the_issue(fleet: &Fixture) -> JobId {
    let made = fleet
        .propose_from("https://example.test/issues/9", None)
        .await
        .expect("a proposal");
    made[0].id().clone()
}

async fn kept(fleet: &Fixture, job: &JobId) -> WrittenBack {
    fleet
        .store()
        .lock()
        .await
        .issue_written_back(job)
        .expect("reads")
        .expect("a row")
}

fn commented(text: &str) -> TrackerWrite {
    TrackerWrite::Commented {
        url: ISSUE.to_string(),
        text: text.to_string(),
    }
}

fn relabelled(add: &[&str], remove: &[&str]) -> TrackerWrite {
    TrackerWrite::Relabelled {
        url: ISSUE.to_string(),
        add: add.iter().map(|one| one.to_string()).collect(),
        remove: remove.iter().map(|one| one.to_string()).collect(),
    }
}

/// A proposal is a view and nothing more; a started Job says so once and
/// takes the label; its pull request is named once; killing it takes the
/// label off and leaves nothing more to write.
#[tokio::test]
async fn a_job_from_an_issue_says_it_started_names_its_pull_request_and_takes_its_label_off_at_the_end() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::empty());
    let fleet = a_fleet(&home, tracker.clone());
    let job = proposed_from_the_issue(&fleet).await;

    assert_eq!(fleet.write_back().await.expect("a pass"), 0);
    assert!(tracker.writes().is_empty(), "awaiting approval writes nothing");

    let started = dispatched(&fleet, &job).await.expect("dispatched");
    assert!(!started.status().is_terminal(), "{:?}", started.status());
    assert_eq!(fleet.write_back().await.expect("a pass"), 1);
    let number = started.number().get();
    let first = tracker.writes();
    assert_eq!(
        first[0],
        commented(&format!(
            "Armada started job {number} on this issue: The reader drops the last row"
        ))
    );
    assert_eq!(first.last(), Some(&relabelled(&[IN_PROGRESS], &[])));
    // The fixture's fake delivery may already have opened one by now.
    if first.len() == 3 {
        assert!(matches!(&first[1], TrackerWrite::Commented { text, .. }
            if text.starts_with(&format!("Job {number} opened a pull request for this issue: "))));
    }
    assert_eq!(fleet.write_back().await.expect("a pass"), 0);
    assert_eq!(tracker.writes().len(), first.len(), "nothing is said twice");

    let pull = "https://example.test/x/y/pull/12";
    fleet
        .store()
        .lock()
        .await
        .record_delivery(
            &job,
            &store::Delivery {
                pull_request: Some(pull.to_string()),
                ..Default::default()
            },
        )
        .expect("recorded");
    fleet.write_back().await.expect("a pass");
    assert_eq!(
        tracker.writes().last(),
        Some(&commented(&format!(
            "Job {number} opened a pull request for this issue: {pull}"
        )))
    );
    let said = tracker.writes().len();

    fleet.kill_job(&job).await.expect("killed");
    fleet.write_back().await.expect("a pass");
    assert_eq!(tracker.writes()[said..], [relabelled(&[], &[IN_PROGRESS])]);
    assert!(kept(&fleet, &job).await.settled);
    fleet.write_back().await.expect("a pass");
    assert_eq!(tracker.writes().len(), said + 1, "a settled issue is not read again");
}

/// A tracker that refuses leaves the Job where it was, keeps what it said on
/// the record, and is not asked again within the retry interval.
#[tokio::test]
async fn a_refused_write_is_kept_and_never_moves_the_job() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::empty());
    let fleet = a_fleet(&home, tracker.clone());
    let job = proposed_from_the_issue(&fleet).await;
    let started = dispatched(&fleet, &job).await.expect("dispatched");
    tracker.refuses_writes("HTTP 403: Resource not accessible by integration");

    assert_eq!(fleet.write_back().await.expect("a pass"), 0);
    let row = kept(&fleet, &job).await;
    assert!(!row.started, "nothing was said, so nothing is marked said");
    assert!(row.failed.expect("the refusal is kept").contains("HTTP 403"));
    assert_eq!(fleet.load(&job).await.expect("loads").status(), started.status());

    tracker.answers_again();
    fleet.write_back().await.expect("a pass");
    assert!(tracker.writes().is_empty(), "not asked again inside ten minutes");
}

/// The pull request body carries the tracker's closing line for the Job's
/// issue, and a Job from no issue gets its body as composed.
#[tokio::test]
async fn the_pull_request_closes_the_issue_the_job_came_from() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, Arc::new(FakeTracker::empty()));
    let job = proposed_from_the_issue(&fleet).await;
    let loaded = fleet.load(&job).await.expect("loads");
    let review = Review::assembled("A title", "## Why\n\nBecause.\n");
    let closed = fleet.closing_its_issue(&loaded, review.clone()).await;
    assert!(closed.body().ends_with("\n\nCloses y#9\n"), "{}", closed.body());
    assert_eq!(closed.title(), "A title");

    let made = fleet.propose_from("the reader drops the last row", None).await.expect("a proposal");
    let none = fleet.load(made[0].id()).await.expect("loads");
    assert_eq!(fleet.closing_its_issue(&none, review.clone()).await, review);
}

/// A tracker that reads the title and not the body is told by the title: the reference leads it,
/// the body is as composed, and a title that already carries the reference is not marked twice.
#[tokio::test]
async fn a_tracker_that_links_by_title_has_its_mark_put_there_and_nothing_in_the_body() {
    let home = TempDir::new();
    let fleet = a_fleet(&home, Arc::new(FakeTracker::linking_by_title()));
    let job = proposed_from_the_issue(&fleet).await;
    let loaded = fleet.load(&job).await.expect("loads");
    let review = Review::assembled("A title", "## Why\n\nBecause.\n");
    let marked = fleet.closing_its_issue(&loaded, review.clone()).await;
    assert_eq!(marked.title(), "y#9 A title");
    assert_eq!(marked.body(), review.body());
    let again = fleet
        .closing_its_issue(&loaded, Review::assembled("y#9 Already marked", "Body.\n"))
        .await;
    assert_eq!(again.title(), "y#9 Already marked");
}

fn row(status: JobStatus, pull_request: Option<&str>, kept: WrittenBack) -> WriteBackOwed {
    WriteBackOwed {
        job_id: JobId::carried(Ulid::carried("01J0000000000000000000000A")),
        number: 4,
        title: "Bound the reader".to_string(),
        status: status.as_wire().to_string(),
        source: IssueSource::read(
            "y#9".to_string(),
            ISSUE.to_string(),
            Timestamp::from_rfc3339("2026-10-11T12:00:00Z"),
        ),
        pull_request: pull_request.map(str::to_string),
        kept,
    }
}

fn labelled(label: &str) -> WrittenBack {
    WrittenBack {
        started: true,
        label: Some(label.to_string()),
        ..WrittenBack::default()
    }
}

#[test]
fn nothing_is_owed_before_the_job_starts() {
    for status in [
        JobStatus::Proposing,
        JobStatus::AwaitingApproval,
        JobStatus::Queued,
    ] {
        let owed = owed(&row(status, None, WrittenBack::default()));
        assert!(owed.comments.is_empty() && owed.add.is_none(), "{status:?}");
        assert!(!owed.next.settled);
    }
}

#[test]
fn a_job_waiting_on_a_person_swaps_its_label_and_back() {
    for status in [JobStatus::Escalated, JobStatus::AwaitingRepair] {
        let owed = owed(&row(status, None, labelled("in_progress")));
        assert_eq!((owed.add, owed.remove), (Some(Label::SetAside), Some(Label::InProgress)));
        assert!(owed.comments.is_empty(), "the start was said already");
    }
    let back = owed(&row(JobStatus::Running, None, labelled("set_aside")));
    assert_eq!((back.add, back.remove), (Some(Label::InProgress), Some(Label::SetAside)));
    assert_eq!(SET_ASIDE, "armada: set aside");
}

#[test]
fn an_ended_job_takes_its_label_off_whether_it_landed_or_not() {
    for status in [
        JobStatus::CompletedSuccess,
        JobStatus::CompletedFailed,
        JobStatus::Killed,
        JobStatus::Rejected,
        JobStatus::Superseded,
    ] {
        let owed = owed(&row(status, None, labelled("in_progress")));
        assert_eq!((owed.add, owed.remove), (None, Some(Label::InProgress)), "{status:?}");
        assert!(owed.next.settled && owed.next.label.is_none());
    }
    let never = owed(&row(JobStatus::Rejected, None, WrittenBack::default()));
    assert!(never.comments.is_empty() && never.remove.is_none() && never.next.settled);
}

#[test]
fn a_pull_request_opened_after_the_start_is_named_once_and_again_if_it_changes() {
    let at = "https://example.test/x/y/pull/12";
    let first = owed(&row(JobStatus::AwaitingReview, Some(at), labelled("in_progress")));
    assert_eq!(first.comments.len(), 1);
    let said = WrittenBack {
        pull_request: Some(at.to_string()),
        ..labelled("in_progress")
    };
    assert!(owed(&row(JobStatus::AwaitingReview, Some(at), said.clone())).comments.is_empty());
    let moved = owed(&row(JobStatus::Running, Some("https://example.test/x/y/pull/13"), said));
    assert_eq!(moved.comments.len(), 1);
}

/// The list is read once and held; refresh reads again; a failed read with a
/// list held answers the old one and says why; with none held it refuses.
#[tokio::test]
async fn the_issue_list_is_held_refreshed_on_ask_and_stale_when_the_tracker_will_not_answer() {
    let home = TempDir::new();
    let tracker = Arc::new(FakeTracker::listing(vec![FakeTracker::issue(9, "The reader drops the last row")]));
    let fleet = a_fleet(&home, tracker.clone());
    let repository = ManifestId::carried(
        fleet.served_named(None).expect("served").manifest().id().as_str(),
    );

    tracker.will_not_list("gh: not logged in");
    let refused = fleet.list_issues(repository.clone(), false).await.expect_err("nothing held");
    assert!(matches!(&refused, Refusal::Fault(e) if e.code == "fleet.issues_unreadable"));
    tracker.answers_again();

    let first = fleet.list_issues(repository.clone(), false).await.expect("lists");
    assert_eq!(first.issues[0].title, "The reader drops the last row");
    assert_eq!(first.stale, None);
    fleet.list_issues(repository.clone(), false).await.expect("lists");
    assert_eq!(tracker.listings(), 2, "the second ask is answered from what is held");

    tracker.now_lists(Vec::new());
    let fresh = fleet.list_issues(repository.clone(), true).await.expect("lists");
    assert!(fresh.issues.is_empty());

    tracker.will_not_list("gh: rate limited");
    let stale = fleet.list_issues(repository.clone(), true).await.expect("the old list");
    assert!(stale.stale.expect("says why").contains("rate limited"));
}

/// An issue a Job is still working names that Job, which is the link from the
/// issue's side the 31 Aug comment on #91 left as a lookup.
#[tokio::test]
async fn an_issue_a_job_came_from_names_the_job() {
    let home = TempDir::new();
    let mut nine = FakeTracker::issue(9, "The reader drops the last row");
    nine.url = adapter_traits::FromOutside::verbatim(ISSUE);
    let tracker = Arc::new(FakeTracker::listing(vec![nine, FakeTracker::issue(10, "Another")]));
    let fleet = a_fleet(&home, tracker.clone());
    let job = proposed_from_the_issue(&fleet).await;
    let repository = ManifestId::carried(
        fleet.served_named(None).expect("served").manifest().id().as_str(),
    );
    let list = fleet.list_issues(repository, false).await.expect("lists");
    assert_eq!(list.issues[0].job_id, Some(ipc::JobId::from(&job)));
    assert_eq!(list.issues[1].job_id, None);
}
