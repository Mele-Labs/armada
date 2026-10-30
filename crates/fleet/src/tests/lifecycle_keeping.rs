//! A Job started now takes its tickets and pull requests on and has them read where Fleet already
//! reads them; a Job that was already running holds nothing. **No forge, tracker or process is
//! reached**: `FakeVcs` and `FakeTracker` say what the sources say, and a test counts what was asked.

use std::sync::Arc;

use adapter_traits::{
    FromOutside, IssueField, Mergeability, Omitted, PullRequestPageFacts, PullRequestStanding, ReadIssue,
    ReviewStanding, Reviewer, ReviewerKind,
};
use ipc::{ManifestId, SessionId, SessionRecord, SessionState, StartWatch, WatchStarts};
use store::{AttachmentState, Holder, KeptAttachment, KeptWatchTaken};
use testkit::{FakeTracker, FakeTrackers};

use super::pull_requesting::{facts, forge_ran, passed};
use super::session_host::{rig_fitted, rig_placing, Rig};

fn manifest(rig: &Rig) -> ManifestId {
    ManifestId::carried(rig.fleet.repositories().first().expect("served").manifest().id().as_str())
}

async fn holds(rig: &Rig, id: &SessionId, kind: &str, target: &str, state: AttachmentState, detail: &[(&str, &str)]) {
    let at = rig.fleet.now().as_str().to_string();
    rig.fleet
        .store()
        .lock()
        .await
        .attach(
            &KeptAttachment {
                holder: Holder::session(id.as_str()),
                kind: kind.into(),
                manifest_id: manifest(rig).as_str().to_string(),
                target: target.into(),
                state,
                detail: detail.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
                since: at.clone(),
                changed_at: at,
            },
            false,
        )
        .expect("attached");
}

async fn record(rig: &Rig, id: &SessionId) -> SessionRecord {
    use api::HostedSessions;
    Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("a session").session
}

/// `(lifecycle, key)` of everything the Job holds, sorted: the ledger lists its rows in no order a test should rely on.
async fn held(rig: &Rig, id: &SessionId) -> Vec<(String, String)> {
    let store = rig.fleet.store().lock().await;
    let kept = store.lifecycle_instances_of(id.as_str()).expect("reads");
    let mut held: Vec<(String, String)> = kept.into_iter().map(|one| (one.lifecycle, one.key)).collect();
    held.sort();
    held
}

/// `(key, from, to)` of every move a Job's instances of `lifecycle` made.
async fn moves(rig: &Rig, id: &SessionId, lifecycle: &str) -> Vec<(String, String, String)> {
    let store = rig.fleet.store().lock().await;
    let kept = store.lifecycle_moves_of(id.as_str()).expect("reads");
    kept.into_iter().filter(|one| one.lifecycle == lifecycle).map(|one| (one.key, one.from, one.to)).collect()
}

fn move_of(key: &str, from: &str, to: &str) -> (String, String, String) {
    (key.to_string(), from.to_string(), to.to_string())
}

#[tokio::test]
async fn a_job_started_now_takes_on_its_tickets_and_the_pull_request_for_them_and_a_running_one_holds_none() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    assert!(rig.fleet.store().lock().await.is_lifecycle_job(id.as_str()).expect("reads"), "marked as it started");
    holds(&rig, &id, "issue", "ARM-41", AttachmentState::Standing, &[("pull_requests", "812")]).await;
    holds(&rig, &id, "issue", "ARM-42", AttachmentState::Standing, &[]).await;
    holds(&rig, &id, "pr", "812", AttachmentState::Standing, &[("state", "open")]).await;

    let row = record(&rig, &id).await;
    rig.fleet.lifecycles_moved(row.clone()).await;
    rig.fleet.lifecycles_moved(row.clone()).await;

    let expected = |l: &str, k: &str| (l.to_string(), k.to_string());
    assert_eq!(held(&rig, &id).await, [expected("pr", "812"), expected("ticket", "ARM-41"), expected("ticket", "ARM-42")]);
    let kept = rig.fleet.store().lock().await.lifecycle_instances_of(id.as_str()).expect("reads");
    let pull = kept.iter().find(|one| one.lifecycle == "pr").expect("taken on");
    assert_eq!(pull.links, [("ticket".to_string(), "ARM-41".to_string())], "the pull request is for the ticket that names it");

    // The same row under a Job started before lifecycles existed: nothing is taken on.
    let mut before = row;
    before.id = SessionId::carried("01RUNNINGBEFORE");
    rig.fleet.lifecycles_moved(before).await;
    assert!(held(&rig, &SessionId::carried("01RUNNINGBEFORE")).await.is_empty());
}

