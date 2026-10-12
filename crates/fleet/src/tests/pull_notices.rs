//! What Fleet tells the owner of a pull request, against a scripted forge. The forge's words are
//! `adapters`' to assert; under test here is who is told what, when, and how many times: a hosted
//! Session, a terminal one and a Job, a hung job started again without a word, and a restart that
//! tells nobody twice.

use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{CiState, FromOutside, PullQueue, RecentlyMerged, WatchedCheck, WatchedPull};
use api::{HostedSessions, Sessions};
use core_model::{JobId, JobStatus};
use ipc::{
    ManifestId, SessionFact, SessionId, SessionOrigin, SessionReport, SessionRow, SessionVoice,
    StartSession, TakeHeld,
};
use store::{AttachmentState, Holder, KeptAttachment};
use testkit::{Delivered, FakeHarness, FakeVcs, FakeWorkProduct};

use super::main_ci::{run, ONE};
use super::main_fix::{a_finished_job, a_fleet_holding_bug};
use super::reviewing::at_the_gate;
use super::session_host::{Shared, StandIn};
use crate::daemon::Fleet;
use crate::tests::daemon::a_fleet_gated_on_a_person;
use crate::tests::tmp::TempDir;

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const LOG_URL: &str = "https://forge.invalid/armada/actions/runs/9/job/31";

fn check(name: &str, state: CiState, required: bool) -> WatchedCheck {
    WatchedCheck {
        name: FromOutside::verbatim(name),
        state,
        required,
        hung: false,
        log_url: Some(FromOutside::verbatim(LOG_URL)),
    }
}

fn pull(number: u64, checks: Vec<WatchedCheck>) -> WatchedPull {
    WatchedPull {
        number,
        branch: FromOutside::verbatim(format!("armada/{number}")),
        url: FromOutside::verbatim(format!("https://forge.invalid/armada/pull/{number}")),
        head: Some(ONE.to_string()),
        conflicting: false,
        queue: PullQueue::Outside,
        checks,
    }
}

fn failing(number: u64) -> WatchedPull {
    pull(
        number,
        vec![
            check("test", CiState::Failed, false),
            check("ci", CiState::Failed, true),
        ],
    )
}

struct Rig {
    fleet: Arc<Hosted>,
    stand_in: Arc<StandIn>,
    _home: TempDir,
}

impl Rig {
    fn around(fleet: Hosted, stand_in: Arc<StandIn>, home: TempDir) -> Rig {
        Rig {
            fleet: Arc::new(fleet),
            stand_in,
            _home: home,
        }
    }

    fn manifest(&self) -> String {
        self.fleet
            .repositories()
            .first()
            .expect("served")
            .manifest()
            .id()
            .as_str()
            .to_string()
    }

    async fn reads(&self) {
        let served = self.fleet.repositories().served().remove(0);
        self.fleet.pulls_told(&served).await;
    }

    /// Fleet restarted: nothing it kept in memory survives, the store does.
    async fn restarts(&self) {
        *self.fleet.sweeping().lock().await = Default::default();
    }

