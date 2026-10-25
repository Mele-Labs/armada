//! What Fleet tells the owner of a pull request, against a scripted forge. The forge's words are
//! `adapters`' to assert; under test here is who is told what, when, and how many times: a hosted
//! Session, a terminal one and a Job, a hung job started again without a word, and a restart that
//! tells nobody twice.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CiState, FromOutside, PullQueue, RecentlyMerged, FollowedCheck, FollowedPull};
use api::{HostedSessions, Sessions};
use ipc::{
    ManifestId, SendSessionMessage, SessionFact, SessionId, SessionOrigin, SessionReport, SessionRow, SessionVoice,
    SetWaitingFor, StartSession, TakeHeld, WaitingInput, WaitingSource,
};
use store::{AttachmentState, Holder, KeptAttachment};
use testkit::{Delivered, FakeHarness, FakeVcs, FakeWorkProduct};

use super::main_ci::{run, ONE};
use super::main_fix::a_fleet_holding_bug;
use super::session_host::{eventually, Shared, StandIn};
use crate::daemon::Fleet;
use crate::tests::slot_leases::{leased, released, By};
use crate::tests::tmp::TempDir;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const LOG_URL: &str = "https://forge.invalid/armada/actions/runs/9/job/31";

pub(super) fn check(name: &str, state: CiState, required: bool) -> FollowedCheck {
    FollowedCheck {
        name: FromOutside::verbatim(name),
        state,
        required,
        hung: false,
        log_url: Some(FromOutside::verbatim(LOG_URL)),
    }
}

pub(super) fn pull(number: u64, checks: Vec<FollowedCheck>) -> FollowedPull {
    FollowedPull {
        number,
        branch: FromOutside::verbatim(format!("armada/{number}")),
        url: FromOutside::verbatim(format!("https://forge.invalid/armada/pull/{number}")),
        author: None,
        head: Some(ONE.to_string()),
        conflicting: false,
        queue: PullQueue::Outside,
        checks,
        title: None,
        auto_merge: false,
    }
}

pub(super) fn failing(number: u64) -> FollowedPull {
    pull(
        number,
        vec![
            check("test", CiState::Failed, false),
            check("ci", CiState::Failed, true),
        ],
    )
}

pub(super) struct Rig {
    pub(super) fleet: Arc<Hosted>,
    stand_in: Arc<StandIn>,
    pub(super) _home: TempDir,
}

impl Rig {
    fn around(fleet: Hosted, stand_in: Arc<StandIn>, home: TempDir) -> Rig {
        Rig {
            fleet: Arc::new(fleet),
            stand_in,
            _home: home,
        }
    }

    pub(super) fn manifest(&self) -> String {
        self.fleet
            .repositories()
            .first()
            .expect("served")
            .manifest()
            .id()
            .as_str()
            .to_string()
    }

    pub(super) async fn reads(&self) {
        let served = self.fleet.repositories().served().remove(0);
        self.fleet.pulls_told(&served).await;
    }

    /// Fleet restarted: nothing it kept in memory survives, the store does.
    async fn restarts(&self) {
        *self.fleet.sweeping().lock().await = Default::default();
    }

    pub(super) async fn holds(&self, holder: Holder, kind: &str, target: &str, state: AttachmentState) {
        let at = self.fleet.now().as_str().to_string();
        self.fleet
            .store()
            .lock()
            .await
            .attach(
                &KeptAttachment {
                    holder,
                    kind: kind.into(),
                    manifest_id: self.manifest(),
                    target: target.into(),
                    state,
                    detail: Default::default(),
                    since: at.clone(),
                    changed_at: at,
                },
                false,
            )
            .unwrap();
    }

    pub(super) async fn a_hosted_session_on(&self, number: u64) -> String {
        let id = self
            .fleet
            .start_session(StartSession {
                manifest_id: ManifestId::carried(self.manifest()),
                title: None,
                model: None,
                effort: None,
                mode: None,
                root: None,
                fork: None,
                request: None,
                after: None,
                held: false,
            })
            .await
            .expect("started")
            .id;
        self.holds(
            Holder::session(id.as_str()),
            "pr",
            &number.to_string(),
            AttachmentState::Standing,
        )
        .await;
        id.as_str().to_string()
    }

    async fn a_terminal_session_on(&self, id: &str, number: u64) {
        self.fleet
            .report_session(SessionReport {
                harness: adapters::HOSTED_HARNESS.into(),
                session_id: SessionId::carried(id),
                fact: SessionFact::Started {
                    cwd: self
                        .fleet
                        .repositories()
                        .first()
                        .unwrap()
                        .root()
                        .to_string(),
                    title: None,
                    origin: SessionOrigin::Terminal,
                    mod_version: None,
                },
            })
            .await
            .unwrap();
        self.holds(
            Holder::session(id),
            "pr",
            &number.to_string(),
            AttachmentState::Standing,
        )
        .await;
    }

    async fn the_mod_asks(&self, id: &str) -> Vec<String> {
        self.fleet
            .take_held_messages(TakeHeld {
                session_id: id.into(),
                wait_ms: None,
            })
            .await
            .unwrap()
            .messages
    }

    async fn thread_of(&self, id: &str) -> Vec<SessionRow> {
        Arc::clone(&self.fleet)
            .get_session(SessionId::carried(id))
            .await
            .unwrap()
            .rows
    }

