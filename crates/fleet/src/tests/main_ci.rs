//! What Fleet learns about main's CI from the forge, and how often it asks.
//!
//! The forge is scripted: what the real commands print is `adapters`' to
//! assert. Under test here is what Fleet makes of an answer, what it keeps
//! across a restart, and which asks it does not make.

use std::time::Duration;

use adapter_traits::{CiRun, CiState, FromOutside, MergedPull, OpenPull};
use api::Queries;
use store::MainState;
use testkit::{FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::main_ci::MainChange;
use crate::noticing::Noticing;
use crate::tests::admitted::dispatched;
use crate::tests::daemon::{
    a_proposal, diff_evidence, fittings, note_evidence, worktree_directory,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<testkit::FakeHarness, FakeVcs, FakeWorkProduct>;

const ONE: &str = "1111111111111111111111111111111111111111";
const TWO: &str = "2222222222222222222222222222222222222222";

/// A runner's own summary, as a job's log ends.
const NEXTEST_TWO_FAILURES: &str = "\
     Summary [   0.021s] 2 tests run: 0 passed, 2 failed, 38 skipped
        FAIL [   0.016s] (1/2) fleet tests::one
        FAIL [   0.019s] (2/2) fleet tests::nested::two
error: test run failed
";

/// The Manifest names a base, a Check, and the CI job that answers for it.
const MANIFEST: &str = "version: 1\nid: 01FIXTUREMANIFEST\nbase: main\nchecks:\n  test:\n    run: cargo nextest run\n    ci_jobs: [ci]\n  lint:\n    run: cargo fmt --check\n";

fn a_fleet_reading_main(home: &TempDir) -> Fixture {
    a_fleet_over(home, MANIFEST)
}

fn a_fleet_over(home: &TempDir, manifest: &str) -> Fixture {
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.noticing = Noticing::every(Duration::ZERO);
    fittings.starting().manifest =
        config::Manifest::parse(std::path::Path::new("armada.yml"), manifest).expect("parses");
    Fleet::assembled(fittings)
}

fn run(name: &str, handle: &str, state: CiState) -> CiRun {
    CiRun {
        name: FromOutside::verbatim(name),
        state,
        handle: FromOutside::verbatim(handle),
        log_url: Some(FromOutside::verbatim(format!(
            "https://forge.invalid/armada/actions/runs/9/job/{handle}"
        ))),
    }
}

fn merged(number: u64) -> MergedPull {
    MergedPull {
        number,
        url: None,
        branch: Some(FromOutside::verbatim("armada/cache")),
    }
}

async fn kept(fleet: &Fixture) -> Option<store::MainCi> {
    let root = fleet.repositories().served()[0].root().to_string();
    fleet.store().lock().await.main_ci(&root).unwrap()
}

#[tokio::test]
async fn a_green_main_is_kept_green_and_asks_for_its_jobs_once() {
    let home = TempDir::new();
    let fleet = a_fleet_reading_main(&home);
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(ONE, vec![run("ci", "11", CiState::Passed)]);

    let first = fleet.turn().await.unwrap();
    assert!(
        first.main_changed.is_empty(),
        "green from the start is not news"
    );
    assert_eq!(kept(&fleet).await.unwrap().state, MainState::Green);

    fleet.turn().await.unwrap();
    fleet.turn().await.unwrap();
    assert_eq!(forge.times_asked_for_runs(), 1, "the head did not move");
    assert_eq!(forge.times_asked_for_a_log(), 0);
}

#[tokio::test]
async fn a_red_names_the_job_its_mapped_check_and_the_tests_its_log_names() {
    let home = TempDir::new();
    let fleet = a_fleet_reading_main(&home);
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(
        ONE,
        vec![
            run("ci", "11", CiState::Failed),
            run("lint", "12", CiState::Passed),
        ],
    );
    forge.log_of("11", NEXTEST_TWO_FAILURES);
    forge.merged_by(ONE, merged(1812));

    let turned = fleet.turn().await.unwrap();
    let [changed] = turned.main_changed.as_slice() else {
        panic!("one change, got {:?}", turned.main_changed)
    };
    assert_eq!(changed.change, MainChange::WentRed);
    let main = kept(&fleet).await.unwrap();
    assert_eq!(main.state, MainState::Red);
    assert_eq!(main.commit, ONE);
    assert!(main.red_at.is_some());
    let [failed] = main.failed.as_slice() else {
        panic!("one failed job, got {:?}", main.failed)
    };
    assert_eq!(failed.name, "ci");
    assert_eq!(failed.check.as_deref(), Some("test"), "declared by ci_jobs");
    assert_eq!(failed.tests, ["tests::one", "tests::nested::two"]);
    assert_eq!(
        failed.log_url.as_deref(),
        Some("https://forge.invalid/armada/actions/runs/9/job/11")
    );
    assert_eq!(main.merge.unwrap().number, 1812);
}

#[tokio::test]
async fn an_unmapped_job_with_a_log_naming_no_test_is_a_red_with_neither() {
    let home = TempDir::new();
    let fleet = a_fleet_reading_main(&home);
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(ONE, vec![run("test-all", "21", CiState::Failed)]);
    forge.log_of("21", "error: something broke\nexit code 1\n");

    fleet.turn().await.unwrap();
    let main = kept(&fleet).await.unwrap();
    assert_eq!(main.state, MainState::Red);
    assert_eq!(main.failed[0].name, "test-all");
    assert_eq!(main.failed[0].check, None, "mapping to nothing is normal");
    assert!(main.failed[0].tests.is_empty(), "never a guess");
}

#[tokio::test]
async fn a_job_named_as_a_check_maps_to_it_without_a_declaration() {
    let home = TempDir::new();
    let fleet = a_fleet_reading_main(&home);
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(ONE, vec![run("lint", "31", CiState::Failed)]);

    fleet.turn().await.unwrap();
    let main = kept(&fleet).await.unwrap();
    assert_eq!(main.failed[0].check.as_deref(), Some("lint"));
    assert!(main.failed[0].tests.is_empty(), "no log was given");
}

#[tokio::test]
async fn a_culprit_the_forge_cannot_name_is_none_and_is_asked_once() {
    let home = TempDir::new();
    let fleet = a_fleet_reading_main(&home);
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(
        ONE,
        vec![
            run("ci", "11", CiState::Failed),
            run("other", "12", CiState::Pending),
        ],
    );

    fleet.turn().await.unwrap();
    assert!(kept(&fleet).await.unwrap().merge.is_none());
    fleet.turn().await.unwrap();
    assert_eq!(forge.times_asked_for_the_merge(), 1);
}

/// The Job whose own pull request merged is found from the Job records.
#[tokio::test]
async fn the_job_that_opened_the_merging_pull_request_is_found() {
    let home = TempDir::new();
    let fleet = a_fleet_over(&home, "version: 1\nid: 01FIXTUREMANIFEST\nbase: main\n");
    let job = fleet
        .propose(a_proposal("fix the off-by-one in the log reader"))
        .await
        .unwrap();
    worktree_directory(&home, &job);
    dispatched(&fleet, job.id()).await.unwrap();
    submitted_by_the_one(&fleet, diff_evidence()).await.unwrap();
    fleet.turn().await.unwrap();
    submitted_by_the_one(&fleet, note_evidence()).await.unwrap();
    fleet.turn().await.unwrap();

    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(ONE, vec![run("ci", "11", CiState::Failed)]);
    // The fake's pull request for a finished Job is number 1.
    forge.merged_by(ONE, merged(1));

    fleet.turn().await.unwrap();
    let merge = kept(&fleet).await.unwrap().merge.unwrap();
    assert_eq!(merge.number, 1);
    assert_eq!(merge.job.as_ref(), Some(job.id()));
    let titled = the_hub(&fleet)
        .await
        .main
        .unwrap()
        .merge
        .unwrap()
        .job
        .unwrap();
    assert_eq!(titled.title, job.title().as_str());
}

#[tokio::test]
async fn a_pull_request_no_job_of_ours_opened_has_a_number_and_no_job() {
    let home = TempDir::new();
    let fleet = a_fleet_reading_main(&home);
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(ONE, vec![run("ci", "11", CiState::Failed)]);
    forge.merged_by(ONE, merged(1815));

    fleet.turn().await.unwrap();
    let merge = kept(&fleet).await.unwrap().merge.unwrap();
    assert_eq!((merge.number, merge.job), (1815, None));
}

#[tokio::test]
async fn running_jobs_are_asked_again_until_they_finish_and_a_log_is_read_once() {
    let home = TempDir::new();
    let fleet = a_fleet_reading_main(&home);
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(ONE, vec![run("ci", "11", CiState::Pending)]);
    forge.log_of("11", NEXTEST_TWO_FAILURES);

    fleet.turn().await.unwrap();
    assert_eq!(kept(&fleet).await.unwrap().state, MainState::Running);
    fleet.turn().await.unwrap();
    assert_eq!(
        forge.times_asked_for_runs(),
        2,
        "still running, so asked again"
    );

    forge.runs_on(
        ONE,
        vec![
            run("ci", "11", CiState::Failed),
            run("lint", "12", CiState::Pending),
        ],
    );
    let turned = fleet.turn().await.unwrap();
    assert_eq!(turned.main_changed[0].change, MainChange::WentRed);
    assert_eq!(kept(&fleet).await.unwrap().unfinished, 1);

    forge.runs_on(
        ONE,
        vec![
            run("ci", "11", CiState::Failed),
            run("lint", "12", CiState::Failed),
        ],
    );
    let turned = fleet.turn().await.unwrap();
    assert!(
        turned.main_changed.is_empty(),
        "the same red, one more job in it"
    );
    assert_eq!(kept(&fleet).await.unwrap().failed.len(), 2);
    assert_eq!(
        forge.times_asked_for_a_log(),
        2,
        "one log a failed job, no rereads"
    );

    fleet.turn().await.unwrap();
    assert_eq!(forge.times_asked_for_runs(), 4, "settled, so no more asks");
}

#[tokio::test]
async fn a_red_survives_a_restart_and_the_green_after_it_is_a_red_turned_green() {
    let home = TempDir::new();
    {
        let fleet = a_fleet_reading_main(&home);
        let forge = &fleet.vcs().main_ci;
        forge.head_is(Some(ONE));
        forge.runs_on(ONE, vec![run("ci", "11", CiState::Failed)]);
        forge.merged_by(ONE, merged(1812));
        fleet.turn().await.unwrap();
    }
    let fleet = a_fleet_reading_main(&home);
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    let turned = fleet.turn().await.unwrap();
    assert!(turned.main_changed.is_empty(), "nothing new, nothing told");
    assert_eq!(forge.times_asked_for_runs(), 0, "kept, so not asked again");
    let main = kept(&fleet).await.unwrap();
    assert_eq!(main.state, MainState::Red);
    assert_eq!(main.merge.unwrap().number, 1812);

    // A fix lands and is still running: main is still red.
    forge.head_is(Some(TWO));
    forge.runs_on(TWO, vec![run("ci", "21", CiState::Pending)]);
    let turned = fleet.turn().await.unwrap();
    assert!(turned.main_changed.is_empty());
    assert!(kept(&fleet).await.unwrap().red_at.is_some());

    forge.runs_on(TWO, vec![run("ci", "21", CiState::Passed)]);
    let turned = fleet.turn().await.unwrap();
    let [changed] = turned.main_changed.as_slice() else {
        panic!("one change, got {:?}", turned.main_changed)
    };
    assert_eq!(changed.change, MainChange::WentGreen);
    assert_eq!(kept(&fleet).await.unwrap().state, MainState::Green);
}

#[tokio::test]
async fn a_forge_that_will_not_answer_changes_nothing_and_a_head_alone_is_one_ask() {
    let home = TempDir::new();
    let fleet = a_fleet_reading_main(&home);
    let turned = fleet.turn().await.unwrap();
    assert!(turned.main_changed.is_empty());
    assert!(kept(&fleet).await.is_none());

    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    fleet.turn().await.unwrap();
    assert!(
        kept(&fleet).await.is_none(),
        "no answer to the jobs, nothing kept"
    );
    assert_eq!(forge.times_asked_for_the_head(), 2);
}

#[tokio::test]
async fn nothing_ran_is_not_green() {
    let home = TempDir::new();
    let fleet = a_fleet_reading_main(&home);
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(ONE, Vec::new());
    fleet.turn().await.unwrap();
    assert_eq!(kept(&fleet).await.unwrap().state, MainState::NothingRan);
}

fn pull(number: u64, ci: Option<CiState>, failing: &[&str]) -> OpenPull {
    OpenPull {
        number,
        title: FromOutside::verbatim(format!("Change {number}")),
        branch: FromOutside::verbatim(format!("armada/{number}")),
        url: FromOutside::verbatim(format!("https://forge.invalid/armada/pull/{number}")),
        author: Some(FromOutside::verbatim("nick")),
        ci,
        failing: failing
            .iter()
            .map(|name| FromOutside::verbatim(*name))
            .collect(),
    }
}

async fn the_hub(fleet: &Fixture) -> ipc::MergeLineHub {
    let root = fleet.repositories().served()[0].root().to_string();
    let hubs = fleet.merge_hubs().await;
    hubs.into_iter()
        .find_map(|(held, hub)| (held == root).then_some(hub))
        .expect("a hub for the served repository")
}

#[tokio::test]
async fn open_pull_requests_are_listed_once_a_visit_with_their_ci() {
    let home = TempDir::new();
    let fleet = a_fleet_reading_main(&home);
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(ONE, vec![run("ci", "11", CiState::Passed)]);
    forge.pulls_are(Some(vec![
        pull(7, Some(CiState::Passed), &[]),
        pull(8, Some(CiState::Pending), &[]),
        pull(9, Some(CiState::Failed), &["ci"]),
        pull(10, None, &[]),
    ]));

    fleet.turn().await.unwrap();
    assert_eq!(forge.times_asked_for_the_pulls(), 1);
    fleet.turn().await.unwrap();
    assert_eq!(forge.times_asked_for_the_pulls(), 2, "one listing a visit");

    let hub = the_hub(&fleet).await;
    let seen: Vec<(u64, Option<ipc::HubPullCi>)> = hub
        .pull_requests
        .iter()
        .map(|one| (one.number, one.ci))
        .collect();
    assert_eq!(
        seen,
        [
            (7, Some(ipc::HubPullCi::Passed)),
            (8, Some(ipc::HubPullCi::Running)),
            (9, Some(ipc::HubPullCi::Failed)),
            (10, None),
        ],
        "main is green, so a failure is the branch's own"
    );
    assert_eq!(hub.pull_requests[0].author.as_deref(), Some("nick"));
    assert!(hub.fixing.is_none());
}

#[tokio::test]
async fn a_pull_request_failing_only_where_main_fails_waits_on_main_and_any_other_failure_is_its_own(
) {
    let home = TempDir::new();
    let fleet = a_fleet_reading_main(&home);
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(
        ONE,
        vec![
            run("ci", "11", CiState::Failed),
            run("lint", "12", CiState::Passed),
        ],
    );
    forge.pulls_are(Some(vec![
        pull(7, Some(CiState::Failed), &["ci"]),
        pull(8, Some(CiState::Failed), &["ci", "lint"]),
        pull(9, Some(CiState::Passed), &[]),
    ]));

    fleet.turn().await.unwrap();
    let hub = the_hub(&fleet).await;
    let seen: Vec<_> = hub.pull_requests.iter().map(|one| one.ci).collect();
    assert_eq!(
        seen,
        [
            Some(ipc::HubPullCi::WaitingOnMain),
            Some(ipc::HubPullCi::Failed),
            Some(ipc::HubPullCi::Passed),
        ]
    );
}

#[tokio::test]
async fn the_hub_names_a_reds_jobs_and_a_fix_still_running_does_not_clear_it() {
    let home = TempDir::new();
    let fleet = a_fleet_reading_main(&home);
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(ONE, vec![run("ci", "11", CiState::Failed)]);
    forge.log_of("11", NEXTEST_TWO_FAILURES);
    forge.merged_by(ONE, merged(1815));

    fleet.turn().await.unwrap();
    let main = the_hub(&fleet).await.main.expect("main was read");
    assert_eq!(main.state, ipc::MainCiState::Red);
    assert!(main.red_since.is_some());
    assert_eq!(main.failed[0].check.as_deref(), Some("test"));
    assert_eq!(main.failed[0].tests, ["tests::one", "tests::nested::two"]);
    assert_eq!(main.merge.as_ref().map(|merge| merge.number), Some(1815));
    assert!(main.merge.unwrap().job.is_none(), "a person's");

    forge.head_is(Some(TWO));
    forge.runs_on(TWO, vec![run("ci", "21", CiState::Pending)]);
    fleet.turn().await.unwrap();
    assert_eq!(
        the_hub(&fleet).await.main.unwrap().state,
        ipc::MainCiState::Red
    );

    forge.runs_on(TWO, vec![run("ci", "21", CiState::Passed)]);
    fleet.turn().await.unwrap();
    let green = the_hub(&fleet).await.main.unwrap();
    assert_eq!(green.state, ipc::MainCiState::Green);
    assert!(green.failed.is_empty() && green.merge.is_none() && green.red_since.is_none());
}

#[tokio::test]
async fn a_forge_that_will_not_list_keeps_the_last_listing() {
    let home = TempDir::new();
    let fleet = a_fleet_reading_main(&home);
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(ONE, vec![run("ci", "11", CiState::Passed)]);
    forge.pulls_are(Some(vec![pull(7, Some(CiState::Passed), &[])]));
    fleet.turn().await.unwrap();

    forge.pulls_are(None);
    fleet.turn().await.unwrap();
    assert_eq!(the_hub(&fleet).await.pull_requests.len(), 1);

    forge.pulls_are(Some(Vec::new()));
    fleet.turn().await.unwrap();
    assert!(
        the_hub(&fleet).await.pull_requests.is_empty(),
        "none open is an answer"
    );
}

#[tokio::test]
async fn the_merge_line_carries_the_hub_where_nobody_has_run_the_runner() {
    let home = TempDir::new();
    let fleet = a_fleet_reading_main(&home);
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(ONE, vec![run("ci", "11", CiState::Passed)]);
    forge.pulls_are(Some(vec![pull(7, Some(CiState::Passed), &[])]));
    assert!(
        crate::merge_lines::answer(&fleet).await.lines.is_empty(),
        "nothing has been read"
    );

    fleet.turn().await.unwrap();
    let lines = crate::merge_lines::answer(&fleet).await;
    let [line] = lines.lines.as_slice() else {
        panic!("one line, got {lines:?}")
    };
    assert!(line.line.is_empty() && line.landed.is_empty() && line.sent_back.is_empty());
    let hub = line.hub.as_ref().expect("the hub");
    assert_eq!(hub.main.as_ref().unwrap().state, ipc::MainCiState::Green);
    assert_eq!(hub.pull_requests[0].number, 7);
}

#[tokio::test]
async fn a_failed_job_on_mains_log_is_served_by_its_check_or_its_name_while_main_is_red() {
    let home = TempDir::new();
    let fleet = a_fleet_reading_main(&home);
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(ONE, vec![run("ci", "11", CiState::Failed)]);
    forge.log_of("11", NEXTEST_TWO_FAILURES);
    fleet.turn().await.unwrap();
    let root = fleet.repositories().served()[0].root().to_string();

    for asked in ["test", "ci"] {
        let opened = fleet
            .observe_land_check(root.clone(), "main".into(), asked.into())
            .await
            .expect("a log for a failed job");
        assert_eq!(opened.branch, "main");
        let read = opened.follow.read(0, true);
        assert!(read.lines.iter().any(|line| line.contains("tests::one")));
        assert!(!opened.follow.writing(), "it ended before it was asked for");
    }
    assert!(
        fleet
            .observe_land_check(root.clone(), "main".into(), "lint".into())
            .await
            .is_err(),
        "a job that did not fail has no log here"
    );

    forge.head_is(Some(TWO));
    forge.runs_on(TWO, vec![run("ci", "21", CiState::Passed)]);
    fleet.turn().await.unwrap();
    assert!(fleet
        .observe_land_check(root, "main".into(), "ci".into())
        .await
        .is_err());
}
