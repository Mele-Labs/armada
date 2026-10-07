//! A red main handed to a Job: by a person's press, and by Fleet itself to the
//! Job whose pull request turned it red. The forge is scripted, as in
//! `main_ci`; the Jobs are the fixture's.

use std::time::Duration;

use adapter_traits::{CiState, FromOutside, RecentlyMerged};
use core_model::{Job, JobStatus};
use store::TakenHow;
use testkit::FakeWorkProduct;

use super::main_ci::{kept, merged, run, the_hub, Fixture, MANIFEST, ONE, TWO};
use crate::daemon::Fleet;
use crate::noticing::Noticing;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_proposal, diff_evidence, fittings, manifest, note_evidence, worktree_directory,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

const LOG: &str = "\
     Summary [   0.021s] 2 tests run: 0 passed, 1 failed, 38 skipped
        FAIL [   0.016s] (1/1) fleet tests::one
error: test run failed
";

/// The repository holds `bug`, the workflow a dispatched fix runs under.
fn bug() -> config::ResolvedWorkflow {
    let text = "version: 1\nworkflow_id: bug\nname: Bug\nsteps:\n  - id: fix\n    label: \"Fix it\"\n    \
                evidence: {submitted: {type: diff}}\n    mechanical_checks:\n      - { type: diff_nonempty }\n    \
                delivers: true\n    advance_gate: auto\n";
    let def = config::WorkflowDef::parse(
        std::path::Path::new("bug.yml"),
        text,
        &config::Roster::offering_nothing(),
    )
    .expect("the bug workflow parses");
    config::ResolvedWorkflow::resolve(&def, &manifest()).expect("it resolves")
}

fn a_fleet_holding_bug(home: &TempDir, with_bug: bool) -> Fixture {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.noticing = Noticing::every(Duration::ZERO);
    fittings.starting().manifest =
        config::Manifest::parse(std::path::Path::new("armada.yml"), MANIFEST).expect("parses");
    if with_bug {
        let mut held = fittings.starting().workflows.clone();
        held.insert(bug().id().clone(), bug());
        fittings.starting().workflows = held;
    }
    Fleet::assembled(fittings)
}

/// A Job that ran and delivered: the fake's pull request for it is number 1.
async fn a_finished_job(fleet: &Fixture, home: &TempDir, title: &str) -> Job {
    let job = fleet.propose(a_proposal(title)).await.unwrap();
    worktree_directory(home, &job);
    dispatched(fleet, job.id()).await.unwrap();
    submitted_by_the_one(fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    submitted_by_the_one(fleet, note_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    let job = fleet.load(job.id()).await.unwrap();
    assert_eq!(job.status(), JobStatus::CompletedSuccess);
    job
}

fn goes_red(fleet: &Fixture, pull: u64) {
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(ONE, vec![run("ci", "11", CiState::Failed)]);
    forge.log_of("11", LOG);
    forge.merged_by(ONE, merged(pull));
}

async fn root(fleet: &Fixture) -> String {
    fleet.repositories().served()[0].root().to_string()
}

async fn takes(fleet: &Fixture) -> Vec<store::MainFix> {
    let main = kept(fleet).await.unwrap();
    let red_at = main.red_at.expect("a red");
    fleet
        .store()
        .lock()
        .await
        .main_fixes_of(&main.repository, &red_at)
        .unwrap()
}

#[tokio::test]
async fn the_job_whose_pull_request_turned_main_red_is_sent_back_by_itself_once() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_bug(&home, true);
    let culprit = a_finished_job(&fleet, &home, "fix the off-by-one in the log reader").await;
    goes_red(&fleet, 1);

    fleet.turn().await.unwrap();
    let [take] = takes(&fleet).await.try_into().expect("one take");
    assert_eq!(take.how, TakenHow::Took);
    assert_eq!(take.check, "test");
    assert_eq!(take.test.as_deref(), Some("tests::one"));
    let fixing = fleet.load(&take.job).await.unwrap();
    assert_eq!(fixing.redispatched_from(), Some(culprit.id()));
    assert!(
        matches!(fixing.status(), JobStatus::Queued | JobStatus::Running),
        "released without a prompt, got {:?}",
        fixing.status()
    );
    assert!(
        fixing.facts().as_str().contains("test fails on main.")
            && fixing.facts().as_str().contains("Test: tests::one")
            && fixing.facts().as_str().contains("Merged in #1"),
        "{}",
        fixing.facts().as_str()
    );
    assert!(fixing.facts().as_str().contains("tests::nested") == false);
    assert!(
        fixing.facts().as_str().contains("FAIL ["),
        "the log's tail goes with it"
    );

    let hub = the_hub(&fleet).await;
    assert_eq!(
        hub.fixing.map(|job| job.id),
        Some(ipc::JobId::from(&take.job))
    );
    let summary = fleet.summarised(&fixing).await.unwrap();
    let mark = summary.fixes_main.expect("the Job carries its part");
    assert_eq!(mark.state, ipc::FixesMainState::Fixing);
    assert_eq!(mark.merge, Some(1));

    // The same red read again, and the culprit asked again: nothing new.
    fleet.turn().await.unwrap();
    fleet.turn().await.unwrap();
    assert_eq!(takes(&fleet).await.len(), 1, "once per red per Job");
    let (loaded, _) = fleet.every_job().await.unwrap();
    assert_eq!(loaded.jobs.len(), 2, "one carried-on Job and no more");
}

#[tokio::test]
async fn a_red_from_nobodys_job_is_left_for_the_bands_buttons() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_bug(&home, true);
    goes_red(&fleet, 1815);
    fleet.turn().await.unwrap();
    assert!(kept(&fleet).await.unwrap().red_at.is_some());
    let main = kept(&fleet).await.unwrap();
    assert!(fleet.taking(&main).await.is_none());
    assert!(the_hub(&fleet).await.fixing.is_none());
    let (loaded, _) = fleet.every_job().await.unwrap();
    assert!(loaded.jobs.is_empty());
}