    pub(super) async fn told(&self, recipient: &str, number: u64, cause: &str) -> bool {
        let root = self
            .fleet
            .repositories()
            .first()
            .unwrap()
            .root()
            .to_string();
        self.fleet
            .store()
            .lock()
            .await
            .pull_notice_kept(
                &root,
                number,
                if cause == "merged" { "" } else { ONE },
                cause,
                recipient,
            )
            .unwrap()
    }

    pub(super) fn runs_sent_to_the_session(&self) -> Vec<String> {
        let started = self.stand_in.starts().len();
        (0..started)
            .flat_map(|at| self.stand_in.process(at).sent.lock().unwrap().clone())
            .collect()
    }
}

pub(super) fn a_rig() -> Rig {
    let home = TempDir::new();
    let stand_in = Arc::new(StandIn::default());
    let fleet = a_fleet_holding_bug(&home, false).hosting_sessions_on(
        Arc::new(Shared(Arc::clone(&stand_in))),
        Duration::from_secs(600),
    );
    Rig::around(fleet, stand_in, home)
}

#[tokio::test]
async fn a_failed_required_check_wakes_the_hosted_session_that_holds_the_pull_request() {
    let rig = a_rig();
    let session = rig.a_hosted_session_on(7).await;
    let forge = &rig.fleet.vcs().main_ci;
    forge.runs_on(ONE, vec![run("test", "31", CiState::Failed)]);
    forge.log_of("31", "one\nFAIL tests::one\nerror: test run failed\n");
    forge.followed_are(Some(vec![failing(7)]));

    rig.reads().await;

    let sent = rig.runs_sent_to_the_session();
    let [line] = sent.as_slice() else {
        panic!("one turn started: {sent:?}")
    };
    for word in [
        "#7 failed its checks",
        "1111111",
        "test: ",
        LOG_URL,
        "FAIL tests::one",
        "armada/7",
    ] {
        assert!(
            line.contains(word),
            "{word} is in what the session is told: {line}"
        );
    }
    let rows = rig.thread_of(&session).await;
    assert!(
        rows.iter().any(|row| matches!(
            row,
            SessionRow::Message { from: SessionVoice::Session { title, .. }, .. } if title == "Fleet"
        )),
        "drawn as Fleet's, not as the person's: {rows:?}"
    );

    rig.reads().await;
    assert_eq!(
        rig.runs_sent_to_the_session().len(),
        1,
        "told once for this commit"
    );
}

#[tokio::test]
async fn a_check_that_is_not_required_does_not_fail_the_pull_request() {
    let rig = a_rig();
    rig.a_hosted_session_on(7).await;
    rig.fleet.vcs().main_ci.followed_are(Some(vec![pull(
        7,
        vec![
            check("lint", CiState::Failed, false),
            check("ci", CiState::Passed, true),
        ],
    )]));
    rig.reads().await;
    assert!(rig.runs_sent_to_the_session().is_empty());
}

#[tokio::test]
async fn a_conflict_is_held_for_the_terminal_session_and_not_repeated_after_a_restart() {
    let rig = a_rig();
    rig.a_terminal_session_on("term-1", 8).await;
    assert!(
        rig.the_mod_asks("term-1").await.is_empty(),
        "now it is listening"
    );
    let mut conflicting = pull(8, vec![check("ci", CiState::Passed, true)]);
    conflicting.conflicting = true;
    rig.fleet.vcs().main_ci.followed_are(Some(vec![conflicting]));

    rig.reads().await;
    let held = rig.the_mod_asks("term-1").await;
    let [said] = held.as_slice() else {
        panic!("one message held: {held:?}")
    };
    assert!(said.starts_with("Fleet: #8 conflicts with"), "{said}");

    rig.restarts().await;
    rig.reads().await;
    assert!(
        rig.the_mod_asks("term-1").await.is_empty(),
        "a restart does not tell it again"
    );
}

#[tokio::test]
async fn a_terminal_session_that_is_not_listening_is_told_when_its_mod_next_asks() {
    let rig = a_rig();
    rig.a_terminal_session_on("term-2", 9).await;
    let mut conflicting = pull(9, vec![]);
    conflicting.conflicting = true;
    rig.fleet.vcs().main_ci.followed_are(Some(vec![conflicting]));

    rig.reads().await;
    assert!(
        !rig.told("session:term-2", 9, "conflicts").await,
        "nothing kept for a notice not delivered"
    );

    rig.the_mod_asks("term-2").await;
    rig.reads().await;
    assert_eq!(rig.the_mod_asks("term-2").await.len(), 1);
    assert!(rig.told("session:term-2", 9, "conflicts").await);
}

#[tokio::test]
async fn leaving_the_merge_queue_is_told_once_and_an_unmergeable_entry_is_told_at_once() {
    let rig = a_rig();
    let session = rig.a_hosted_session_on(10).await;
    let mut queued = pull(10, vec![]);
    queued.queue = PullQueue::Waiting;
    rig.fleet.vcs().main_ci.followed_are(Some(vec![queued]));
    rig.reads().await;
    assert!(
        rig.runs_sent_to_the_session().is_empty(),
        "waiting in the queue is nothing to say"
    );

    rig.fleet
        .vcs()
        .main_ci
        .followed_are(Some(vec![pull(10, vec![])]));
    rig.reads().await;
    rig.restarts().await;
    rig.reads().await;
    let sent = rig.runs_sent_to_the_session();
    let [line] = sent.as_slice() else {
        panic!("told once: {sent:?}")
    };
    assert!(line.contains("#10 left the merge queue"), "{line}");

    let mut stuck = pull(11, vec![]);
    stuck.queue = PullQueue::Unmergeable;
    rig.holds(
        Holder::session(&session),
        "pr",
        "11",
        AttachmentState::Standing,
    )
    .await;
    rig.fleet.vcs().main_ci.followed_are(Some(vec![stuck]));
    rig.reads().await;
    assert!(rig
        .runs_sent_to_the_session()
        .iter()
        .any(|line| line.contains("#11 is unmergeable")));
}

