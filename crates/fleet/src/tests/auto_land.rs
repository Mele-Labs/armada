//! Auto land on a hosted Session: the forge's auto-merge follows the flag. **No forge is called**:
//! `FakeVcs` stands in for `gh`, and a test says what was asked of it by counting.

use adapter_traits::{CiState, FromOutside, PullQueue, PullRequestStanding, WatchedCheck, WatchedPull};
use api::{HostedSessions, Refusal, Sessions};
use ipc::{AttachmentReport, SessionFact, SessionId, SessionRecord, SessionReport, SetSessionAutoLand};
use store::{AttachmentState, Holder, KeptAttachment};

use super::pull_requesting::{facts, forge_ran, passed};
use super::session_host::{rig_placing, Rig};

const NUMBER: u64 = 12;

fn flag(id: &SessionId, on: bool) -> SetSessionAutoLand {
    SetSessionAutoLand { id: id.clone(), on }
}

fn manifest(rig: &Rig) -> String {
    rig.fleet.repositories().first().expect("served").manifest().id().as_str().to_string()
}

async fn holds(rig: &Rig, id: &SessionId, kind: &str, target: &str) {
    let at = rig.fleet.now().as_str().to_string();
    rig.fleet
        .store()
        .lock()
        .await
        .attach(
            &KeptAttachment {
                holder: Holder::session(id.as_str()),
                kind: kind.into(),
                manifest_id: manifest(rig),
                target: target.into(),
                state: AttachmentState::Standing,
                detail: Default::default(),
                since: at.clone(),
                changed_at: at,
            },
            false,
        )
        .expect("attached");
}

/// A session holding pull request 12, which the forge shows as `standing` with auto-merge `on`.
async fn holding_pull_request(standing: PullRequestStanding, on: bool) -> (Rig, SessionId) {
    let rig = rig_placing(None);
    let id = rig.start().await;
    holds(&rig, &id, "pr", &NUMBER.to_string()).await;
    let mut shown = facts(standing);
    shown.auto_merge = on;
    rig.fleet.vcs().now_pull_request(Some(shown));
    forge_ran(&rig.fleet, passed());
    (rig, id)
}

async fn row_of(rig: &Rig, id: &SessionId) -> SessionRecord {
    std::sync::Arc::clone(&rig.fleet).get_session(id.clone()).await.expect("a session").session
}

fn code(refusal: &Refusal) -> &str {
    &refusal.error().code
}

fn watched(number: u64) -> WatchedPull {
    WatchedPull {
        number,
        branch: FromOutside::verbatim(format!("armada/{number}")),
        url: FromOutside::verbatim(format!("https://forge.invalid/armada/pull/{number}")),
        author: None,
        head: Some("a".repeat(40)),
        conflicting: false,
        queue: PullQueue::Outside,
        checks: Vec::new(),
        title: None,
        auto_merge: false,
    }
}

/// The session worked branch `armada/12`, and the forge shows an open pull request from it that
/// nobody has reported: the pull watch attaches it on its next read.
async fn working_a_branch_with_an_open_pull_request() -> (Rig, SessionId) {
    let rig = rig_placing(None);
    let id = rig.start().await;
    holds(&rig, &id, "branch", "armada/12").await;
    let mut shown = facts(PullRequestStanding::Open);
    shown.branch = "armada/12".into();
    rig.fleet.vcs().now_pull_request(Some(shown));
    forge_ran(&rig.fleet, passed());
    rig.fleet.vcs().main_ci.watched_are(Some(vec![watched(NUMBER)]));
    (rig, id)
}

async fn read_the_pull_watch(rig: &Rig) {
    let served = rig.fleet.repositories().served().remove(0);
    rig.fleet.pulls_told(&served).await;
}

#[tokio::test]
async fn a_pull_request_that_attaches_to_a_session_with_auto_land_on_gets_auto_merge() {
    let (rig, id) = working_a_branch_with_an_open_pull_request().await;
    rig.fleet.set_session_auto_land(flag(&id, true)).await.expect("on");
    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 0, "it holds no pull request yet");

    read_the_pull_watch(&rig).await;

    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 1);
}