    async fn holds(&self, holder: Holder, kind: &str, target: &str, state: AttachmentState) {
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

    async fn a_hosted_session_on(&self, number: u64) -> String {
        let id = self
            .fleet
            .start_session(StartSession {
                manifest_id: ManifestId::carried(self.manifest()),
                title: None,
                model: None,
                effort: None,
                mode: None,
                pilot: None,
                fork: None,
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

    async fn told(&self, recipient: &str, number: u64, cause: &str) -> bool {
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

    fn runs_sent_to_the_session(&self) -> Vec<String> {
        let started = self.stand_in.starts().len();
        (0..started)
            .flat_map(|at| self.stand_in.process(at).sent.lock().unwrap().clone())
            .collect()
    }
}

fn a_rig() -> Rig {
    let home = TempDir::new();
    let stand_in = Arc::new(StandIn::default());
    let fleet = a_fleet_holding_bug(&home, false).hosting_sessions_on(
        Arc::new(Shared(Arc::clone(&stand_in))),
        Duration::from_secs(600),
    );
    Rig::around(fleet, stand_in, home)
}

fn a_rig_at_a_gate() -> (Rig, TempDir) {
    let home = TempDir::new();
    let stand_in = Arc::new(StandIn::default());
    let fleet = a_fleet_gated_on_a_person(
        &home,
        FakeWorkProduct::changed(&["src/log.rs"]),
        "implement",
        FakeVcs::new(),
    )
    .hosting_sessions_on(
        Arc::new(Shared(Arc::clone(&stand_in))),
        Duration::from_secs(600),
    );
    (Rig::around(fleet, stand_in, TempDir::new()), home)
}

async fn owns_pull(rig: &Rig, job: &JobId, number: u64) {
    rig.fleet
        .store()
        .lock()
        .await
        .record_delivery(
            job,
            &store::Delivery {
                commit: None,
                pushed: None,
                pull_request: Some(format!("https://forge.invalid/armada/pull/{number}")),
                landed: None,
                unpushed: None,
            },
        )
        .unwrap();
}

#[tokio::test]
async fn a_failed_required_check_wakes_the_hosted_session_that_holds_the_pull_request() {
    let rig = a_rig();
    let session = rig.a_hosted_session_on(7).await;
    let forge = &rig.fleet.vcs().main_ci;
    forge.runs_on(ONE, vec![run("test", "31", CiState::Failed)]);
    forge.log_of("31", "one\nFAIL tests::one\nerror: test run failed\n");
    forge.watched_are(Some(vec![failing(7)]));

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
    rig.fleet.vcs().main_ci.watched_are(Some(vec![pull(
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
    rig.fleet.vcs().main_ci.watched_are(Some(vec![conflicting]));

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
    rig.fleet.vcs().main_ci.watched_are(Some(vec![conflicting]));

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
    rig.fleet.vcs().main_ci.watched_are(Some(vec![queued]));
    rig.reads().await;
    assert!(
        rig.runs_sent_to_the_session().is_empty(),
        "waiting in the queue is nothing to say"
    );

    rig.fleet
        .vcs()
        .main_ci
        .watched_are(Some(vec![pull(10, vec![])]));
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
    rig.fleet.vcs().main_ci.watched_are(Some(vec![stuck]));
    rig.reads().await;
    assert!(rig
        .runs_sent_to_the_session()
        .iter()
        .any(|line| line.contains("#11 is unmergeable")));
}

#[tokio::test]
async fn a_merge_is_told_to_the_session_that_held_it_and_noted_on_the_job() {
    let rig = a_rig();
    let finished = a_finished_job(&rig.fleet, &rig._home, "fix the reader").await;
    let session = rig.a_hosted_session_on(1).await;
    rig.holds(
        Holder::session(&session),
        "pr",
        "1",
        AttachmentState::Spent,
    )
    .await;
    rig.fleet.vcs().main_ci.watched_are(Some(Vec::new()));
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
    assert!(
        rig.told(&format!("job:{}", finished.id().as_str()), 1, "merged")
            .await
    );
    let job = rig.fleet.load(finished.id()).await.unwrap();
    assert_eq!(
        job.status(),
        JobStatus::CompletedSuccess,
        "a merge sends nothing back"
    );
    let (loaded, _) = rig.fleet.every_job().await.unwrap();
    assert_eq!(loaded.jobs.len(), 1, "and starts nothing");
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
    rig.fleet.vcs().main_ci.watched_are(Some(Vec::new()));
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
    rig.fleet.vcs().main_ci.watched_are(Some(Vec::new()));
    rig.fleet
        .vcs()
        .now_pull_request(Some(adapter_traits::PullRequestFacts {
            standing: adapter_traits::PullRequestStanding::Merged,
            branch: "armada/1".into(),
            auto_merge: false,
            title: "Fix the reader".into(),
            url: "https://forge.invalid/armada/pull/1".into(),
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

#[tokio::test]
async fn a_job_at_its_review_gate_is_sent_back_with_the_failure_and_a_session_is_told_too() {
    let (rig, home) = a_rig_at_a_gate();
    let job_id = at_the_gate(&rig.fleet, &home).await;
    owns_pull(&rig, &job_id, 12).await;
    let session = rig.a_hosted_session_on(12).await;
    rig.fleet.vcs().main_ci.watched_are(Some(vec![failing(12)]));

    rig.reads().await;

    let job = rig.fleet.load(&job_id).await.unwrap();
    assert_eq!(
        job.status(),
        JobStatus::Queued,
        "the work went back, as a requested change does"
    );
    let note = job
        .redirect_waiting()
        .expect("the failure waits for the next Drone");
    assert!(
        format!("{note:?}").contains("#12 failed its checks"),
        "{note:?}"
    );
    assert!(
        rig.runs_sent_to_the_session()
            .iter()
            .any(|line| line.contains("#12 failed")),
        "{session}"
    );

    rig.restarts().await;
    rig.reads().await;
    assert_eq!(rig.runs_sent_to_the_session().len(), 1);
    assert!(
        rig.told(&format!("job:{}", job_id.as_str()), 12, "checks_failed")
            .await
    );
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

async fn redispatched_for_a_failing_pull_request(
    rig: &Rig,
) -> (core_model::Job, Option<core_model::Landing>) {
    let earlier = a_finished_job(&rig.fleet, &rig._home, "fold the routes").await;
    rig.fleet.vcs().main_ci.watched_are(Some(vec![failing(1)]));

    rig.reads().await;
    rig.reads().await;
    rig.restarts().await;
    rig.reads().await;

    let (loaded, _) = rig.fleet.every_job().await.unwrap();
    assert_eq!(loaded.jobs.len(), 2, "one carried-on Job and no more");
    let carried = loaded
        .jobs
        .iter()
        .find(|job| job.id() != earlier.id())
        .unwrap()
        .clone();
    assert_eq!(carried.redispatched_from(), Some(earlier.id()));
    let landing = rig
        .fleet
        .store()
        .lock()
        .await
        .landing(carried.id())
        .unwrap();
    (carried, landing)
}

#[tokio::test]
async fn a_job_that_ended_is_redispatched_once_onto_its_pull_requests_branch() {
    let rig = a_rig_holding(Some("armada/1"));
    let (_, landing) = redispatched_for_a_failing_pull_request(&rig).await;
    let landing = landing.expect("a landing was kept");
    assert_eq!(
        landing.from_ref.as_ref().map(|it| it.as_str()),
        Some("armada/1")
    );
    assert_eq!(
        landing.target.as_ref().map(|it| it.as_str()),
        Some("armada/1")
    );
}

#[tokio::test]
async fn a_pull_request_whose_branch_is_gone_is_redispatched_on_a_fresh_branch() {
    let rig = a_rig_holding(None);
    let (_, landing) = redispatched_for_a_failing_pull_request(&rig).await;
    assert!(landing.is_none(), "cut from the base, as a red main is");
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
        .watched_are(Some(vec![pull(13, vec![hung])]));

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
    rig.fleet.vcs().main_ci.watched_are(None);
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
    rig.fleet.vcs().main_ci.watched_are(Some(vec![pull(12, vec![])]));

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
    rig.fleet.vcs().main_ci.watched_are(Some(vec![pull(12, vec![])]));

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
    rig.fleet.vcs().main_ci.watched_are(Some(vec![pull(12, vec![])]));

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
    rig.fleet.vcs().main_ci.watched_are(Some(Vec::new()));
    rig.fleet
        .vcs()
        .now_pull_request(Some(adapter_traits::PullRequestFacts {
            standing: adapter_traits::PullRequestStanding::Open,
            branch: "armada/1".into(),
            auto_merge: true,
            title: "Fix the reader".into(),
            url: "https://forge.invalid/armada/pull/1".into(),
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
    let slot = adapter_traits::slot_path(&root, 3);
    std::fs::create_dir_all(&slot).unwrap();
    std::fs::write(
        format!("{slot}.lease"),
        "branch docs/approve\nholder job somebody\nsince 1\n",
    )
    .unwrap();
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
    rig.fleet.vcs().main_ci.watched_are(Some(vec![mine]));

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
                    job_id: None,
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
    rig.fleet.vcs().main_ci.watched_are(Some(vec![pull(30, vec![])]));

    let claimed = rig.claims("s-claims", 30).await.expect("claimed");

    assert_eq!((claimed.number, claimed.holder_id.as_str()), (30, "s-claims"));
    assert_eq!(rig.standing_holders_of_pull(30).await, vec!["s-claims".to_string()]);
}

#[tokio::test]
async fn a_pull_request_a_live_session_holds_is_refused_with_the_reason() {
    let rig = a_rig_holding(None);
    rig.a_session_that_worked("s-live", "armada/31", AttachmentState::Standing).await;
    rig.a_session_that_worked("s-other", "armada/0", AttachmentState::GivenBack).await;
    rig.fleet.vcs().main_ci.watched_are(Some(vec![pull(31, vec![])]));

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
    rig.fleet.vcs().main_ci.watched_are(Some(vec![pull(32, vec![])]));

    rig.claims("s-new", 32).await.expect("claimed");

    assert_eq!(rig.standing_holders_of_pull(32).await, vec!["s-new".to_string()]);
}

#[tokio::test]
async fn a_pull_request_that_is_not_open_is_not_claimed() {
    let rig = a_rig_holding(None);
    rig.a_session_that_worked("s-claims", "armada/0", AttachmentState::GivenBack).await;
    rig.fleet.vcs().main_ci.watched_are(Some(vec![]));
    assert!(rig.claims("s-claims", 33).await.is_err());
}

#[tokio::test]
async fn a_person_attaches_an_open_pull_request_to_a_job_they_picked() {
    let rig = a_rig_holding(None);
    let job = a_finished_job(&rig.fleet, &rig._home, "fold the routes").await;
    rig.fleet.vcs().main_ci.watched_are(Some(vec![pull(40, vec![])]));

    let claimed = rig
        .fleet
        .claim_pull_request(
            None,
            ipc::ClaimPullRequest {
                number: 40,
                session_id: None,
                job_id: Some(ipc::JobId::from(job.id())),
            },
        )
        .await
        .expect("attached");

    assert_eq!(
        (claimed.holder_kind.as_str(), claimed.holder_id.as_str()),
        ("job", job.id().as_str())
    );
    assert_eq!(
        rig.standing_holders_of_pull(40).await,
        vec![job.id().as_str().to_string()]
    );
}

#[tokio::test]
async fn a_claim_naming_both_a_session_and_a_job_is_refused() {
    let rig = a_rig_holding(None);
    let job = a_finished_job(&rig.fleet, &rig._home, "fold the routes").await;
    rig.a_session_that_worked("s-both", "armada/0", AttachmentState::GivenBack).await;
    rig.fleet.vcs().main_ci.watched_are(Some(vec![pull(41, vec![])]));

    let refused = rig
        .fleet
        .claim_pull_request(
            None,
            ipc::ClaimPullRequest {
                number: 41,
                session_id: Some(SessionId::carried("s-both")),
                job_id: Some(ipc::JobId::from(job.id())),
            },
        )
        .await;

    assert!(refused.is_err());
    assert!(rig.standing_holders_of_pull(41).await.is_empty());
}
