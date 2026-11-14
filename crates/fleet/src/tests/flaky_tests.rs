//! A failing test that threw a pull request out of the merge queue, with two Macs on one store: a
//! pull request thrown out for a test another Mac's Job is fixing waits naming that Job and is
//! queued again once the fix merges and main is green; a test the change cannot reach is claimed
//! and gets one fix Job; one the change does reach is the Job's to answer, and is claimed only once
//! it calls the test flaky; and two Fleets racing for one red main end with one fix Job.
//! `docs/concepts/fleet.md`, *A test that fails the merge queue*.

use std::sync::Arc;

use adapter_traits::{ClaimKind, PullRequestDiff, Store};
use api::{HostedSessions, Sessions};
use core_model::{EdgeGrain, EdgeSource, TestMapEdge, Timestamp};
use ipc::{ClaimWaitEnds, SessionId, SessionRow};
use store::{AttachmentState, Holder, KeptAttachment, MainCi, MainState};
use testkit::{Delivering, FakeStore, FakeVcs};

use crate::flaky_tests::fix_title;
use crate::tests::planted::Held;
use crate::tests::red_claim::{in_a_project_storing_at, Planted, SLUG};
use crate::tests::session_host::{rig_fitted, Rig};

const TEST: &str = "two_macs::acting::a_queue_race_settles";
const REACHED: &str = "crates/fleet/src/queue.rs";
const LOG: &str = "test two_macs::acting::a_queue_race_settles ... FAILED";

/// One Mac: its Fleet on a clock the test moves, and its pull requests' changed files.
struct Mac {
    rig: Rig,
    clock: Arc<Held>,
}

async fn mac(name: &str, shared: &FakeStore, changed: &[&str]) -> Mac {
    let (view, clock) = (Arc::new(shared.view()), Arc::new(Held::started()));
    let ticking = Arc::clone(&clock);
    let files: Vec<String> = changed.iter().map(|file| file.to_string()).collect();
    let rig = rig_fitted(move |fittings| {
        let vcs = std::mem::replace(&mut fittings.vcs, FakeVcs::new());
        fittings.vcs = vcs.delivering(Delivering { pull_request_diff: Some(PullRequestDiff { files, patch: String::new() }), ..Delivering::default() });
        fittings.reaching = Arc::new(Planted::with_store(view));
        fittings.clock = ticking;
    });
    crate::profiles::Profiles::in_machine_directory(rig.home())
        .join(&adapter_traits::SignedInAs { id: 583_231, login: "octocat".into() }, crate::profiles::Orgs::Read(vec!["mele-labs".into()]), "2026-10-14T09:00:00.000Z")
        .expect("a profile");
    let changes = [("machine.name".to_string(), Some(ipc::SettingValue::Text(name.to_string())))].into_iter().collect();
    api::Settings::save_settings(&*rig.fleet, ipc::SaveSettings { changes }).await.expect("saved");
    in_a_project_storing_at(&rig.fleet, "https://store.test").await;
    let edge = TestMapEdge {
        repository: rig.fleet.repositories().first().expect("served").manifest().id().clone(),
        check: "ci".into(),
        member: String::new(),
        test: TEST.into(),
        file: REACHED.into(),
        source: EdgeSource::Measured,
        grain: EdgeGrain::Test,
        commit: "a".repeat(40),
        at: Timestamp::from_rfc3339("2026-11-13T00:00:00Z".to_string()),
        stale: false,
    };
    rig.fleet.store().lock().await.record_test_map_edges(&[edge]).expect("kept");
    Mac { rig, clock }
}

impl Mac {
    /// A Job whose pull request `number` is open.
    async fn job_on(&self, title: &str, number: u64) -> SessionId {
        let job = self.rig.start_titled(Some(title)).await;
        self.pull(&job, number, AttachmentState::Standing).await;
        job
    }

    async fn pull(&self, job: &SessionId, number: u64, state: AttachmentState) {
        let at = self.rig.fleet.now().as_str().to_string();
        let row = KeptAttachment {
            holder: Holder::session(job.as_str()),
            kind: crate::pull_requesting::PR.into(),
            manifest_id: self.rig.manifest.as_str().to_string(),
            target: number.to_string(),
            state,
            detail: Default::default(),
            since: at.clone(),
            changed_at: at,
        };
        self.rig.fleet.store().lock().await.attach(&row, false).expect("kept");
    }

    /// The merge queue threw `number` out, `job`'s, for the test: what Fleet tells the Job.
    async fn thrown_out(&self, job: &SessionId, number: u64) -> String {
        let served = self.rig.fleet.repositories().served().remove(0);
        self.rig.fleet.ejection_judged(&served, number, &[job.as_str().to_string()], &[TEST.to_string()], LOG).await
    }

