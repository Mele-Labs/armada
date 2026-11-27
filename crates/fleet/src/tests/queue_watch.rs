//! A merge queue that holds runners for nothing or cannot get one, against a scripted forge: runs
//! no entry stands behind are stopped, a drop for want of a runner is put back quietly up to
//! `limits.queueRequeuesPerCommit` and then called stuck, and a failure in a pull request's own
//! code is told to its Job as ever. The forge's sentences are `adapters`' to assert.

use std::sync::Arc;

use adapter_traits::{
    CiState, FromOutside, InfraCause, InfraFailure, Oid, OpenPull, PullQueue, PullRequestStanding, QueueEntry, QueueRun,
    QueueState, RecentlyMerged,
};
use core_model::Timestamp;
use ipc::QueueStuckCause;

use super::pull_notices::{a_rig, check, pull, Rig};
use super::pull_requesting::{facts, forge_ran, passed};
use crate::queue_watch::{infrastructure_of, orphans, Moved, QueueWatch, Seen};

const LONG_AGO: &str = "2020-01-01T00:00:00Z";
const FAR_AHEAD: &str = "2099-01-01T00:00:00Z";

fn sha(digit: char) -> String {
    digit.to_string().repeat(40)
}

fn entry(number: u64, head: char, enqueued: &str, estimate: Option<u64>) -> QueueEntry {
    QueueEntry {
        number,
        position: 1,
        state: QueueState::AwaitingChecks,
        head: Oid::parse(&sha(head)),
        enqueued_at: Some(FromOutside::verbatim(enqueued)),
        estimated_seconds: estimate,
    }
}

fn run(id: u64, number: u64, head: char) -> QueueRun {
    QueueRun {
        id,
        branch: FromOutside::verbatim(format!("gh-readonly-queue/main/pr-{number}-abc")),
        head: Oid::parse(&sha(head)),
        number: Some(number),
    }
}

fn failed(job: &str, cause: InfraCause) -> InfraFailure {
    InfraFailure { job: FromOutside::verbatim(job), log_url: Some(FromOutside::verbatim(format!("https://forge.invalid/job/{job}"))), cause }
}

fn at(text: &str) -> Timestamp {
    Timestamp::from_rfc3339(text)
}

fn ms(text: &str) -> i64 {
    at(text).epoch_millis().expect("an instant")
}

#[test]
fn a_run_no_entry_stands_behind_is_an_orphan_and_one_an_entry_holds_is_not() {
    let runs = [run(1, 10, 'a'), run(2, 11, 'b'), run(3, 12, 'c')];
    let queue = [entry(10, 'a', LONG_AGO, None), entry(11, 'f', LONG_AGO, None)];

    assert_eq!(orphans(&runs, &queue), [2, 3], "#11's entry stands at another commit, and #12 has none");
}

#[test]
fn a_run_whose_commit_or_entry_head_is_unknown_is_kept() {
    let mut headless = run(1, 10, 'a');
    headless.head = None;
    let mut unnamed = entry(11, 'b', LONG_AGO, None);
    unnamed.head = None;
    let mut nobody = run(3, 99, 'c');
    nobody.number = None;

    assert!(orphans(&[headless, run(2, 11, 'b'), nobody], &[entry(10, 'a', LONG_AGO, None), unnamed]).is_empty());
}

#[test]
fn a_queue_as_long_as_one_read_holds_decides_nothing() {
    let queue: Vec<QueueEntry> = (1..=adapter_traits::MOST_QUEUE_ENTRIES as u64).map(|n| entry(n, 'a', LONG_AGO, None)).collect();
    assert!(orphans(&[run(1, 999, 'a')], &queue).is_empty(), "the entry may be past the cut");
}

#[test]
fn a_drop_is_infrastructure_only_where_every_failed_job_says_so() {
    let found = infrastructure_of(&[failed("native", InfraCause::NoRunner), failed("lint", InfraCause::TimedOut)]).expect("infrastructure");
    assert_eq!((found.cause, found.job.as_str()), (QueueStuckCause::NoRunner, "native"));
    assert_eq!(infrastructure_of(&[failed("unit", InfraCause::TimedOut)]).map(|it| it.cause), Some(QueueStuckCause::TimedOut));
    assert_eq!(infrastructure_of(&[failed("native", InfraCause::NoRunner), failed("unit", InfraCause::Other)]), None, "one failed in its own code");
    assert_eq!(infrastructure_of(&[]), None, "no failed job is a drop by hand");
}