/// #2649: queued again with no new commit, and dropped again, the second drop is news.
#[tokio::test]
async fn a_second_drop_out_of_the_queue_at_one_commit_is_told_again() {
    let rig = a_rig();
    rig.a_hosted_session_on(12).await;
    let mut queued = pull(12, vec![]);
    queued.queue = PullQueue::Waiting;
    for _ in 0..2 {
        rig.fleet.vcs().main_ci.followed_are(Some(vec![queued.clone()]));
        rig.reads().await;
        rig.fleet.vcs().main_ci.followed_are(Some(vec![pull(12, vec![])]));
        rig.reads().await;
    }
    let sent = rig.runs_sent_to_the_session();
    let drops = sent.iter().filter(|line| line.contains("#12 left the merge queue")).count();
    assert_eq!(drops, 2, "{sent:?}");
}

/// #2649: the queue failed a check its pull request's own run skips, and the Job, told only that
/// it left, saw green and queued it again unchanged.
#[tokio::test]
async fn a_drop_names_the_checks_the_queue_failed_with_the_end_of_a_log() {
    let rig = a_rig();
    rig.a_hosted_session_on(13).await;
    let tested = "2222222222222222222222222222222222222222";
    let ci = &rig.fleet.vcs().main_ci;
    ci.queue_commit_is(13, tested);
    ci.runs_on(
        tested,
        vec![
            run("components_test", "41", CiState::Failed),
            run("typecheck", "42", CiState::Passed),
        ],
    );
    ci.log_of("41", "FAIL NeedsYouPanel.stories.tsx > An Empty List Folds And Goes");
    let mut queued = pull(13, vec![check("ci", CiState::Passed, true)]);
    queued.queue = PullQueue::Waiting;
    ci.followed_are(Some(vec![queued]));
    rig.reads().await;
    ci.followed_are(Some(vec![pull(13, vec![check("ci", CiState::Passed, true)])]));
    rig.reads().await;

    let sent = rig.runs_sent_to_the_session();
    let [line] = sent.as_slice() else { panic!("told once: {sent:?}") };
    assert!(line.contains("#13 left the merge queue"), "{line}");
    assert!(line.contains("components_test: https://forge.invalid/armada/actions/runs/9/job/41"), "{line}");
    assert!(line.contains("An Empty List Folds And Goes"), "{line}");
    assert!(!line.contains("typecheck"), "a check that passed is not named: {line}");
}

#[tokio::test]
async fn a_merge_reaches_the_session_that_opened_it_after_its_slot_was_released() {
    let rig = a_rig();
    let session = rig.a_hosted_session_on(1).await;
    // What a Session that opened the pull request holds in the ledger: its `pr` row, spent by
    // the merge, and a `branch` row already given back with the slot.
    rig.holds(
        Holder::session(&session),
        "pr",
        "1",
        AttachmentState::Spent,
    )
    .await;
    rig.holds(
        Holder::session(&session),
        "branch",
        "armada/1",
        AttachmentState::GivenBack,
    )
    .await;
    rig.fleet.vcs().main_ci.followed_are(Some(Vec::new()));
    rig.fleet
        .vcs()
        .main_ci
        .recently_merged_are(Some(vec![RecentlyMerged {
            number: 1,
            title: FromOutside::verbatim("Fix the reader"),
            branch: FromOutside::verbatim("armada/1"),
            url: FromOutside::verbatim("https://forge.invalid/armada/pull/1"),
            author: None,
            merged_at: FromOutside::verbatim(rig.fleet.now().as_str()),
            commit: None,
        }]));
    let served = rig.fleet.repositories().served().remove(0);
    rig.fleet.notice_merged(&served).await;

    rig.reads().await;

    let sent = rig.runs_sent_to_the_session();
    assert!(
        sent.iter().any(|line| line.contains("#1 merged into")),
        "{sent:?}"
    );
}

#[tokio::test]
async fn the_watch_settles_the_row_of_a_merged_pull_request_a_terminal_session_holds() {
    let rig = a_rig();
    rig.a_terminal_session_on("t1", 1).await;
    rig.holds(Holder::session("t1"), "pr", "1", AttachmentState::Standing)
        .await;
    rig.fleet.vcs().main_ci.followed_are(Some(Vec::new()));
    rig.fleet
        .vcs()
        .now_pull_request(Some(adapter_traits::PullRequestFacts {
            standing: adapter_traits::PullRequestStanding::Merged,
            branch: "armada/1".into(),
            auto_merge: false,
            title: "Fix the reader".into(),
            url: "https://forge.invalid/armada/pull/1".into(),
            body: None,
            closes: Vec::new(),
        }));
    rig.fleet.vcs().now_under_review(adapter_traits::UnderReview {
        checks: adapter_traits::WhatTheForgeRan::AllPassed { checks: 1 },
        ..adapter_traits::UnderReview::unreadable()
    });

    rig.reads().await;

    let held = rig
        .fleet
        .store()
        .lock()
        .await
        .attachments_of(&Holder::session("t1"))
        .unwrap();
    let row = held.iter().find(|one| one.kind == "pr").expect("the row");
    assert_eq!(row.state, AttachmentState::Spent);
    assert_eq!(row.detail.get("state").map(String::as_str), Some("merged"));
}

