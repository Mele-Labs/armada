//! What a running Autopilot answers that is a repository's and not one item of a Job's: a green pull
//! request, an issue the picker was unsure of and a red main. And whose Jobs it answers for: those
//! it started, or with `running` every live Job of the repository. The call is the fake.

use std::sync::Arc;

use adapter_traits::{CiState, FollowedCheck, FollowedPull, FromOutside, PullQueue, PullRequestStanding};
use api::{Autopilots, HostedSessions, Sessions};
use ipc::{AutopilotDecision, AutopilotSettings, AutopilotWork};
use store::{AttachmentState, Holder, KeptAttachment, KeptPick, MainCi, MainFailedJob, MainState};
use testkit::{FakeJudge, FakeTracker, FakeTrackers};

use super::autopilot_decisions::{item, keep, rig_calling, started, told, waiting, OLD, SURE_OF_THE_FIRST};
use super::pull_requesting::{facts, forge_ran, passed};
use super::session_host::{rig_fitted, Rig};

const NUMBER: u64 = 12;

fn manifest(rig: &Rig) -> String {
    rig.fleet.repositories().first().expect("served").manifest().id().as_str().to_string()
}

async fn holds_pull_request(rig: &Rig, id: &ipc::SessionId) {
    let at = rig.fleet.now().as_str().to_string();
    rig.fleet
        .store()
        .lock()
        .await
        .attach(
            &KeptAttachment {
                holder: Holder::session(id.as_str()),
                kind: "pr".into(),
                manifest_id: manifest(rig),
                target: NUMBER.to_string(),
                state: AttachmentState::Standing,
                detail: Default::default(),
                since: at.clone(),
                changed_at: at,
            },
            false,
        )
        .expect("attached");
}

fn pull(state: CiState) -> FollowedPull {
    FollowedPull {
        number: NUMBER,
        branch: FromOutside::verbatim("armada/12"),
        url: FromOutside::verbatim("https://forge.invalid/armada/pull/12"),
        author: None,
        head: Some("a".repeat(40)),
        conflicting: false,
        queue: PullQueue::Outside,
        checks: vec![FollowedCheck { name: FromOutside::verbatim("ci"), state, required: true, hung: false, log_url: None }],
        title: None,
        auto_merge: false,
    }
}

/// A Job holding open pull request 12, which the forge reads as `read`.
async fn holding(read: FollowedPull) -> (Rig, Arc<FakeJudge>, ipc::SessionId) {
    let (rig, judge) = rig_calling("choice: 1\nsure: yes\nbecause: its checks passed");
    let id = rig.start().await;
    holds_pull_request(&rig, &id).await;
    rig.fleet.vcs().now_pull_request(Some(facts(PullRequestStanding::Open)));
    forge_ran(&rig.fleet, passed());
    rig.fleet.vcs().main_ci.followed_by_number_are(vec![read]);
    (rig, judge, id)
}

#[tokio::test]
async fn a_green_pull_request_is_landed_the_way_auto_land_lands_it() {
    let (rig, judge, id) = holding(pull(CiState::Passed)).await;
    started(&rig, &[AutopilotDecision::Land]).await;

    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 1);
    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0, "a commit is decided once");

    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 1);
    assert!(judge.asked().last().unwrap().contains("Take a draft out"), "the helper reads the pull request's title");
    let rows = rig.fleet.store().lock().await.autopilot_decisions_to_review().unwrap();
    assert_eq!((rows[0].kind.as_str(), rows[0].session_id.as_deref()), ("land", Some(id.as_str())));
}

#[tokio::test]
async fn a_pull_request_whose_checks_are_not_green_is_never_landed() {
    for read in [pull(CiState::Failed), pull(CiState::Pending), FollowedPull { checks: Vec::new(), ..pull(CiState::Passed) }] {
        let (rig, judge, _) = holding(read).await;
        started(&rig, &[AutopilotDecision::Land]).await;
        let before = judge.asked().len();

        assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0);

        assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 0);
        assert_eq!(judge.asked().len(), before, "a pull request not green is not asked about");
    }
}

#[tokio::test]
async fn a_conflicting_one_a_merging_one_and_an_unchecked_land_are_left() {
    for (read, boxes) in [
        (FollowedPull { conflicting: true, ..pull(CiState::Passed) }, vec![AutopilotDecision::Land]),
        (FollowedPull { auto_merge: true, ..pull(CiState::Passed) }, vec![AutopilotDecision::Land]),
        (FollowedPull { queue: PullQueue::Waiting, ..pull(CiState::Passed) }, vec![AutopilotDecision::Land]),
        (pull(CiState::Passed), vec![AutopilotDecision::Plan]),
    ] {
        let (rig, _, _) = holding(read).await;
        started(&rig, &boxes).await;
        assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0);
        assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), 0);
    }
}

