//! Auto land on a hosted Session: the forge's auto-merge follows the flag. **No forge is called**:
//! `FakeVcs` stands in for `gh`, and a test says what was asked of it by counting.

use adapter_traits::{CiState, FromOutside, PullQueue, PullRequestStanding, FollowedCheck, FollowedPull};
use api::{HostedSessions, Sessions};
use ipc::{AttachmentReport, SessionFact, SessionId, SessionRecord, SessionReport, SetSessionAutoLand};
use store::{AttachmentState, Holder, KeptAttachment};

use super::pull_requesting::{facts, forge_ran, passed};
use super::session_host::{rig_placing, Rig};

const NUMBER: u64 = 12;

fn flag(id: &SessionId, on: bool) -> SetSessionAutoLand {
    SetSessionAutoLand { id: id.clone(), on }
}

/// Auto land set, and the forge's half waited on: what the forge was asked is settled after.
pub(super) async fn auto_land(rig: &Rig, id: &SessionId, on: bool) -> SessionRecord {
    let (record, asking) = rig.fleet.auto_land_answered(flag(id, on)).await.expect("set");
    asking.await.expect("the forge's half ran");
    record
}

/// What Fleet told the Job, every process's sends joined.
fn told(rig: &Rig) -> String {
    let sent: Vec<String> = (0..rig.stand_in.starts().len())
        .flat_map(|at| rig.stand_in.process(at).sent.lock().unwrap().clone())
        .collect();
    sent.join("\n")
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

fn followed(number: u64) -> FollowedPull {
    FollowedPull {
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
        draft: false,
        approved: false,
    }
}

/// The session worked branch `armada/12`, and the forge shows an open pull request from it that
/// nobody has reported: the pull reading attaches it on its next read.
async fn working_a_branch_with_an_open_pull_request() -> (Rig, SessionId) {
    let rig = rig_placing(None);
    let id = rig.start().await;
    holds(&rig, &id, "branch", "armada/12").await;
    let mut shown = facts(PullRequestStanding::Open);
    shown.branch = "armada/12".into();
    rig.fleet.vcs().now_pull_request(Some(shown));
    forge_ran(&rig.fleet, passed());
    rig.fleet.vcs().main_ci.followed_are(Some(vec![followed(NUMBER)]));
    (rig, id)
}

async fn read_the_pull_reading(rig: &Rig) {
    let served = rig.fleet.repositories().served().remove(0);
    rig.fleet.pulls_told(&served).await;
}

#[tokio::test]
async fn a_pull_request_that_attaches_to_a_session_with_auto_land_on_gets_auto_merge() {
    let (rig, id) = working_a_branch_with_an_open_pull_request().await;
    auto_land(&rig, &id, true).await;
    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 0, "it holds no pull request yet");

    read_the_pull_reading(&rig).await;

    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 1);
}

#[tokio::test]
async fn a_pull_request_that_attaches_with_auto_land_off_is_not_asked() {
    let (rig, _id) = working_a_branch_with_an_open_pull_request().await;

    read_the_pull_reading(&rig).await;

    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 0);
}

#[tokio::test]
async fn turning_auto_land_on_asks_for_auto_merge_on_the_open_pull_requests() {
    let (rig, id) = holding_pull_request(PullRequestStanding::Open, false).await;

    let record = auto_land(&rig, &id, true).await;

    assert!(record.auto_land, "the record reads the flag");
    assert!(row_of(&rig, &id).await.auto_land, "and so does a later read");
    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 1);
}

/// 4 Nov 2026: a Job holding five pull requests asked the forge some twenty-five times inside the
/// request, and Bridge gave up at twenty seconds. The answer comes before the forge is asked.
#[tokio::test]
async fn turning_auto_land_on_answers_before_the_forge_is_asked() {
    let (rig, id) = holding_pull_request(PullRequestStanding::Open, false).await;

    let record = std::sync::Arc::clone(&rig.fleet).set_session_auto_land(flag(&id, true)).await.expect("answered");

    assert!(record.auto_land, "the record reads the flag");
    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 0, "the answer did not wait on the forge");
    super::session_host::eventually(|| async { rig.fleet.vcs().times_asked_for_auto_merge() == 1 }).await;
}

#[tokio::test]
async fn an_off_pressed_after_an_on_is_not_undone_by_the_on() {
    let (rig, id) = holding_pull_request(PullRequestStanding::Open, false).await;

    // The on's half of the forge, running once the off has kept the flag.
    rig.fleet.auto_land_asked_of_forge(id.as_str(), &manifest(&rig), true).await;

    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 0, "the on's ask saw the flag off");
}