/// A rig whose repository holds `branch`, as one with the pull request's branch on it does.
fn a_rig_holding(branch: Option<&str>) -> Rig {
    let home = TempDir::new();
    let stand_in = Arc::new(StandIn::default());
    let mut fittings =
        crate::tests::daemon::fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.noticing = crate::noticing::Noticing::every(Duration::ZERO);
    if let Some(branch) = branch {
        fittings.vcs = FakeVcs::new().with_ref_at(branch, ONE);
    }
    let fleet = Fleet::assembled(fittings).hosting_sessions_on(
        Arc::new(Shared(Arc::clone(&stand_in))),
        Duration::from_secs(600),
    );
    Rig::around(fleet, stand_in, home)
}

#[tokio::test]
async fn a_job_that_hung_is_started_again_once_without_a_word() {
    let rig = a_rig();
    rig.a_hosted_session_on(13).await;
    let mut hung = check("ci", CiState::Failed, true);
    hung.hung = true;
    rig.fleet
        .vcs()
        .main_ci
        .followed_are(Some(vec![pull(13, vec![hung])]));

    rig.reads().await;
    assert!(
        rig.runs_sent_to_the_session().is_empty(),
        "nothing is said of a hang"
    );
    let reruns = || {
        rig.fleet
            .vcs()
            .delivered()
            .iter()
            .filter(|one| matches!(one, Delivered::RerunFailed { pull_request } if pull_request == "13"))
            .count()
    };
    assert_eq!(reruns(), 1);

    rig.restarts().await;
    rig.reads().await;
    assert_eq!(reruns(), 1, "started again once for this commit");
    assert_eq!(
        rig.runs_sent_to_the_session().len(),
        1,
        "a second hang is a failure and is told"
    );
}

#[tokio::test]
async fn a_forge_that_will_not_answer_changes_nothing() {
    let rig = a_rig();
    rig.a_hosted_session_on(14).await;
    rig.fleet.vcs().main_ci.followed_are(None);
    rig.reads().await;
    assert!(rig.runs_sent_to_the_session().is_empty());
}

impl Rig {
    async fn a_session_that_worked(&self, id: &str, branch: &str, state: AttachmentState) {
        let root = self.fleet.repositories().first().unwrap().root().to_string();
        self.fleet
            .report_session(SessionReport {
                harness: adapters::HOSTED_HARNESS.into(),
                session_id: SessionId::carried(id),
                fact: SessionFact::Started {
                    cwd: root,
                    title: None,
                    origin: SessionOrigin::Terminal,
                    mod_version: None,
                },
            })
            .await
            .unwrap();
        self.holds(Holder::session(id), "branch", branch, state).await;
    }

    async fn holders_of_pull(&self, number: u64) -> Vec<String> {
        let manifest = self.manifest();
        self.fleet
            .store()
            .lock()
            .await
            .attachments_at("pr", &number.to_string(), Some(&manifest))
            .unwrap()
            .into_iter()
            .map(|row| row.holder.id)
            .collect()
    }
}

#[tokio::test]
async fn an_open_pull_request_nobody_reported_is_attached_to_the_session_that_worked_its_branch() {
    let rig = a_rig_holding(None);
    rig.a_session_that_worked("s-made-it", "armada/12", AttachmentState::Standing)
        .await;
    rig.a_session_that_worked("s-elsewhere", "armada/99", AttachmentState::Standing)
        .await;
    rig.fleet.vcs().main_ci.followed_are(Some(vec![pull(12, vec![])]));

    rig.reads().await;
    rig.reads().await;

    assert_eq!(rig.holders_of_pull(12).await, vec!["s-made-it".to_string()]);
}

#[tokio::test]
async fn a_pull_request_a_session_already_holds_is_not_attached_to_a_second() {
    let rig = a_rig_holding(None);
    rig.a_terminal_session_on("s-reported", 12).await;
    rig.a_session_that_worked("s-branch", "armada/12", AttachmentState::Standing)
        .await;
    rig.fleet.vcs().main_ci.followed_are(Some(vec![pull(12, vec![])]));

    rig.reads().await;

    assert_eq!(rig.holders_of_pull(12).await, vec!["s-reported".to_string()]);
}

#[tokio::test]
async fn a_branch_two_sessions_worked_gives_its_pull_request_to_the_one_still_on_it() {
    let rig = a_rig_holding(None);
    rig.a_session_that_worked("s-gone", "armada/12", AttachmentState::GivenBack)
        .await;
    rig.a_session_that_worked("s-here", "armada/12", AttachmentState::Standing)
        .await;
    rig.fleet.vcs().main_ci.followed_are(Some(vec![pull(12, vec![])]));

    rig.reads().await;

    assert_eq!(rig.holders_of_pull(12).await, vec!["s-here".to_string()]);
}