#[tokio::test]
async fn a_person_dispatches_a_new_job_that_claims_the_failing_test() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_bug(&home, true);
    goes_red(&fleet, 1815);
    fleet.turn().await.unwrap();

    let brief = "test fails on main.\nTest: tests::one\nMerged in #1815 (armada/cache).";
    let job = fleet
        .fix_main(&root(&fleet).await, None, Some(brief))
        .await
        .unwrap();
    assert_eq!(job.status(), JobStatus::Queued);
    assert_eq!(job.title().as_str(), "Fix test on main");
    assert!(job.facts().as_str().starts_with(brief));
    assert!(job.facts().as_str().contains("Log: https://forge.invalid"));

    let main = kept(&fleet).await.unwrap();
    let [take] = takes(&fleet).await.try_into().expect("one take");
    assert_eq!((take.how, &take.job), (TakenHow::Dispatched, job.id()));
    let owner = core_model::ManifestId::carried(core_model::Ulid::carried(
        fleet.names().owner_of(job.id()).expect("owned"),
    ));
    let claim = fleet
        .store()
        .lock()
        .await
        .breakage_claimed(&owner, "test", "tests::one")
        .unwrap()
        .expect("the test is claimed for the fix");
    assert_eq!(&claim.fix, job.id());
    assert_eq!(
        the_hub(&fleet).await.fixing.unwrap().title,
        "Fix test on main"
    );

    let again = fleet.fix_main(&main.repository, None, None).await;
    assert!(again.unwrap_err().to_string().contains("already working"));
}

#[tokio::test]
async fn a_ci_job_with_no_check_is_dispatched_with_no_claim() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_bug(&home, true);
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(ONE, vec![run("test-all", "21", CiState::Failed)]);
    forge.log_of("21", "error: broke\n");
    fleet.turn().await.unwrap();
    let job = fleet
        .fix_main(&root(&fleet).await, None, None)
        .await
        .unwrap();
    assert_eq!(job.title().as_str(), "Fix test-all on main");
    let owner = core_model::ManifestId::carried(core_model::Ulid::carried(
        fleet.names().owner_of(job.id()).expect("owned"),
    ));
    let claims = fleet
        .store()
        .lock()
        .await
        .breakages_claimed_by(job.id())
        .unwrap();
    assert!(claims.is_empty(), "no Check, no claim: {owner:?}");
}

#[tokio::test]
async fn work_sent_back_to_a_job_that_ended_continues_it_on_a_fresh_branch() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_bug(&home, true);
    let earlier = a_finished_job(&fleet, &home, "fold the notification routes").await;
    goes_red(&fleet, 1815);
    fleet.turn().await.unwrap();

    let job = fleet
        .fix_main(&root(&fleet).await, Some(earlier.id()), None)
        .await
        .unwrap();
    assert_ne!(job.id(), earlier.id());
    assert_eq!(job.redispatched_from(), Some(earlier.id()));
    assert_eq!(job.title(), earlier.title());
    assert_eq!(job.status(), JobStatus::Queued);
    let [take] = takes(&fleet).await.try_into().expect("one take");
    assert_eq!((take.how, &take.job), (TakenHow::SentBack, job.id()));
}