#[tokio::test]
async fn turning_auto_land_off_takes_auto_merge_off_the_open_pull_requests() {
    let (rig, id) = holding_pull_request(PullRequestStanding::Open, true).await;
    auto_land(&rig, &id, true).await;

    let record = auto_land(&rig, &id, false).await;

    assert!(!record.auto_land);
    assert_eq!(rig.fleet.vcs().times_asked_to_stop_auto_merge(), 1);
    let held = rig.fleet.store().lock().await.attachments_of(&Holder::session(id.as_str())).unwrap();
    let row = held.iter().find(|one| one.kind == "pr").expect("the row");
    assert_eq!(row.detail.get("auto_merge").map(String::as_str), Some("false"));
}

#[tokio::test]
async fn turning_auto_land_off_leaves_a_merged_pull_request_alone() {
    let (rig, id) = holding_pull_request(PullRequestStanding::Merged, true).await;

    auto_land(&rig, &id, false).await;

    assert_eq!(rig.fleet.vcs().times_asked_to_stop_auto_merge(), 0);
}

/// The forge's refusal is told to the Job in its words, never the answer to the press: the flag is
/// what the person set.
#[tokio::test]
async fn a_forge_that_refuses_is_told_to_the_job_in_its_words_and_the_flag_still_changes() {
    let (rig, id) = holding_pull_request(PullRequestStanding::Open, false).await;
    rig.fleet.vcs().now_auto_merge(Err("auto merge is not allowed for this repository".into()));

    let record = std::sync::Arc::clone(&rig.fleet).set_session_auto_land(flag(&id, true)).await.expect("answered, not refused");

    assert!(record.auto_land, "the flag is what the person set");
    super::session_host::eventually(|| async { told(&rig).contains("not allowed for this repository") }).await;
    assert!(told(&rig).contains("Auto land is on, and the forge would not turn on auto-merge for #12"), "{}", told(&rig));

    rig.fleet.vcs().now_pull_request(Some({
        let mut shown = facts(PullRequestStanding::Open);
        shown.auto_merge = true;
        shown
    }));
    rig.fleet.vcs().auto_merge_off_refuses("the forge is down");
    let record = auto_land(&rig, &id, false).await;
    assert!(!record.auto_land);
    assert!(told(&rig).contains("Auto land is off, and the forge would not turn off auto-merge for #12"), "{}", told(&rig));
    assert!(!row_of(&rig, &id).await.auto_land);
}

fn checked(state: CiState) -> FollowedPull {
    let mut pull = followed(NUMBER);
    pull.checks = vec![FollowedCheck { name: FromOutside::verbatim("ci"), state, required: true, hung: false, log_url: None }];
    pull
}

/// The job opened pull request 12 itself, as `gh pr create` reports it, with Auto land as `on`.
async fn opened_by_the_job(on: bool) -> (Rig, SessionId) {
    let rig = rig_placing(None);
    let id = rig.start().await;
    auto_land(&rig, &id, on).await;
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
/// the owner asked, since only a pull request the pull reading or a claim attaches was asked for.
#[tokio::test]
async fn a_pull_request_the_job_opened_is_landed_once_its_checks_pass() {
    let (rig, _id) = opened_by_the_job(true).await;
    rig.fleet.vcs().main_ci.followed_are(Some(vec![checked(CiState::Pending)]));
    read_the_pull_reading(&rig).await;
    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 0, "not before its checks pass");

    rig.fleet.vcs().main_ci.followed_are(Some(vec![checked(CiState::Passed)]));
    read_the_pull_reading(&rig).await;
    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 1);

    read_the_pull_reading(&rig).await;
    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 1, "once a commit");
}

#[tokio::test]
async fn a_passed_pull_request_is_not_landed_with_auto_land_off() {
    let (rig, _id) = opened_by_the_job(false).await;
    rig.fleet.vcs().main_ci.followed_are(Some(vec![checked(CiState::Passed)]));

    read_the_pull_reading(&rig).await;

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
        rig.fleet.vcs().main_ci.followed_are(Some(vec![pull]));

        read_the_pull_reading(&rig).await;

        assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 0, "{named}");
    }
}

/// A commit seen merging that is then green and neither queued nor on auto-merge is asked once
/// more, whoever took it off: with Auto land on, Fleet merges a green pull request.
#[tokio::test]
async fn a_commit_that_left_the_queue_or_auto_merge_is_asked_once_more() {
    let mut queued = checked(CiState::Passed);
    queued.queue = PullQueue::Waiting;
    let mut on_auto_merge = checked(CiState::Passed);
    on_auto_merge.auto_merge = true;
    for (named, merging) in [("dropped", queued), ("taken off auto-merge", on_auto_merge)] {
        let (rig, _id) = opened_by_the_job(true).await;
        rig.fleet.vcs().main_ci.followed_are(Some(vec![merging]));
        read_the_pull_reading(&rig).await;

        rig.fleet.vcs().main_ci.followed_are(Some(vec![checked(CiState::Passed)]));
        read_the_pull_reading(&rig).await;

        assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 1, "{named}");
    }
}