fn page(review: ReviewStanding, reviewers: Vec<Reviewer>) -> PullRequestPageFacts {
    PullRequestPageFacts {
        base: "main".into(),
        author: Some("author".into()),
        created_at: None,
        additions: 0,
        deletions: 0,
        mergeable: Mergeability::Unknown,
        review,
        approvals: 0,
        conversation: Vec::new(),
        checks: Vec::new(),
        files: Vec::new(),
        commits: Vec::new(),
        omitted: Omitted::default(),
        head: None,
        body: None,
        threads: Vec::new(),
        reviewers,
    }
}

fn asked(login: &str) -> Reviewer {
    Reviewer { login: login.into(), kind: ReviewerKind::Person, verdict: None, name: None, url: None, team: None }
}

#[tokio::test]
async fn a_pull_request_is_read_where_the_ledger_reads_it_and_moves_only_when_what_the_forge_shows_changes() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    holds(&rig, &id, "pr", "12", AttachmentState::Standing, &[("state", "open")]).await;
    rig.fleet.lifecycles_moved(record(&rig, &id).await).await;
    forge_ran(&rig.fleet, passed());
    let (manifest_id, root) = (manifest(&rig), rig.root.clone());
    let reading = || async { rig.fleet.pull_ledger_kept_current(&manifest_id, &root).await };

    rig.fleet.vcs().now_pull_request(Some(facts(PullRequestStanding::Draft)));
    reading().await;
    reading().await;
    rig.fleet.vcs().now_pull_request(Some(facts(PullRequestStanding::Open)));
    rig.fleet.vcs().now_pull_request_page(None);
    reading().await;
    assert_eq!(moves(&rig, &id, "pr").await, [move_of("12", "unread", "in:draft")], "a forge that will not show the page says nothing");

    rig.fleet.vcs().now_pull_request_page(Some(page(ReviewStanding::Required, vec![asked("ana")])));
    reading().await;
    rig.fleet.vcs().now_pull_request_page(Some(page(ReviewStanding::Approved, vec![])));
    reading().await;
    reading().await;
    rig.fleet.vcs().now_pull_request(Some(facts(PullRequestStanding::Merged)));
    reading().await;

    assert_eq!(
        moves(&rig, &id, "pr").await,
        [
            move_of("12", "unread", "in:draft"),
            move_of("12", "in:draft", "in:reviewers_on"),
            move_of("12", "in:reviewers_on", "in:approved"),
            move_of("12", "in:approved", "in:merged"),
        ]
    );
}

fn status(word: &str) -> ReadIssue {
    let mut read = ReadIssue::bare("ARM-41".into(), FromOutside::verbatim("t"), FromOutside::verbatim("u"), false);
    read.fields.push(IssueField { name: "Status".into(), values: vec![FromOutside::verbatim(word)] });
    read
}