#[tokio::test]
async fn a_job_with_auto_land_on_is_left_to_auto_land() {
    let (rig, _, id) = holding(pull(CiState::Passed)).await;
    started(&rig, &[AutopilotDecision::Land]).await;
    rig.fleet.set_session_auto_land(ipc::SetSessionAutoLand { id, on: true }).await.expect("on");
    let asked = rig.fleet.vcs().times_asked_for_auto_merge();

    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0);

    assert_eq!(rig.fleet.vcs().times_asked_for_auto_merge(), asked);
}

// ---- An issue the picker was unsure of ----

async fn set_aside(rig: &Rig, title: &str) -> String {
    let run = rig.fleet.store().lock().await.autopilot(rig.manifest.as_str()).unwrap().expect("held").run_id;
    let pick = KeptPick {
        pick_id: "01PICK".into(),
        run_id: run,
        manifest_id: rig.manifest.as_str().to_string(),
        issue_reference: "#5".into(),
        issue_title: title.into(),
        issue_url: "no-link-resolves-this".into(),
        workflow_id: None,
        started_session: None,
        state: "set_aside".into(),
        reason: "the instruction does not say whether it covers docs".into(),
        at: OLD.into(),
    };
    rig.fleet.store().lock().await.add_autopilot_pick(&pick).expect("kept");
    pick.pick_id
}

#[tokio::test]
async fn an_issue_the_picker_set_aside_is_started_when_unsure_is_checked_and_the_helper_is_sure() {
    let (rig, judge) = rig_calling("choice: 1\nsure: yes\nbecause: the instruction covers it");
    started(&rig, &[AutopilotDecision::Unsure]).await;
    let pick = set_aside(&rig, "Fix the reader").await;

    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 1);
    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0, "it is started once");

    assert!(judge.asked().last().unwrap().contains("the instruction does not say whether it covers docs"));
    let started_as = rig.fleet.get_autopilot(rig.manifest.clone()).await.expect("held");
    let one = started_as.picks.iter().find(|one| one.pick_id == pick).expect("the pick");
    assert_eq!(one.state, ipc::AutopilotPickState::Started);
    assert!(one.session_id.is_some(), "a Job started on it");
}

#[tokio::test]
async fn an_issue_set_aside_waits_when_unsure_is_not_checked_or_the_helper_is_not_sure() {
    let (rig, _) = rig_calling("choice: 1\nsure: yes\nbecause: x");
    started(&rig, &[AutopilotDecision::Land]).await;
    set_aside(&rig, "Fix the reader").await;
    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0);

    let (rig, _) = rig_calling("choice: 1\nsure: no\nbecause: it may need a decision");
    started(&rig, &[AutopilotDecision::Unsure]).await;
    set_aside(&rig, "Fix the reader").await;
    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0);

    let (rig, judge) = rig_calling("choice: 1\nsure: yes\nbecause: x");
    started(&rig, &EVERY).await;
    set_aside(&rig, "Remove the legacy reader").await;
    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0, "destructive waits");
    assert!(judge.asked().is_empty());
}

const EVERY: [AutopilotDecision; 4] =
    [AutopilotDecision::Unsure, AutopilotDecision::Red, AutopilotDecision::Land, AutopilotDecision::Open];

// ---- A red main ----

async fn red_main(rig: &Rig, commit: &str) {
    let at = rig.fleet.now();
    let main = MainCi {
        repository: rig.root.clone(),
        base: "main".into(),
        commit: commit.into(),
        state: MainState::Red,
        read_at: at.clone(),
        red_at: Some(at),
        red_commit: Some(commit.into()),
        decided_commit: Some(commit.into()),
        newer_running: 0,
        unfinished: 0,
        failed: vec![MainFailedJob { name: "ci".into(), check: None, log_url: None, tests: vec!["reader::stops".into()] }],
        merge: None,
    };
    rig.fleet.store().lock().await.record_main_ci(&main).expect("kept");
}

async fn jobs_titled(rig: &Rig, title: &str) -> usize {
    let store = rig.fleet.store().lock().await;
    let found = store.find_sessions(&store::SessionSearch { manifest_id: None, state: None, text: None }).unwrap();
    found.iter().filter(|one| one.title.as_deref() == Some(title)).count()
}

#[tokio::test]
async fn a_red_main_is_fixed_by_one_job_once_for_each_red_commit() {
    let (rig, judge) = rig_calling(SURE_OF_THE_FIRST);
    started(&rig, &[AutopilotDecision::Red]).await;
    red_main(&rig, &"b".repeat(40)).await;

    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 1);
    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0, "a Job holds it, and it was started for");

    assert_eq!(jobs_titled(&rig, "Fix main").await, 1);
    assert!(judge.asked().is_empty(), "no call is made for a red");
    let rows = rig.fleet.store().lock().await.autopilot_decisions_to_review().unwrap();
    assert_eq!(rows[0].kind, "red");
    let main = rig.fleet.store().lock().await.main_ci(&rig.root).unwrap().unwrap();
    assert!(rig.fleet.taking(&main).await.is_some(), "the red is claimed for the Job it started");
}