/// #3217: the queue dropped a green pull request between two readings and nothing merged it, and
/// nobody was told. Asked once more, then the Job is told, once.
#[tokio::test]
async fn a_green_pull_request_that_nothing_merges_is_asked_once_more_then_told_to_the_job() {
    let mut queued = checked(CiState::Passed);
    queued.queue = PullQueue::Waiting;
    let (rig, _id) = opened_by_the_job(true).await;
    rig.fleet.vcs().main_ci.followed_are(Some(vec![queued]));
    read_the_pull_reading(&rig).await;

    rig.fleet.vcs().main_ci.followed_are(Some(vec![checked(CiState::Passed)]));
    for _ in 0..3 {
        read_the_pull_reading(&rig).await;
    }

    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 1, "once more, not every reading");
    let sent: Vec<String> = (0..rig.stand_in.starts().len())
        .flat_map(|at| rig.stand_in.process(at).sent.lock().unwrap().clone())
        .collect();
    let said = sent.join("\n");
    assert_eq!(said.matches("neither on auto-merge nor in the merge queue").count(), 1, "{said}");
    assert!(said.contains("#12"), "{said}");
}

#[tokio::test]
async fn a_forge_that_will_not_land_a_passed_pull_request_is_told_to_the_job() {
    let (rig, _id) = opened_by_the_job(true).await;
    rig.fleet.vcs().now_auto_merge(Err("auto merge is not allowed for this repository".into()));
    rig.fleet.vcs().main_ci.followed_are(Some(vec![checked(CiState::Passed)]));

    read_the_pull_reading(&rig).await;

    let sent: Vec<String> = (0..rig.stand_in.starts().len())
        .flat_map(|at| rig.stand_in.process(at).sent.lock().unwrap().clone())
        .collect();
    let said = sent.join("\n");
    assert!(said.contains("Auto land is on, and the forge would not turn on auto-merge for #12"), "{said}");
    assert!(said.contains("not allowed for this repository"), "{said}");
}

fn approved(state: CiState) -> FollowedPull {
    FollowedPull { approved: true, ..checked(state) }
}

#[tokio::test]
async fn an_approved_green_pull_request_is_told_ready_to_merge_once_with_auto_land_off() {
    let (rig, _id) = opened_by_the_job(false).await;
    rig.fleet.vcs().main_ci.followed_are(Some(vec![approved(CiState::Passed)]));

    read_the_pull_reading(&rig).await;
    assert!(told(&rig).contains("#12 is approved and its checks passed"), "{}", told(&rig));

    read_the_pull_reading(&rig).await;
    assert_eq!(told(&rig).matches("is ready to merge").count(), 1, "once a commit");
}

#[tokio::test]
async fn a_pull_request_is_told_ready_again_at_a_new_commit() {
    let (rig, _id) = opened_by_the_job(false).await;
    rig.fleet.vcs().main_ci.followed_are(Some(vec![approved(CiState::Passed)]));
    read_the_pull_reading(&rig).await;

    rig.fleet.vcs().main_ci.followed_are(Some(vec![FollowedPull { head: Some("b".repeat(40)), ..approved(CiState::Passed) }]));
    read_the_pull_reading(&rig).await;

    assert_eq!(told(&rig).matches("is ready to merge").count(), 2, "{}", told(&rig));
}

#[tokio::test]
async fn a_pull_request_is_not_told_ready_with_auto_land_on() {
    let (rig, _id) = opened_by_the_job(true).await;
    rig.fleet.vcs().main_ci.followed_are(Some(vec![approved(CiState::Passed)]));

    read_the_pull_reading(&rig).await;

    assert!(!told(&rig).contains("is ready to merge"), "{}", told(&rig));
}

#[tokio::test]
async fn a_pull_request_is_not_told_ready_until_approved_green_and_clear() {
    let mut draft = approved(CiState::Passed);
    draft.draft = true;
    let mut conflicting = approved(CiState::Passed);
    conflicting.conflicting = true;
    let mut on_auto_merge = approved(CiState::Passed);
    on_auto_merge.auto_merge = true;
    let mut queued = approved(CiState::Passed);
    queued.queue = PullQueue::Waiting;
    for (named, pull) in [
        ("not approved", checked(CiState::Passed)),
        ("red", approved(CiState::Failed)),
        ("pending", approved(CiState::Pending)),
        ("draft", draft),
        ("conflicting", conflicting),
        ("on auto-merge", on_auto_merge),
        ("queued", queued),
    ] {
        let (rig, _id) = opened_by_the_job(false).await;
        rig.fleet.vcs().main_ci.followed_are(Some(vec![pull]));

        read_the_pull_reading(&rig).await;

        assert!(!told(&rig).contains("is ready to merge"), "{named}: {}", told(&rig));
    }
}