/// A pull request settled as merged while its own detail says it is open (a `gh pr merge
/// --auto` read as a merge, #2027) is read again and stood back up while the forge says open.
#[tokio::test]
async fn a_pull_request_settled_too_soon_is_stood_back_up_while_it_is_open() {
    let rig = a_rig();
    rig.a_terminal_session_on("t1", 1).await;
    let at = rig.fleet.now().as_str().to_string();
    rig.fleet
        .store()
        .lock()
        .await
        .attach(
            &KeptAttachment {
                holder: Holder::session("t1"),
                kind: "pr".into(),
                manifest_id: rig.manifest(),
                target: "1".into(),
                state: AttachmentState::Spent,
                detail: [("state".to_string(), "open".to_string())].into(),
                since: at.clone(),
                changed_at: at,
            },
            false,
        )
        .unwrap();
    rig.fleet.vcs().main_ci.followed_are(Some(Vec::new()));
    rig.fleet
        .vcs()
        .now_pull_request(Some(adapter_traits::PullRequestFacts {
            standing: adapter_traits::PullRequestStanding::Open,
            branch: "armada/1".into(),
            auto_merge: true,
            title: "Fix the reader".into(),
            url: "https://forge.invalid/armada/pull/1".into(),
            body: None,
            closes: Vec::new(),
        }));
    rig.fleet.vcs().now_under_review(adapter_traits::UnderReview {
        checks: adapter_traits::WhatTheForgeRan::AllPassed { checks: 1 },
        ..adapter_traits::UnderReview::unreadable()
    });

    rig.reads().await;

    let held = rig.fleet.store().lock().await.attachments_of(&Holder::session("t1")).unwrap();
    let row = held.iter().find(|one| one.kind == "pr").expect("the row");
    assert_eq!(row.state, AttachmentState::Standing);
}

/// A slot's lease names the branch worked in it, and a session that stands in the slot holds that
/// branch, so the pull request from it is the session's without anyone reporting the branch.
#[tokio::test]
async fn a_pull_request_from_the_branch_a_sessions_slot_is_leased_on_is_attached_to_it() {
    let rig = a_rig_holding(None);
    let root = rig.fleet.repositories().first().unwrap().root().to_string();
    let slot = leased(&root, 3, "docs/approve", By::Nobody);
    rig.fleet
        .report_session(SessionReport {
            harness: "a_harness".into(),
            session_id: SessionId::carried("s-in-slot"),
            fact: SessionFact::Started {
                cwd: slot,
                title: None,
                origin: SessionOrigin::Terminal,
                mod_version: None,
            },
        })
        .await
        .unwrap();
    let mut mine = pull(21, vec![]);
    mine.branch = FromOutside::verbatim("docs/approve");
    rig.fleet.vcs().main_ci.followed_are(Some(vec![mine]));

    rig.reads().await;

    assert_eq!(rig.holders_of_pull(21).await, vec!["s-in-slot".to_string()]);
}

impl Rig {
    async fn claims(&self, id: &str, number: u64) -> Result<ipc::PullRequestClaimed, api::Refusal> {
        self.fleet
            .claim_pull_request(
                None,
                ipc::ClaimPullRequest {
                    number,
                    session_id: Some(SessionId::carried(id)),
                },
            )
            .await
    }

    async fn standing_holders_of_pull(&self, number: u64) -> Vec<String> {
        let manifest = self.manifest();
        self.fleet
            .store()
            .lock()
            .await
            .attachments_at("pr", &number.to_string(), Some(&manifest))
            .unwrap()
            .into_iter()
            .filter(|row| row.state == AttachmentState::Standing)
            .map(|row| row.holder.id)
            .collect()
    }
}

#[tokio::test]
async fn a_session_claims_an_open_pull_request_nobody_holds() {
    let rig = a_rig_holding(None);
    rig.a_session_that_worked("s-claims", "armada/0", AttachmentState::GivenBack).await;
    rig.fleet.vcs().main_ci.followed_are(Some(vec![pull(30, vec![])]));

    let claimed = rig.claims("s-claims", 30).await.expect("claimed");

    assert_eq!((claimed.number, claimed.holder_id.as_str()), (30, "s-claims"));
    assert_eq!(rig.standing_holders_of_pull(30).await, vec!["s-claims".to_string()]);
}

#[tokio::test]
async fn a_pull_request_a_live_session_holds_is_refused_with_the_reason() {
    let rig = a_rig_holding(None);
    rig.a_session_that_worked("s-live", "armada/31", AttachmentState::Standing).await;
    rig.a_session_that_worked("s-other", "armada/0", AttachmentState::GivenBack).await;
    rig.fleet.vcs().main_ci.followed_are(Some(vec![pull(31, vec![])]));

    let refused = rig.claims("s-other", 31).await.expect_err("held live");

    assert!(format!("{refused:?}").contains("still working"), "{refused:?}");
    assert!(rig.standing_holders_of_pull(31).await.is_empty());
}

#[tokio::test]
async fn a_pull_request_whose_holder_has_ended_is_claimed_and_the_holder_gives_it_back() {
    let rig = a_rig_holding(None);
    rig.a_terminal_session_on("s-old", 32).await;
    rig.fleet
        .report_session(SessionReport {
            harness: adapters::HOSTED_HARNESS.into(),
            session_id: SessionId::carried("s-old"),
            fact: SessionFact::Ended { reason: "closed".into() },
        })
        .await
        .unwrap();
    rig.a_session_that_worked("s-new", "armada/0", AttachmentState::GivenBack).await;
    rig.fleet.vcs().main_ci.followed_are(Some(vec![pull(32, vec![])]));

    rig.claims("s-new", 32).await.expect("claimed");

    assert_eq!(rig.standing_holders_of_pull(32).await, vec!["s-new".to_string()]);
}

#[tokio::test]
async fn a_pull_request_that_is_not_open_is_not_claimed() {
    let rig = a_rig_holding(None);
    rig.a_session_that_worked("s-claims", "armada/0", AttachmentState::GivenBack).await;
    rig.fleet.vcs().main_ci.followed_are(Some(vec![]));
    assert!(rig.claims("s-claims", 33).await.is_err());
}

impl Rig {
    fn root(&self) -> String {
        self.fleet.repositories().first().unwrap().root().to_string()
    }