#[tokio::test]
async fn a_pull_request_that_attaches_with_auto_land_off_is_not_asked() {
    let (rig, _id) = working_a_branch_with_an_open_pull_request().await;

    read_the_pull_watch(&rig).await;

    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 0);
}

#[tokio::test]
async fn turning_auto_land_on_asks_for_auto_merge_on_the_open_pull_requests() {
    let (rig, id) = holding_pull_request(PullRequestStanding::Open, false).await;

    let record = rig.fleet.set_session_auto_land(flag(&id, true)).await.expect("on");

    assert!(record.auto_land, "the record reads the flag");
    assert!(row_of(&rig, &id).await.auto_land, "and so does a later read");
    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 1);
}

#[tokio::test]
async fn turning_auto_land_off_takes_auto_merge_off_the_open_pull_requests() {
    let (rig, id) = holding_pull_request(PullRequestStanding::Open, true).await;
    rig.fleet.set_session_auto_land(flag(&id, true)).await.expect("on");

    let record = rig.fleet.set_session_auto_land(flag(&id, false)).await.expect("off");

    assert!(!record.auto_land);
    assert_eq!(rig.fleet.vcs().times_asked_to_stop_auto_merge(), 1);
    let held = rig.fleet.store().lock().await.attachments_of(&Holder::session(id.as_str())).unwrap();
    let row = held.iter().find(|one| one.kind == "pr").expect("the row");
    assert_eq!(row.detail.get("auto_merge").map(String::as_str), Some("false"));
}

#[tokio::test]
async fn turning_auto_land_off_leaves_a_merged_pull_request_alone() {
    let (rig, id) = holding_pull_request(PullRequestStanding::Merged, true).await;

    rig.fleet.set_session_auto_land(flag(&id, false)).await.expect("off");

    assert_eq!(rig.fleet.vcs().times_asked_to_stop_auto_merge(), 0);
}

#[tokio::test]
async fn a_forge_that_refuses_is_reported_in_its_words_and_the_flag_still_changes() {
    let (rig, id) = holding_pull_request(PullRequestStanding::Open, false).await;
    rig.fleet.vcs().now_auto_merge(Err("auto merge is not allowed for this repository".into()));

    let refused = rig.fleet.set_session_auto_land(flag(&id, true)).await.expect_err("refused");

    assert_eq!(code(&refused), "fleet.auto_merge_refused");
    assert!(refused.error().message.contains("not allowed for this repository"));
    assert!(row_of(&rig, &id).await.auto_land, "the flag is what the person set");

    rig.fleet.vcs().now_pull_request(Some({
        let mut shown = facts(PullRequestStanding::Open);
        shown.auto_merge = true;
        shown
    }));
    rig.fleet.vcs().auto_merge_off_refuses("the forge is down");
    let refused = rig.fleet.set_session_auto_land(flag(&id, false)).await.expect_err("refused");
    assert_eq!(code(&refused), "fleet.auto_merge_refused");
    assert!(!row_of(&rig, &id).await.auto_land);
}

fn checked(state: CiState) -> WatchedPull {
    let mut pull = watched(NUMBER);
    pull.checks = vec![WatchedCheck { name: FromOutside::verbatim("ci"), state, required: true, hung: false, log_url: None }];
    pull
}

/// The job opened pull request 12 itself, as `gh pr create` reports it, with Auto land as `on`.
async fn opened_by_the_job(on: bool) -> (Rig, SessionId) {
    let rig = rig_placing(None);
    let id = rig.start().await;
    rig.fleet.set_session_auto_land(flag(&id, on)).await.expect("set");
    let attachment = AttachmentReport {
        kind: "pr".into(),
        target: NUMBER.to_string(),
        detail: [("created".to_string(), "true".to_string())].into(),
    };
    rig.fleet
        .report_session(SessionReport {
            harness: adapters::HOSTED_HARNESS.into(),
            session_id: id.clone(),
            fact: SessionFact::Attached { attachment },
        })
        .await
        .expect("attached");
    rig.fleet.vcs().now_pull_request(Some(facts(PullRequestStanding::Open)));
    forge_ran(&rig.fleet, passed());
    (rig, id)
}