#[tokio::test]
async fn a_red_waits_when_red_is_not_checked_or_a_live_job_holds_it() {
    let (rig, _) = rig_calling(SURE_OF_THE_FIRST);
    started(&rig, &[AutopilotDecision::Land]).await;
    red_main(&rig, &"b".repeat(40)).await;
    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0);
    assert_eq!(jobs_titled(&rig, "Fix main").await, 0);

    let (rig, _) = rig_calling(SURE_OF_THE_FIRST);
    let holder = rig.start().await;
    started(&rig, &[AutopilotDecision::Red]).await;
    red_main(&rig, &"b".repeat(40)).await;
    let main = rig.fleet.store().lock().await.main_ci(&rig.root).unwrap().unwrap();
    rig.fleet.recorded_session_take(&main, holder.as_str(), store::TakenHow::Took, None).await;
    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0, "a live Job holds it");
    assert_eq!(jobs_titled(&rig, "Fix main").await, 0);
}

// ---- Whose Jobs ----

#[tokio::test]
async fn without_running_only_the_jobs_autopilot_started_are_answered() {
    let (rig, judge) = rig_calling(SURE_OF_THE_FIRST);
    let id = rig.start().await;
    let settings = |running| AutopilotSettings {
        instruction: "Fix what is open".into(),
        at_once: 1,
        until: None,
        spend_cap_micros: None,
        session_cap: None,
        on_autopilot: AutopilotWork { backlog: false, running },
        decisions: [AutopilotDecision::Recommended].into_iter().collect(),
    };
    rig.fleet.start_autopilot(rig.manifest.clone(), settings(false)).await.expect("started");
    keep(&rig, &id, &[item("a", "Which clock?", &["Frozen (Recommended)", "Fake"])]).await;

    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 0, "a Job Autopilot did not start is not its to answer");
    assert!(judge.asked().is_empty());

    rig.fleet.stop_autopilot(rig.manifest.clone()).await.expect("stopped");
    rig.fleet.start_autopilot(rig.manifest.clone(), settings(true)).await.expect("started");
    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 1, "with running it brings every live Job under the rules");
    assert!(told(&rig, &id).await.last().unwrap().starts_with("Re: Which clock?"));
    assert!(waiting(&rig, &id).await.is_empty());
}

#[tokio::test]
async fn an_autopilot_answers_for_the_jobs_it_started_by_a_pick() {
    let (rig, _) = rig_calling("choice: 1\nsure: yes\nbecause: the instruction covers it");
    started(&rig, &[AutopilotDecision::Unsure, AutopilotDecision::Recommended]).await;
    set_aside(&rig, "Fix the reader").await;
    rig.fleet.decision_pass().await.expect("passed");
    let session = rig.fleet.store().lock().await.autopilot_decisions_to_review().unwrap()[0].session_id.clone().expect("a Job");
    let session = ipc::SessionId::carried(session);
    keep(&rig, &session, &[item("a", "Which clock?", &["Frozen (Recommended)", "Fake"])]).await;

    assert_eq!(rig.fleet.decision_pass().await.expect("passed"), 1);

    assert!(waiting(&rig, &session).await.is_empty());
}

// ---- The backlog ----

#[tokio::test]
async fn with_the_backlog_off_the_pass_picks_no_issues() {
    for (backlog, picks) in [(false, false), (true, true)] {
        let judge = Arc::new(FakeJudge::saying("issue: none\nworkflow: none\nsure: no\nbecause: nothing fits"));
        let held = Arc::clone(&judge);
        let rig = rig_fitted(move |fittings| {
            fittings.judge = held;
            fittings.trackers = Arc::new(FakeTrackers::empty().tracker(Arc::new(FakeTracker::listing(vec![FakeTracker::issue(5, "Fix the reader")]))));
        });
        let settings = AutopilotSettings {
            instruction: "Fix what is open".into(),
            at_once: 1,
            until: None,
            spend_cap_micros: None,
            session_cap: None,
            on_autopilot: AutopilotWork { backlog, running: false },
            decisions: Default::default(),
        };
        rig.fleet.start_autopilot(rig.manifest.clone(), settings).await.expect("started");

        rig.fleet.autopilot_pass().await.expect("passed");

        let asked = judge.asked().iter().any(|question| question.contains("You choose the next issue"));
        assert_eq!(asked, picks, "backlog {backlog}");
    }
}