fn evaluated(watch: &mut QueueWatch, queue: &[QueueEntry], merged_at: &[i64], now: &Timestamp) -> Moved {
    watch.evaluated(&Seen { queue, merged_at, open: &[] }, now)
}

#[test]
fn an_entry_that_waited_past_twice_its_estimate_and_an_hour_is_stuck_and_a_merge_after_it_clears_it() {
    let now = at("2026-11-25T12:00:00Z");
    let mut watch = QueueWatch::default();

    let young = [entry(1, 'a', "2026-11-25T11:30:00Z", Some(60))];
    evaluated(&mut watch, &young, &[], &now);
    assert!(watch.stuck.is_none(), "half an hour is under the hour a wait is always allowed");

    let long = [entry(1, 'a', "2026-11-25T10:50:00Z", Some(60))];
    evaluated(&mut watch, &long, &[], &now);
    let stuck = watch.stuck.clone().expect("an hour and ten minutes against an estimate of one");
    assert_eq!((stuck.cause, stuck.pulls.as_slice()), (QueueStuckCause::PastEstimate, [1].as_slice()));

    let before = [ms("2026-11-25T11:00:00Z")];
    assert_eq!(evaluated(&mut watch, &long, &before, &now), Moved::Nothing, "a merge before it began does not clear it");
    let after = [ms("2026-11-25T12:01:00Z")];
    assert_eq!(evaluated(&mut watch, &long, &after, &at("2026-11-25T12:02:00Z")), Moved::Lifted);
    assert!(watch.stuck.is_none());

    evaluated(&mut watch, &long, &[], &at("2026-11-25T12:03:00Z"));
    assert!(watch.stuck.is_none(), "an entry that waited before the queue moved is not stuck again at once");
}

#[test]
fn a_long_estimate_allows_a_long_wait() {
    let now = at("2026-11-25T12:00:00Z");
    let mut watch = QueueWatch::default();
    let queue = [entry(1, 'a', "2026-11-25T09:00:00Z", Some(2 * 3600))];
    watch.evaluated(&Seen { queue: &queue, merged_at: &[], open: &[] }, &now);
    assert!(watch.stuck.is_none(), "three hours is under twice the two the forge estimated");
}

#[test]
fn a_queue_that_empties_clears_a_stuck_one_but_a_dropped_pull_request_still_open_keeps_it() {
    let now = at("2026-11-25T12:00:00Z");
    let found = infrastructure_of(&[failed("native", InfraCause::NoRunner)]).expect("infrastructure");
    let mut watch = QueueWatch::default();
    watch.declared(&found, 13, &now);

    assert_eq!(watch.evaluated(&Seen { queue: &[], merged_at: &[], open: &[13] }, &now), Moved::Nothing);
    assert_eq!(watch.stuck.as_ref().map(|it| it.pulls.clone()), Some(vec![13]), "it waits outside the queue");
    assert_eq!(watch.evaluated(&Seen { queue: &[], merged_at: &[], open: &[] }, &now), Moved::Lifted, "closed, so nothing waits");
}

fn open(number: u64) -> OpenPull {
    OpenPull {
        number,
        title: FromOutside::verbatim("Fix the reader"),
        branch: FromOutside::verbatim(format!("armada/{number}")),
        url: FromOutside::verbatim(format!("https://forge.invalid/armada/pull/{number}")),
        author: None,
        ci: None,
        head: None,
        failing: Vec::new(),
        auto_merge: false,
        facts: Default::default(),
    }
}

async fn visited(rig: &Rig) {
    let served = rig.fleet.repositories().served().remove(0);
    rig.fleet.notice_pulls(&served).await;
    assert!(rig.fleet.notice_queue(&served).await, "the queue answered");
}

async fn stuck_on_the_line(rig: &Rig) -> Option<ipc::QueueStuck> {
    rig.fleet.merge_hubs().await.into_iter().next().and_then(|(_, hub)| hub.queue_stuck)
}