    /// A pass, `seconds` after the last.
    async fn pass(&self, seconds: u64) {
        self.clock.on(seconds);
        self.rig.fleet.claims_passed().await;
    }

    async fn main_reads(&self, state: MainState, commit: &str) {
        let at = self.rig.fleet.now();
        let red = (state == MainState::Red).then(|| at.clone());
        let main = MainCi {
            repository: self.rig.root.clone(),
            base: "main".into(),
            commit: commit.into(),
            state,
            read_at: at,
            red_at: red.clone(),
            red_commit: red.as_ref().map(|_| commit.to_string()),
            decided_commit: Some(commit.into()),
            newer_running: 0,
            unfinished: 0,
            failed: match red {
                Some(_) => vec![store::MainFailedJob { name: "ci".into(), check: None, log_url: None, tests: vec!["reader::stops".into()] }],
                None => Vec::new(),
            },
            merge: None,
        };
        self.rig.fleet.store().lock().await.record_main_ci(&main).expect("kept");
    }

    /// The live Jobs titled `title` on this Mac.
    async fn live_titled(&self, title: &str) -> Vec<String> {
        let store = self.rig.fleet.store().lock().await;
        let found = store.find_sessions(&store::SessionSearch { manifest_id: None, state: None, text: None }).expect("found");
        found.into_iter().filter(|one| one.title.as_deref() == Some(title) && one.ended_at.is_none()).map(|one| one.id).collect()
    }

    async fn told(&self, job: &SessionId) -> Vec<String> {
        let rows = Arc::clone(&self.rig.fleet).get_session(job.clone()).await.expect("read").rows;
        rows.into_iter()
            .filter_map(|row| match row {
                SessionRow::Message { text, notice: Some(notice), .. } if notice == "claims" => Some(text),
                _ => None,
            })
            .collect()
    }
}

fn holder_of(store: &FakeStore, kind: ClaimKind, named: &str) -> Option<String> {
    let repository = crate::red_claim::repository_key_of(SLUG).expect("a key");
    let key = crate::job_claims::key_of(&crate::job_claims::Wanted { kind, named: named.to_string() }, SLUG).expect("a key");
    store.read_claim(&repository, kind, &key).expect("read").map(|held| held.holder.id)
}

#[tokio::test]
async fn a_pull_request_thrown_out_for_a_test_another_mac_fixes_waits_naming_it_and_is_queued_again_when_the_fix_merges() {
    let shared = FakeStore::new();
    let (home, work) = (mac("home", &shared, &["docs/notes.md"]).await, mac("work", &shared, &[REACHED]).await);
    let first = home.job_on("Write the queue notes", 7).await;

    let said = home.thrown_out(&first, 7).await;

    let fixers = home.live_titled(&fix_title(TEST)).await;
    let [fixer] = fixers.as_slice() else { panic!("one fix Job: {fixers:?}") };
    assert_eq!(holder_of(&shared, ClaimKind::Test, TEST).as_deref(), Some(fixer.as_str()), "claimed before it was briefed");
    assert!(said.contains("#7's changes do not reach it") && said.contains("Fleet started Job"), "{said}");

    let second = work.job_on("Land the queue race fix", 12).await;
    let said = work.thrown_out(&second, 12).await;
    assert!(said.contains("is being fixed by Job \"Fix the flaky test a_queue_race_settles\" on octocat's Mac home"), "{said}");
    assert!(work.live_titled(&fix_title(TEST)).await.is_empty(), "the second Mac starts nothing");

    home.pass(1).await;
    work.pass(1).await;
    let waits = work.rig.fleet.list_sessions(None, Some(second.as_str().into()), None).await.expect("listed").sessions.remove(0).claim_waits;
    let [wait] = waits.as_slice() else { panic!("one wait: {waits:?}") };
    assert_eq!((wait.item.as_str(), wait.machine.as_deref()), (TEST, Some("home")));
    assert_eq!((wait.title.as_deref(), wait.ends), (Some("Fix the flaky test a_queue_race_settles"), ClaimWaitEnds::FixMerges));
    assert_eq!(work.rig.fleet.vcs().times_asked_for_auto_merge(), 0, "nothing queues it while it waits");

    home.clock.on(60);
    home.pull(&SessionId::carried(fixer.as_str()), 40, AttachmentState::Spent).await;
    home.pass(1).await;
    assert_eq!(holder_of(&shared, ClaimKind::Test, TEST).as_deref(), Some(fixer.as_str()), "a merged fix holds until main is green");
    home.clock.on(60);
    home.main_reads(MainState::Green, &"c".repeat(40)).await;
    home.pass(1).await;
    assert_eq!(holder_of(&shared, ClaimKind::Test, TEST), None, "the fix merged and main is green");

    work.rig.fleet.vcs().now_pull_request(Some(adapter_traits::PullRequestFacts {
        standing: adapter_traits::PullRequestStanding::Open,
        branch: "armada/12".into(),
        base: "main".into(),
        auto_merge: false,
        title: String::new(),
        url: "https://forge.invalid/armada/pull/12".into(),
        body: None,
        closes: Vec::new(),
    }));
    work.rig.fleet.vcs().now_under_review(adapter_traits::UnderReview {
        checks: adapter_traits::WhatTheForgeRan::AllPassed { checks: 1 },
        ..adapter_traits::UnderReview::unreadable()
    });
    work.pass(300).await;
    assert_eq!(work.rig.fleet.vcs().times_asked_for_auto_merge(), 1, "queued again");
    let told = work.told(&second).await;
    assert_eq!(told.last().map(String::as_str), Some(format!("The fix for {TEST} merged. Fleet queued #12 again.").as_str()), "{told:?}");
    let record = work.rig.fleet.list_sessions(None, Some(second.as_str().into()), None).await.expect("listed").sessions.remove(0);
    assert!(record.claim_waits.is_empty(), "the wait ends");
}