    /// A terminal Session reporting from `cwd`: its start, then the pull request it read there.
    async fn reports_from(&self, id: &str, cwd: &str, pull: Option<&str>) {
        let fact = |fact| SessionReport {
            harness: adapters::HOSTED_HARNESS.into(),
            session_id: SessionId::carried(id),
            fact,
        };
        self.fleet
            .report_session(fact(SessionFact::Started {
                cwd: cwd.into(),
                title: None,
                origin: SessionOrigin::Terminal,
                mod_version: None,
            }))
            .await
            .unwrap();
        for (kind, target) in [("branch", "armada/9"), ("pr", pull.unwrap_or(""))] {
            if target.is_empty() {
                continue;
            }
            self.fleet
                .report_session(fact(SessionFact::Attached {
                    attachment: ipc::AttachmentReport {
                        kind: kind.into(),
                        target: target.into(),
                        detail: Default::default(),
                    },
                }))
                .await
                .unwrap();
        }
    }

    async fn standing_of(&self, id: &str, kind: &str) -> Vec<(String, AttachmentState)> {
        self.fleet
            .store()
            .lock()
            .await
            .attachments_of(&Holder::session(id))
            .unwrap()
            .into_iter()
            .filter(|one| one.kind == kind)
            .map(|one| (one.target, one.state))
            .collect()
    }
}

/// #2239. Session A leased slot 13 and took pull request 9, and released it; Session B leased it
/// and worked the same branch. A's directory still reads slot 13, so what it reports from there is
/// not credited, and the notice goes to B alone.
#[tokio::test]
async fn a_notice_goes_to_the_session_the_pool_says_works_the_branch_and_not_to_one_that_left_it() {
    let rig = a_rig();
    let root = rig.root();
    let slot = leased(&root, 13, "armada/9", By::Session("sess-a"));
    rig.reports_from("sess-a", &slot, Some("9")).await;

    released(&root, 13);
    leased(&root, 13, "armada/9", By::Session("sess-b"));
    rig.reports_from("sess-b", &slot, Some("9")).await;
    // A reads the branch and pull request again from where its directory still is.
    rig.reports_from("sess-a", &format!("{slot}/crates"), Some("9")).await;

    let mut conflicting = pull(9, vec![]);
    conflicting.conflicting = true;
    rig.fleet.vcs().main_ci.followed_are(Some(vec![conflicting]));
    rig.the_mod_asks("sess-a").await;
    rig.the_mod_asks("sess-b").await;
    rig.reads().await;

    assert!(rig.told("session:sess-b", 9, "conflicts").await, "B works the branch");
    assert!(!rig.told("session:sess-a", 9, "conflicts").await, "A left it");
    assert_eq!(
        rig.standing_of("sess-a", "slot").await,
        vec![("13".to_string(), AttachmentState::GivenBack)],
        "the sweep settled what the release left"
    );
    assert_eq!(rig.standing_of("sess-a", "pr").await, vec![("9".to_string(), AttachmentState::Standing)]);
}

/// A pull request the session opened from a slot it holds is its own although its directory reads
/// another's slot, as when it leased a second slot and left the first. One it only read there is not.
#[tokio::test]
async fn a_pull_request_a_session_opened_is_credited_from_any_slot_but_one_it_only_read_is_not() {
    let rig = a_rig();
    let root = rig.root();
    let slot = leased(&root, 13, "armada/9", By::Session("sess-b"));
    rig.reports_from("sess-b", &slot, None).await;
    rig.reports_from("sess-a", &slot, None).await;
    let attach = |target: &str, opened: bool| SessionReport {
        harness: adapters::HOSTED_HARNESS.into(),
        session_id: SessionId::carried("sess-a"),
        fact: SessionFact::Attached {
            attachment: ipc::AttachmentReport {
                kind: "pr".into(),
                target: target.into(),
                detail: opened.then(|| ("created".to_string(), "true".to_string())).into_iter().collect(),
            },
        },
    };

    rig.fleet.report_session(attach("8", false)).await.unwrap();
    rig.fleet.report_session(attach("9", true)).await.unwrap();

    assert_eq!(rig.standing_of("sess-a", "pr").await, vec![("9".to_string(), AttachmentState::Standing)]);
}

/// The Session that opened a pull request is still told once nobody works its branch: the pool
/// confirms no one, so the oldest `pr` row stands, as `docs/concepts/fleet.md` has it.
#[tokio::test]
async fn a_pull_request_nobody_works_is_told_to_the_session_whose_row_is_oldest() {
    let rig = a_rig();
    let root = rig.root();
    let slot = leased(&root, 13, "armada/9", By::Session("sess-a"));
    rig.reports_from("sess-a", &slot, Some("9")).await;
    rig.a_terminal_session_on("sess-later", 9).await;
    released(&root, 13);

    let mut conflicting = pull(9, vec![]);
    conflicting.conflicting = true;
    rig.fleet.vcs().main_ci.followed_are(Some(vec![conflicting]));
    rig.the_mod_asks("sess-a").await;
    rig.the_mod_asks("sess-later").await;
    rig.reads().await;

    assert!(rig.told("session:sess-a", 9, "conflicts").await);
    assert!(!rig.told("session:sess-later", 9, "conflicts").await);
}

#[tokio::test]
async fn a_branch_that_was_only_given_back_is_nobodys_to_attach_a_pull_request_to() {
    let rig = a_rig_holding(None);
    rig.a_session_that_worked("s-gone", "armada/12", AttachmentState::GivenBack)
        .await;
    rig.fleet.vcs().main_ci.followed_are(Some(vec![pull(12, vec![])]));

    rig.reads().await;

    assert_eq!(rig.holders_of_pull(12).await, Vec::<String>::new());
}