#[tokio::test]
async fn a_queue_run_no_entry_stands_behind_is_stopped_and_one_an_entry_holds_is_not() {
    let rig = a_rig();
    let forge = &rig.fleet.vcs().main_ci;
    forge.queue_runs_are(Some(vec![run(71, 9, 'a'), run(72, 10, 'b')]));
    forge.queue_is(Some(vec![entry(10, 'b', FAR_AHEAD, None)]));

    visited(&rig).await;

    assert_eq!(forge.runs_cancelled(), [71]);
}

#[tokio::test]
async fn a_queue_that_would_not_answer_stops_no_run() {
    let rig = a_rig();
    let forge = &rig.fleet.vcs().main_ci;
    forge.queue_runs_are(Some(vec![run(71, 9, 'a')]));
    forge.queue_is(None);
    let served = rig.fleet.repositories().served().remove(0);

    assert!(!rig.fleet.notice_queue(&served).await, "the read failed");

    assert!(forge.runs_cancelled().is_empty(), "a failed read is no empty queue");
}

#[tokio::test]
async fn runs_that_would_not_be_listed_stop_nothing() {
    let rig = a_rig();
    let forge = &rig.fleet.vcs().main_ci;
    forge.queue_runs_are(None);
    forge.queue_is(Some(Vec::new()));

    visited(&rig).await;

    assert!(forge.runs_cancelled().is_empty());
}

/// A rig whose Job holds #13, that the queue drops and the next reading finds out of it.
struct Dropping {
    rig: Rig,
    session: String,
}

impl Dropping {
    async fn with(failures: Vec<InfraFailure>) -> Dropping {
        let rig = a_rig();
        let session = rig.a_hosted_session_on(13).await;
        let tested = sha('2');
        let forge = &rig.fleet.vcs().main_ci;
        forge.queue_commit_is(13, &tested);
        forge.infra_failures_are(&tested, failures);
        forge.pulls_are(Some(vec![open(13)]));
        forge.queue_is(Some(Vec::new()));
        rig.fleet.vcs().now_pull_request(Some(facts(PullRequestStanding::Open)));
        forge_ran(&rig.fleet, passed());
        Dropping { rig, session }
    }

    /// Queued, then out of the queue at the same commit.
    async fn drops(&self) {
        // The forge took auto-merge off with the drop, which the fake does not.
        self.rig.fleet.vcs().now_pull_request(Some(facts(PullRequestStanding::Open)));
        let forge = &self.rig.fleet.vcs().main_ci;
        let mut queued = pull(13, vec![check("ci", CiState::Passed, true)]);
        queued.queue = PullQueue::Waiting;
        forge.followed_are(Some(vec![queued]));
        self.rig.reads().await;
        forge.followed_are(Some(vec![pull(13, vec![check("ci", CiState::Passed, true)])]));
        self.rig.reads().await;
    }

    fn queued_again(&self) -> usize {
        self.rig.fleet.vcs().times_asked_for_auto_merge()
    }

    async fn the_line(&self) -> Vec<String> {
        self.rig.fleet.store().lock().await.calls_of(&self.session).expect("reads").into_iter().map(|one| one.text).collect()
    }
}

#[tokio::test]
async fn a_drop_for_want_of_a_runner_is_queued_again_without_a_word_up_to_the_setting_and_then_stuck() {
    let dropping = Dropping::with(vec![failed("native", InfraCause::NoRunner)]).await;

    dropping.drops().await;
    dropping.drops().await;
    assert_eq!(dropping.queued_again(), 2, "limits.queueRequeuesPerCommit ships at two");
    assert!(dropping.rig.runs_sent_to_the_session().is_empty(), "told to nobody");
    assert!(stuck_on_the_line(&dropping.rig).await.is_none(), "not yet stuck");
    assert!(dropping.the_line().await.is_empty());

    dropping.drops().await;
    assert_eq!(dropping.queued_again(), 2, "not a third time");
    assert!(dropping.rig.runs_sent_to_the_session().is_empty(), "stuck is the merge line's, not a message");
    visited(&dropping.rig).await;
    let stuck = stuck_on_the_line(&dropping.rig).await.expect("stuck");
    assert_eq!((stuck.cause, stuck.job.as_deref(), stuck.pulls.as_slice()), (QueueStuckCause::NoRunner, Some("native"), [13].as_slice()));
    assert_eq!(stuck.log_url.as_deref(), Some("https://forge.invalid/job/native"));
    assert_eq!(dropping.the_line().await, ["The merge queue is stuck: native got no runner."], "one line, on the Job that holds #13");
}