#[tokio::test]
async fn a_test_the_change_reaches_is_the_jobs_to_answer_and_is_claimed_only_once_it_calls_it_flaky() {
    let shared = FakeStore::new();
    let home = mac("home", &shared, &[REACHED]).await;
    let job = home.job_on("Speed up the queue", 9).await;

    let said = home.thrown_out(&job, 9).await;

    assert!(said.starts_with(&format!("Your change reaches {TEST}.")), "{said}");
    assert_eq!(holder_of(&shared, ClaimKind::Test, TEST), None, "nothing is claimed before the Job answers");
    assert!(home.live_titled(&fix_title(TEST)).await.is_empty());

    let other = home.job_on("Unrelated", 10).await;
    let call = |id: &SessionId, line: &str| ipc::CallTestFlaky { session_id: Some(id.clone()), test: TEST.into(), log_line: line.into() };
    assert!(Arc::clone(&home.rig.fleet).test_called_flaky(None, call(&other, LOG)).await.is_err(), "Fleet did not ask that Job");
    assert!(Arc::clone(&home.rig.fleet).test_called_flaky(None, call(&job, " ")).await.is_err(), "a flake is called with its log line");
    assert_eq!(holder_of(&shared, ClaimKind::Test, TEST), None);

    let answered = Arc::clone(&home.rig.fleet).test_called_flaky(None, call(&job, LOG)).await.expect("called");

    let fixers = home.live_titled(&fix_title(TEST)).await;
    let [fixer] = fixers.as_slice() else { panic!("one fix Job: {fixers:?}") };
    assert_eq!(holder_of(&shared, ClaimKind::Test, TEST).as_deref(), Some(fixer.as_str()));
    assert_eq!(answered.pull, 9);
    assert!(answered.said.contains("the Job that owns #9 says so"), "{}", answered.said);
    assert!(Arc::clone(&home.rig.fleet).test_called_flaky(None, call(&job, LOG)).await.is_err(), "answered once");
}

#[tokio::test]
async fn two_fleets_racing_for_one_red_main_end_with_one_fix_job() {
    let shared = FakeStore::new();
    let (home, work) = (mac("home", &shared, &[]).await, mac("work", &shared, &[]).await);
    let red = "b".repeat(40);
    home.main_reads(MainState::Red, &red).await;
    work.main_reads(MainState::Red, &red).await;

    let (here, there) = tokio::join!(home.rig.fleet.reds_fixed(), work.rig.fleet.reds_fixed());

    assert_eq!(here + there, 1, "one Fleet started the fix");
    let (home_fixes, work_fixes) = (home.live_titled("Fix main").await, work.live_titled("Fix main").await);
    let fixers: Vec<&String> = home_fixes.iter().chain(&work_fixes).collect();
    let [fixer] = fixers.as_slice() else { panic!("one live fix Job: {fixers:?}") };
    let repository = crate::red_claim::repository_key_of(SLUG).expect("a key");
    let (_, key) = crate::red_claim::keys_of(SLUG, &red).expect("keys");
    let held = shared.read_claim(&repository, ClaimKind::Red, &key).expect("read").expect("claimed");
    assert_eq!(&held.holder.id, *fixer);
    assert_eq!(home.rig.fleet.reds_fixed().await + work.rig.fleet.reds_fixed().await, 0, "nobody starts a second");
}