pub(super) fn written_by(mut pull: FollowedPull, login: &str) -> FollowedPull {
    pull.author = Some(FromOutside::verbatim(login));
    pull
}

#[tokio::test]
async fn only_the_pull_requests_the_forge_login_opened_wake_their_owners() {
    let rig = a_rig();
    rig.a_hosted_session_on(7).await;
    let theirs = rig.a_hosted_session_on(8).await;
    let forge = &rig.fleet.vcs().main_ci;
    forge.login_is(Some("Octocat"));
    forge.runs_on(ONE, vec![run("test", "31", CiState::Failed)]);
    forge.log_of("31", "FAIL tests::one\n");
    forge.followed_are(Some(vec![
        written_by(failing(7), "octocat"),
        written_by(failing(8), "teammate"),
    ]));

    rig.reads().await;

    let sent = rig.runs_sent_to_the_session();
    let [line] = sent.as_slice() else {
        panic!("one turn started, for the pull request that is mine: {sent:?}")
    };
    assert!(line.contains("#7 failed its checks"), "{line}");
    assert!(!rig.told(&format!("session:{theirs}"), 8, "checks_failed").await, "nothing was kept for the other");
}

#[tokio::test]
async fn where_no_login_is_known_every_pull_request_is_told_as_before() {
    let rig = a_rig();
    rig.a_hosted_session_on(7).await;
    rig.a_hosted_session_on(8).await;
    let forge = &rig.fleet.vcs().main_ci;
    forge.login_is(None);
    forge.runs_on(ONE, vec![run("test", "31", CiState::Failed)]);
    forge.log_of("31", "FAIL tests::one\n");
    forge.followed_are(Some(vec![
        written_by(failing(7), "octocat"),
        written_by(failing(8), "teammate"),
        failing(9),
    ]));

    rig.reads().await;

    assert_eq!(rig.runs_sent_to_the_session().len(), 2, "both sessions are held, and both told");
}

#[tokio::test]
async fn a_pull_request_with_no_author_is_mine_and_one_by_another_leaves_the_queue_unremarked() {
    let rig = a_rig();
    rig.a_hosted_session_on(7).await;
    rig.a_hosted_session_on(8).await;
    let forge = &rig.fleet.vcs().main_ci;
    forge.login_is(Some("octocat"));
    let queued = |number, login: Option<&str>| {
        let mut one = pull(number, Vec::new());
        one.queue = PullQueue::Waiting;
        one.author = login.map(FromOutside::verbatim);
        one
    };
    forge.followed_are(Some(vec![queued(7, None), queued(8, Some("teammate"))]));
    rig.reads().await;
    // Both leave the queue.
    forge.followed_are(Some(vec![pull(7, Vec::new()), written_by(pull(8, Vec::new()), "teammate")]));
    rig.reads().await;

    let sent = rig.runs_sent_to_the_session();
    let [line] = sent.as_slice() else { panic!("only the one with no author is told: {sent:?}") };
    assert!(line.contains("#7 left the merge queue"), "{line}");
}

#[tokio::test]
async fn a_merge_of_another_authors_pull_request_is_not_told() {
    let rig = a_rig();
    let mine = rig.a_hosted_session_on(1).await;
    let theirs = rig.a_hosted_session_on(2).await;
    for (session, number) in [(&mine, "1"), (&theirs, "2")] {
        rig.holds(Holder::session(session), "pr", number, AttachmentState::Spent).await;
    }
    let forge = &rig.fleet.vcs().main_ci;
    forge.login_is(Some("octocat"));
    forge.followed_are(Some(Vec::new()));
    let merged = |number: u64, login: &str| RecentlyMerged {
        number,
        title: FromOutside::verbatim("Fix the reader"),
        branch: FromOutside::verbatim(format!("armada/{number}")),
        url: FromOutside::verbatim(format!("https://forge.invalid/armada/pull/{number}")),
        author: Some(FromOutside::verbatim(login)),
        merged_at: FromOutside::verbatim(rig.fleet.now().as_str()),
        commit: None,
    };
    forge.recently_merged_are(Some(vec![merged(1, "OCTOCAT"), merged(2, "teammate")]));
    let served = rig.fleet.repositories().served().remove(0);
    rig.fleet.notice_merged(&served).await;

    rig.reads().await;

    let sent = rig.runs_sent_to_the_session();
    assert!(sent.iter().any(|line| line.contains("#1 merged into")), "{sent:?}");
    assert!(!sent.iter().any(|line| line.contains("#2 merged into")), "{sent:?}");
}

/// The person turns auto-merge on: the Session's item asking for the merge is taken off, the notice
/// says so in Fleet's voice, once, and an item about something else stands.
#[tokio::test]
async fn auto_merge_settles_the_sessions_wait_on_that_pull_request_and_tells_it_once() {
    let rig = a_rig();
    let session = rig.a_hosted_session_on(7).await;
    let item = |id: &str, text: &str| WaitingInput { id: id.into(), text: text.into(), act: None, options: Vec::new(), steps: Vec::new() };
    rig.fleet
        .set_waiting(None, SetWaitingFor { items: vec![item("merge", "Merge #7 when ci is green"), item("restart", "Restart Fleet onto #70")], session_id: Some(SessionId::carried(&session)) })
        .await
        .unwrap();
    rig.fleet.vcs().main_ci.followed_are(Some(vec![FollowedPull { auto_merge: true, ..pull(7, Vec::new()) }]));

    rig.reads().await;

    let sent = rig.runs_sent_to_the_session();
    let [line] = sent.as_slice() else { panic!("one turn started: {sent:?}") };
    assert!(line.contains("Auto-merge is on for #7"), "{line}");
    assert!(line.contains("Taken off Needs you: Merge #7 when ci is green"), "{line}");
    let left: Vec<String> = Arc::clone(&rig.fleet).get_session(SessionId::carried(&session)).await.unwrap().session.waiting_for.into_iter().map(|one| one.id).collect();
    assert_eq!(left, ["restart"]);

    rig.reads().await;
    assert_eq!(rig.runs_sent_to_the_session().len(), 1, "told once");
}