#[tokio::test]
async fn a_ticket_is_read_in_the_tracker_s_words_from_the_read_the_issue_ledger_already_makes() {
    let tracker = Arc::new(FakeTracker::empty());
    tracker.now_knows(status("In Progress"));
    let held_tracker = Arc::clone(&tracker);
    let rig = rig_fitted(move |fittings| fittings.trackers = Arc::new(FakeTrackers::empty().tracker(held_tracker)));
    let id = rig.start().await;
    holds(&rig, &id, "issue", "ARM-41", AttachmentState::Standing, &[]).await;
    rig.fleet.lifecycles_moved(record(&rig, &id).await).await;
    let (manifest_id, root) = (manifest(&rig), rig.root.clone());

    rig.fleet.issue_ledger_kept_current(&manifest_id, &root).await;
    rig.fleet.issue_ledger_kept_current(&manifest_id, &root).await;
    tracker.now_knows(status("Code Review"));
    rig.fleet.issue_ledger_kept_current(&manifest_id, &root).await;

    assert_eq!(
        moves(&rig, &id, "ticket").await,
        [
            move_of("ARM-41", "unread", "unmapped:In Progress"),
            move_of("ARM-41", "unmapped:In Progress", "unmapped:Code Review"),
        ],
        "the lifecycle Armada carries lists no words, so each is kept as the tracker said it"
    );
}

#[tokio::test]
async fn a_job_another_job_started_is_an_instance_linked_to_the_starter_and_landing_ends_it() {
    let rig = rig_placing(None);
    let parent = rig.start().await;
    let child = rig.start().await;
    holds(&rig, &child, "started_by", parent.as_str(), AttachmentState::Spent, &[]).await;

    let mut row = record(&rig, &child).await;
    rig.fleet.lifecycles_moved(row.clone()).await;
    let kept = rig.fleet.store().lock().await.lifecycle_instances_of(parent.as_str()).expect("reads");
    assert_eq!(kept.len(), 1);
    assert_eq!((kept[0].lifecycle.as_str(), kept[0].key.as_str()), ("job", child.as_str()));
    assert_eq!(kept[0].links, [("job".to_string(), parent.as_str().to_string())]);

    row.state = SessionState::Ended;
    rig.fleet.lifecycles_moved(row.clone()).await;
    holds(&rig, &child, "pr", "9", AttachmentState::Spent, &[("state", "merged")]).await;
    let mut landed = record(&rig, &child).await;
    landed.state = SessionState::Ended;
    rig.fleet.lifecycles_moved(landed).await;

    assert_eq!(
        moves(&rig, &parent, "job").await,
        [
            move_of(child.as_str(), "unread", "in:running"),
            move_of(child.as_str(), "in:running", "in:ended"),
            move_of(child.as_str(), "in:ended", "in:landed"),
        ]
    );
}

#[tokio::test]
async fn what_a_watch_follows_is_handed_each_reading_it_keeps_and_a_repeat_moves_nothing() {
    let rig = rig_placing(None);
    let id = rig.start().await;
    let watch = StartWatch {
        session_id: Some(id.clone()),
        name: "QA deploy".to_string(),
        starts: WatchStarts::Command { command: vec!["/bin/cat".into(), "absent.json".into()], every: "30s".into(), gives_up_after: None },
        follows: None,
        set_aside: Vec::new(),
    };
    let watch_id = {
        use api::Watches;
        rig.fleet.start_watch(None, watch).await.expect("started").id
    };
    {
        let mut store = rig.fleet.store().lock().await;
        store.take_on_lifecycle_instance(id.as_str(), "deploy", "web", &[], "t0").expect("taken on");
        store.follow_lifecycle_instance_with_watch(id.as_str(), "deploy", "web", &watch_id).expect("followed");
    }
    let reading = |state: &str| KeptWatchTaken::Read { state: state.into(), finished: false, says: String::new(), link: None, parts: Vec::new() };

    for state in ["waiting", "running", "running", "passed"] {
        rig.fleet.watch_read(&watch_id, reading(state)).await;
    }

    assert_eq!(
        moves(&rig, &id, "deploy").await,
        [
            move_of("web", "unread", "unmapped:waiting"),
            move_of("web", "unmapped:waiting", "in:rolling_out"),
            move_of("web", "in:rolling_out", "in:healthy"),
        ],
        "a watch that has not started reads waiting, which no state of Deploy claims"
    );
}