/// #2679: the job opened its pull request with Auto land on, `ci` passed, and it sat open until
/// the owner asked, since only a pull request the watch or a claim attaches was asked for.
#[tokio::test]
async fn a_pull_request_the_job_opened_is_landed_once_its_checks_pass() {
    let (rig, _id) = opened_by_the_job(true).await;
    rig.fleet.vcs().main_ci.watched_are(Some(vec![checked(CiState::Pending)]));
    read_the_pull_watch(&rig).await;
    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 0, "not before its checks pass");

    rig.fleet.vcs().main_ci.watched_are(Some(vec![checked(CiState::Passed)]));
    read_the_pull_watch(&rig).await;
    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 1);

    read_the_pull_watch(&rig).await;
    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 1, "once a commit");
}

#[tokio::test]
async fn a_passed_pull_request_is_not_landed_with_auto_land_off() {
    let (rig, _id) = opened_by_the_job(false).await;
    rig.fleet.vcs().main_ci.watched_are(Some(vec![checked(CiState::Passed)]));

    read_the_pull_watch(&rig).await;

    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 0);
}

#[tokio::test]
async fn a_pull_request_that_failed_conflicts_or_is_already_merging_is_not_asked_again() {
    let mut conflicting = checked(CiState::Passed);
    conflicting.conflicting = true;
    let mut on_auto_merge = checked(CiState::Passed);
    on_auto_merge.auto_merge = true;
    let mut queued = checked(CiState::Passed);
    queued.queue = PullQueue::Waiting;
    for (named, pull) in [("failed", checked(CiState::Failed)), ("conflicting", conflicting), ("on auto-merge", on_auto_merge), ("queued", queued)] {
        let (rig, _id) = opened_by_the_job(true).await;
        rig.fleet.vcs().main_ci.watched_are(Some(vec![pull]));

        read_the_pull_watch(&rig).await;

        assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 0, "{named}");
    }
}

/// A commit seen merging is not started again: someone took it off auto-merge, or the queue
/// dropped it, and asking again would undo the one and fail the other a second time.
#[tokio::test]
async fn a_commit_that_left_the_queue_or_auto_merge_is_not_asked_again() {
    let mut queued = checked(CiState::Passed);
    queued.queue = PullQueue::Waiting;
    let mut on_auto_merge = checked(CiState::Passed);
    on_auto_merge.auto_merge = true;
    for (named, merging) in [("dropped", queued), ("taken off auto-merge", on_auto_merge)] {
        let (rig, _id) = opened_by_the_job(true).await;
        rig.fleet.vcs().main_ci.watched_are(Some(vec![merging]));
        read_the_pull_watch(&rig).await;

        rig.fleet.vcs().main_ci.watched_are(Some(vec![checked(CiState::Passed)]));
        read_the_pull_watch(&rig).await;

        assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 0, "{named}");
    }
}

#[tokio::test]
async fn a_forge_that_will_not_land_a_passed_pull_request_is_told_to_the_job() {
    let (rig, _id) = opened_by_the_job(true).await;
    rig.fleet.vcs().now_auto_merge(Err("auto merge is not allowed for this repository".into()));
    rig.fleet.vcs().main_ci.watched_are(Some(vec![checked(CiState::Passed)]));

    read_the_pull_watch(&rig).await;

    let sent: Vec<String> = (0..rig.stand_in.starts().len())
        .flat_map(|at| rig.stand_in.process(at).sent.lock().unwrap().clone())
        .collect();
    let said = sent.join("\n");
    assert!(said.contains("Auto land is on, and the forge would not turn on auto-merge for #12"), "{said}");
    assert!(said.contains("not allowed for this repository"), "{said}");
}