/// The forge reports a pull request in the merge queue with auto-merge off: the queue is what lands
/// it, so the Session's item asking for the merge is taken off all the same.
#[tokio::test]
async fn a_pull_request_the_queue_holds_settles_the_sessions_wait_without_auto_merge() {
    let rig = a_rig();
    let session = rig.a_hosted_session_on(7).await;
    let item = |id: &str, text: &str| WaitingInput { id: id.into(), text: text.into(), act: None, options: Vec::new(), steps: Vec::new() };
    rig.fleet
        .set_waiting(None, SetWaitingFor { items: vec![item("queue", "Merge #7 once ci is green")], session_id: Some(SessionId::carried(&session)) })
        .await
        .unwrap();
    rig.fleet.vcs().main_ci.followed_are(Some(vec![FollowedPull { queue: PullQueue::Waiting, auto_merge: false, ..pull(7, Vec::new()) }]));

    rig.reads().await;

    let sent = rig.runs_sent_to_the_session();
    let [line] = sent.as_slice() else { panic!("one turn started: {sent:?}") };
    assert!(line.contains("#7 is in the merge queue"), "{line}");
    assert!(line.contains("Taken off Needs you: Merge #7 once ci is green"), "{line}");
    let left = Arc::clone(&rig.fleet).get_session(SessionId::carried(&session)).await.unwrap().session.waiting_for;
    assert!(left.is_empty(), "{left:?}");

    rig.reads().await;
    assert_eq!(rig.runs_sent_to_the_session().len(), 1, "nothing more to take off, nothing said");
}

/// The notice is kept once a pull request, so a wait stated after it was told would stand for good
/// unless every reading looks again.
#[tokio::test]
async fn a_wait_stated_after_the_queue_notice_is_taken_off_at_the_next_reading() {
    let rig = a_rig();
    let session = rig.a_hosted_session_on(7).await;
    let item = |id: &str, text: &str| WaitingInput { id: id.into(), text: text.into(), act: None, options: Vec::new(), steps: Vec::new() };
    rig.fleet.vcs().main_ci.followed_are(Some(vec![FollowedPull { auto_merge: true, ..pull(7, Vec::new()) }]));
    rig.reads().await;
    assert_eq!(rig.runs_sent_to_the_session().len(), 1, "told once");

    rig.fleet
        .set_waiting(None, SetWaitingFor { items: vec![item("merge", "Merge #7 when ci is green")], session_id: Some(SessionId::carried(&session)) })
        .await
        .unwrap();
    rig.reads().await;

    let sent = rig.runs_sent_to_the_session();
    let [_, line] = sent.as_slice() else { panic!("a second turn started: {sent:?}") };
    assert!(line.contains("Taken off Needs you: Merge #7 when ci is green"), "{line}");
    let left = Arc::clone(&rig.fleet).get_session(SessionId::carried(&session)).await.unwrap().session.waiting_for;
    assert!(left.is_empty(), "{left:?}");

    rig.reads().await;
    assert_eq!(rig.runs_sent_to_the_session().len(), 2, "nothing more to take off, nothing said");
}

/// A line Fleet read off the reply's close is the person's to answer like the agent's own, and a
/// merge that is Fleet's to wait for settles it the same.
#[tokio::test]
async fn an_ask_read_off_a_reply_naming_the_pull_request_is_taken_off_at_auto_merge() {
    let rig = a_rig();
    let session = rig.a_hosted_session_on(7).await;
    let id = SessionId::carried(&session);
    Arc::clone(&rig.fleet)
        .send_session_message(SendSessionMessage { session_id: id.clone(), text: "land it".into(), attachments: Vec::new(), mentions: Vec::new(), aim: None })
        .await
        .unwrap();
    eventually(|| async { !rig.stand_in.starts().is_empty() }).await;
    rig.stand_in.init(0);
    rig.stand_in.finishes(0, "The change is in.\n\nWill you merge #7 once ci is green?");
    let asked = || async { Arc::clone(&rig.fleet).get_session(id.clone()).await.unwrap().session.waiting_for.iter().any(|one| one.source == WaitingSource::Replied) };
    eventually(asked).await;
    let sends = rig.runs_sent_to_the_session().len();

    rig.fleet.vcs().main_ci.followed_are(Some(vec![FollowedPull { auto_merge: true, ..pull(7, Vec::new()) }]));
    rig.reads().await;

    let sent = rig.runs_sent_to_the_session();
    assert_eq!(sent.len(), sends + 1, "the notice is sent: {sent:?}");
    let line = sent.last().unwrap();
    assert!(line.contains("Taken off Needs you: Will you merge #7 once ci is green?"), "{line}");
    let left = Arc::clone(&rig.fleet).get_session(id).await.unwrap().session.waiting_for;
    assert!(left.iter().all(|one| one.source != WaitingSource::Replied), "{left:?}");
}