#[tokio::test]
async fn a_person_queueing_a_stuck_pull_request_again_lets_fleet_put_it_back_after_a_later_drop() {
    use api::PullRequests;
    let dropping = Dropping::with(vec![failed("native", InfraCause::NoRunner)]).await;
    for _ in 0..3 {
        dropping.drops().await;
    }
    assert_eq!(dropping.queued_again(), 2, "the setting is spent");

    Arc::clone(&dropping.rig.fleet).enable_auto_merge(ipc::ManifestId::carried(dropping.rig.manifest()), 13).await.expect("queued");
    assert_eq!(dropping.queued_again(), 3, "the person's own ask");
    dropping.drops().await;

    assert_eq!(dropping.queued_again(), 4, "Fleet put it back again, its count forgotten");
}

#[tokio::test]
async fn a_stuck_queue_that_a_merge_clears_takes_the_line_off_the_job() {
    let dropping = Dropping::with(vec![failed("native", InfraCause::TimedOut)]).await;
    for _ in 0..3 {
        dropping.drops().await;
    }
    let served = dropping.rig.fleet.repositories().served().remove(0);
    visited(&dropping.rig).await;
    dropping.rig.fleet.queue_watched(&served).await;
    assert_eq!(dropping.the_line().await.len(), 1);

    let forge = &dropping.rig.fleet.vcs().main_ci;
    forge.recently_merged_are(Some(vec![RecentlyMerged {
        number: 14,
        title: FromOutside::verbatim("Another"),
        branch: FromOutside::verbatim("armada/14"),
        url: FromOutside::verbatim("https://forge.invalid/armada/pull/14"),
        author: None,
        merged_at: FromOutside::verbatim(FAR_AHEAD),
        commit: None,
    }]));
    dropping.rig.fleet.notice_merged(&served).await;
    visited(&dropping.rig).await;
    dropping.rig.fleet.queue_watched(&served).await;

    assert!(stuck_on_the_line(&dropping.rig).await.is_none());
    assert!(dropping.the_line().await.is_empty(), "taken off the Job's list");
}

#[tokio::test]
async fn a_drop_that_failed_in_the_pull_requests_own_code_is_told_as_ever() {
    let dropping = Dropping::with(vec![failed("native", InfraCause::NoRunner), failed("unit", InfraCause::Other)]).await;

    dropping.drops().await;

    assert_eq!(dropping.queued_again(), 0, "not put back");
    let sent = dropping.rig.runs_sent_to_the_session();
    assert!(sent.iter().any(|line| line.contains("#13 left the merge queue")), "{sent:?}");
    assert!(stuck_on_the_line(&dropping.rig).await.is_none());
}

#[tokio::test]
async fn an_entry_past_its_estimate_is_stuck_on_the_merge_line_with_no_job_named() {
    let rig = a_rig();
    let session = rig.a_hosted_session_on(13).await;
    let forge = &rig.fleet.vcs().main_ci;
    forge.pulls_are(Some(vec![open(13)]));
    forge.queue_is(Some(vec![entry(13, 'a', LONG_AGO, Some(600))]));
    let served = rig.fleet.repositories().served().remove(0);

    visited(&rig).await;
    rig.fleet.queue_watched(&served).await;

    let stuck = stuck_on_the_line(&rig).await.expect("stuck");
    assert_eq!((stuck.cause, stuck.job, stuck.pulls), (QueueStuckCause::PastEstimate, None, vec![13]));
    let line = rig.fleet.store().lock().await.calls_of(&session).expect("reads");
    assert_eq!(line.len(), 1);
    assert!(line[0].text.contains("#13 waited past the forge's estimate"), "{}", line[0].text);
}