#[tokio::test]
async fn work_cannot_be_sent_back_to_a_job_still_working_or_where_main_is_not_red() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_bug(&home, true);
    let working = fleet.propose(a_proposal("still at it")).await.unwrap();
    let green = fleet.fix_main(&root(&fleet).await, None, None).await;
    assert!(green.unwrap_err().to_string().contains("main is not red"));

    goes_red(&fleet, 1815);
    fleet.turn().await.unwrap();
    let refused = fleet
        .fix_main(&root(&fleet).await, Some(working.id()), None)
        .await
        .unwrap_err();
    assert!(
        refused.to_string().contains("awaiting_approval"),
        "{refused}"
    );
    assert!(takes(&fleet).await.is_empty(), "nothing was recorded");
}

#[tokio::test]
async fn a_repository_without_the_bug_workflow_cannot_be_dispatched_for() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_bug(&home, false);
    goes_red(&fleet, 1815);
    fleet.turn().await.unwrap();
    let refused = fleet
        .fix_main(&root(&fleet).await, None, None)
        .await
        .unwrap_err();
    assert!(
        refused.to_string().contains("no `bug` workflow"),
        "{refused}"
    );
}

#[tokio::test]
async fn the_green_after_a_take_ends_it_and_names_the_job_whose_pull_request_did_it() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_bug(&home, true);
    goes_red(&fleet, 1815);
    fleet.turn().await.unwrap();
    let job = fleet
        .fix_main(&root(&fleet).await, None, None)
        .await
        .unwrap();
    let url = "https://forge.invalid/armada/pull/1821";
    fleet
        .store()
        .lock()
        .await
        .record_delivery(
            job.id(),
            &store::Delivery {
                commit: None,
                pushed: None,
                pull_request: Some(url.to_string()),
                landed: None,
                unpushed: None,
            },
        )
        .unwrap();

    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(TWO));
    forge.runs_on(TWO, vec![run("ci", "21", CiState::Passed)]);
    forge.merged_by(TWO, merged(1821));
    fleet.turn().await.unwrap();

    assert!(
        the_hub(&fleet).await.fixing.is_none(),
        "the hub's fixing clears"
    );
    let mark = fleet
        .summarised(&fleet.load(job.id()).await.unwrap())
        .await
        .unwrap()
        .fixes_main
        .expect("the Job says it fixed main");
    assert_eq!(mark.state, ipc::FixesMainState::Fixed);
    assert_eq!(mark.fixed_in, Some(1821));
}

#[tokio::test]
async fn a_job_that_was_stopped_no_longer_holds_the_red() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_bug(&home, true);
    goes_red(&fleet, 1815);
    fleet.turn().await.unwrap();
    let job = fleet
        .fix_main(&root(&fleet).await, None, None)
        .await
        .unwrap();
    Fleet::kill_job(&fleet, job.id()).await.unwrap();
    assert!(the_hub(&fleet).await.fixing.is_none());
    let again = fleet.fix_main(&root(&fleet).await, None, None).await;
    assert!(again.is_ok(), "offered again once the first was stopped");
}

#[tokio::test]
async fn the_hub_lists_the_newest_merged_pull_requests_with_the_job_that_opened_each() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_bug(&home, true);
    let finished = a_finished_job(&fleet, &home, "fix the reader").await;
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(ONE, vec![run("ci", "11", CiState::Passed)]);
    let listed = |number: u64, at: &str| RecentlyMerged {
        number,
        title: FromOutside::verbatim(format!("Change {number}")),
        branch: FromOutside::verbatim(format!("armada/{number}")),
        url: FromOutside::verbatim(format!("https://forge.invalid/armada/pull/{number}")),
        author: Some(FromOutside::verbatim("nick")),
        merged_at: FromOutside::verbatim(at),
        commit: Some(FromOutside::verbatim(ONE)),
    };
    forge.recently_merged_are(Some(vec![
        listed(1815, "2026-10-06T21:00:00Z"),
        listed(1, "2026-10-06T20:00:00Z"),
    ]));
    let before = forge.times_asked_for_the_recently_merged();
    fleet.turn().await.unwrap();

    let hub = the_hub(&fleet).await;
    let numbers: Vec<u64> = hub.merged.iter().map(|one| one.number).collect();
    assert_eq!(numbers, [1815, 1]);
    assert!(hub.merged[0].job.is_none(), "a person's");
    assert_eq!(
        hub.merged[1].job.as_ref().map(|job| job.id.clone()),
        Some(ipc::JobId::from(finished.id()))
    );
    assert_eq!(hub.merged[0].commit.as_deref(), Some(ONE));
    assert_eq!(
        forge.times_asked_for_the_recently_merged(),
        before + 1,
        "one call a visit"
    );

    // A forge that will not answer keeps what it last said.
    forge.recently_merged_are(None);
    fleet.turn().await.unwrap();
    assert_eq!(the_hub(&fleet).await.merged.len(), 2);
}
