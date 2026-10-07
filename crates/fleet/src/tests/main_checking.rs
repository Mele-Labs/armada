//! A red main while a newer commit's CI is still running: held, not acted on,
//! and what the forge is asked for the merged commits' own runs.

use adapter_traits::{CiState, FromOutside, RecentlyMerged};
use api::Queries;
use ipc::{MainCiState, MainRunState};

use super::main_ci::{kept, merged, run, the_hub, Fixture, ONE, TWO};
use super::main_fix::{a_finished_job, a_fleet_holding_bug, goes_red, root, takes};
use crate::main_ci::MainChange;
use crate::tests::tmp::TempDir;

const THREE: &str = "3333333333333333333333333333333333333333";

fn listed(number: u64, commit: &str, at: &str) -> RecentlyMerged {
    RecentlyMerged {
        number,
        title: FromOutside::verbatim(format!("Change {number}")),
        branch: FromOutside::verbatim(format!("armada/{number}")),
        url: FromOutside::verbatim(format!("https://forge.invalid/armada/pull/{number}")),
        author: None,
        merged_at: FromOutside::verbatim(at),
        commit: Some(FromOutside::verbatim(commit)),
    }
}

/// Red at ONE, then a newer commit TWO merged by #1852 with its run going.
async fn held(fleet: &Fixture) {
    goes_red(fleet, 1815);
    fleet.turn().await.unwrap();
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(TWO));
    forge.runs_on(TWO, vec![run("ci", "21", CiState::Pending)]);
    forge.recently_merged_are(Some(vec![listed(1852, TWO, "2026-10-06T21:00:00Z")]));
    fleet.turn().await.unwrap();
}

#[tokio::test]
async fn a_red_with_a_newer_run_going_is_held_and_names_what_is_running() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_bug(&home, false);
    held(&fleet).await;

    let main = the_hub(&fleet).await.main.unwrap();
    assert_eq!(main.state, MainCiState::Red, "an older Bridge reads red");
    assert_eq!(main.red_commit.as_deref(), Some(ONE));
    assert_eq!(main.failed[0].name, "ci", "the red's facts are kept");
    assert_eq!(main.merge.as_ref().unwrap().number, 1815);
    let [checking] = main.checking.as_slice() else {
        panic!("one commit running, got {:?}", main.checking)
    };
    assert_eq!(checking.commit, TWO);
    assert_eq!(checking.pull_request.as_ref().unwrap().number, 1852);

    // The red's own log is still the red commit's, not the running one's.
    let opened = fleet
        .observe_land_check(root(&fleet).await, "main".into(), "ci".into())
        .await;
    assert!(opened.is_ok(), "the held band's job still opens its log");
}

#[tokio::test]
async fn a_green_after_a_held_red_clears_everything() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_bug(&home, false);
    held(&fleet).await;
    fleet
        .vcs()
        .main_ci
        .runs_on(TWO, vec![run("ci", "21", CiState::Passed)]);
    let turned = fleet.turn().await.unwrap();
    assert_eq!(turned.main_changed[0].change, MainChange::WentGreen);
    let main = the_hub(&fleet).await.main.unwrap();
    assert_eq!(main.state, MainCiState::Green);
    assert!(main.checking.is_empty() && main.red_commit.is_none() && main.failed.is_empty());
}

#[tokio::test]
async fn the_same_job_failing_again_brings_the_red_back_and_a_different_one_is_a_new_red() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_bug(&home, false);
    held(&fleet).await;
    fleet
        .vcs()
        .main_ci
        .runs_on(TWO, vec![run("ci", "21", CiState::Failed)]);
    fleet.vcs().main_ci.log_of("21", "boom");
    let turned = fleet.turn().await.unwrap();
    assert_eq!(turned.main_changed[0].change, MainChange::BackToRed);
    let main = the_hub(&fleet).await.main.unwrap();
    assert!(main.checking.is_empty(), "buttons are back");
    assert_eq!(main.red_commit.as_deref(), Some(TWO));
    assert_eq!(
        main.merge.unwrap().number,
        1815,
        "the same red, the same culprit"
    );

    // Held again, then red on a job it was not red on: a new red naming the newer merge.
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(THREE));
    forge.runs_on(THREE, vec![run("ci", "31", CiState::Pending)]);
    fleet.turn().await.unwrap();
    assert!(!the_hub(&fleet).await.main.unwrap().checking.is_empty());
    forge.runs_on(THREE, vec![run("lint", "32", CiState::Failed)]);
    forge.merged_by(THREE, merged(1860));
    let turned = fleet.turn().await.unwrap();
    assert_eq!(turned.main_changed[0].change, MainChange::WentRed);
    let main = the_hub(&fleet).await.main.unwrap();
    assert!(main.checking.is_empty());
    assert_eq!(main.failed[0].name, "lint");
    assert_eq!(main.merge.unwrap().number, 1860);
}

#[tokio::test]
async fn while_held_the_fix_route_refuses_and_fleet_does_not_pick_the_red_up_until_it_returns() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_bug(&home, true);
    let culprit = a_finished_job(&fleet, &home, "fix the reader").await;
    // The red is a person's at first, so nothing was picked up; then it is read
    // as the Job's, as if the forge had named it late.
    goes_red(&fleet, 1815);
    fleet.turn().await.unwrap();
    {
        let mut main = kept(&fleet).await.unwrap();
        main.merge = Some(store::MainMerge {
            number: 1,
            url: None,
            branch: None,
            job: Some(culprit.id().clone()),
        });
        fleet.store().lock().await.record_main_ci(&main).unwrap();
    }
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(TWO));
    forge.runs_on(TWO, vec![run("ci", "21", CiState::Pending)]);
    fleet.turn().await.unwrap();

    assert!(takes(&fleet).await.is_empty(), "pickup waits");
    let refused = fleet
        .fixed_main(ipc::FixMain {
            root: root(&fleet).await,
            job: None,
            brief: None,
        })
        .await
        .unwrap_err();
    assert_eq!(refused.error().code, "fleet.main_checks_running");
    assert!(takes(&fleet).await.is_empty(), "nothing was recorded");

    forge.runs_on(TWO, vec![run("ci", "21", CiState::Failed)]);
    forge.log_of("21", "boom");
    fleet.turn().await.unwrap();
    let [take] = takes(&fleet).await.try_into().expect("picked up on return");
    assert_eq!(take.how, store::TakenHow::Took);
}

#[tokio::test]
async fn a_merged_commits_run_is_read_once_asked_again_only_while_it_runs_and_kept_when_settled() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_bug(&home, false);
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(THREE));
    forge.runs_on(THREE, vec![run("ci", "31", CiState::Passed)]);
    forge.runs_on(TWO, vec![run("ci", "21", CiState::Pending)]);
    forge.runs_on(ONE, vec![run("ci", "11", CiState::Failed)]);
    forge.log_of("11", "the failure");
    forge.recently_merged_are(Some(vec![
        listed(3, THREE, "2020-01-01T00:00:00Z"),
        listed(2, TWO, "2020-01-01T00:00:00Z"),
        listed(1, ONE, "2020-01-01T00:00:00Z"),
    ]));

    fleet.turn().await.unwrap();
    let states = |hub: &ipc::MergeLineHub| -> Vec<Option<MainRunState>> {
        hub.merged
            .iter()
            .map(|one| one.main_run.as_ref().map(|run| run.state))
            .collect()
    };
    let hub = the_hub(&fleet).await;
    assert_eq!(
        states(&hub),
        [
            Some(MainRunState::Passed),
            Some(MainRunState::Running),
            Some(MainRunState::Failed)
        ]
    );
    assert_eq!(hub.merged[2].main_run.as_ref().unwrap().failed, ["ci"]);
    let asked = forge.times_asked_for_runs();

    // Next visit: only the unfinished one is asked again (the head is settled).
    fleet.turn().await.unwrap();
    assert_eq!(forge.times_asked_for_runs(), asked + 1);

    forge.runs_on(TWO, vec![run("ci", "21", CiState::Passed)]);
    fleet.turn().await.unwrap();
    let settled = forge.times_asked_for_runs();
    fleet.turn().await.unwrap();
    fleet.turn().await.unwrap();
    assert_eq!(
        forge.times_asked_for_runs(),
        settled,
        "all settled: no asks"
    );
    assert_eq!(
        states(&the_hub(&fleet).await)[1],
        Some(MainRunState::Passed)
    );

    // The failed one's log opens through the Check log under `main@<commit>`.
    let opened = fleet
        .observe_land_check(root(&fleet).await, format!("main@{ONE}"), "ci".into())
        .await
        .expect("the failed run's log");
    assert_eq!(opened.branch, format!("main@{ONE}"));
    assert!(fleet
        .observe_land_check(root(&fleet).await, format!("main@{TWO}"), "ci".into())
        .await
        .is_err());
}

#[tokio::test]
async fn a_merge_nothing_has_run_on_yet_is_asked_again_while_it_is_new_and_none_once_old() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_bug(&home, false);
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(ONE, vec![]);
    forge.recently_merged_are(Some(vec![
        listed(1, ONE, "2999-01-01T00:00:00Z"),
        listed(2, TWO, "2020-01-01T00:00:00Z"),
    ]));
    forge.runs_on(TWO, vec![]);
    fleet.turn().await.unwrap();
    let after_first = forge.times_asked_for_runs();
    // The head is settled and not asked again: only the minutes-old merge is.
    fleet.turn().await.unwrap();
    assert_eq!(forge.times_asked_for_runs(), after_first + 1);
    assert!(the_hub(&fleet)
        .await
        .merged
        .iter()
        .all(|one| one.main_run.is_none()));
}

fn sha(digit: char) -> String {
    digit.to_string().repeat(40)
}

fn forge_lists(fleet: &Fixture, merges: &[(u64, char, CiState)]) {
    let forge = &fleet.vcs().main_ci;
    forge.recently_merged_are(Some(
        merges
            .iter()
            .enumerate()
            .map(|(at, (number, digit, _))| {
                listed(*number, &sha(*digit), &format!("2020-01-01T00:0{at}:00Z"))
            })
            .collect(),
    ));
    for (number, digit, state) in merges {
        forge.runs_on(&sha(*digit), vec![run("ci", &format!("{number}"), *state)]);
        forge.merged_by(&sha(*digit), merged(*number));
    }
}

#[tokio::test]
async fn a_green_run_in_between_clears_a_red_while_newer_checks_are_still_running() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_bug(&home, false);
    goes_red(&fleet, 1845);
    fleet.turn().await.unwrap();
    assert_eq!(kept(&fleet).await.unwrap().state, store::MainState::Red);

    // #1870 passed on main; #1872 to #1875 are running, the head among them.
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(&sha('f')));
    forge_lists(
        &fleet,
        &[
            (1875, 'f', CiState::Pending),
            (1874, 'e', CiState::Pending),
            (1873, 'd', CiState::Pending),
            (1872, 'c', CiState::Pending),
            (1870, 'b', CiState::Passed),
        ],
    );
    let turned = fleet.turn().await.unwrap();
    assert_eq!(turned.main_changed[0].change, MainChange::WentGreen);
    let main = the_hub(&fleet).await.main.unwrap();
    assert_eq!(main.state, MainCiState::Green);
    assert!(main.failed.is_empty() && main.merge.is_none() && main.red_since.is_none());
    assert!(main.red_commit.is_none());
    let running: Vec<u64> = main
        .checking
        .iter()
        .map(|one| one.pull_request.as_ref().unwrap().number)
        .collect();
    assert_eq!(running, [1875, 1874, 1873, 1872], "neutral checks running");

    let refused = fleet
        .fixed_main(ipc::FixMain {
            root: root(&fleet).await,
            job: None,
            brief: None,
        })
        .await
        .unwrap_err();
    assert_eq!(refused.error().code, "fleet.main_not_red", "nothing to fix");
}

#[tokio::test]
async fn a_red_after_a_green_names_the_first_red_after_it_and_red_red_red_names_the_first() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_bug(&home, false);
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(&sha('c')));
    forge_lists(
        &fleet,
        &[
            (1881, 'c', CiState::Failed),
            (1880, 'b', CiState::Passed),
            (1879, 'a', CiState::Failed),
        ],
    );
    forge.log_of("1881", "boom");
    fleet.turn().await.unwrap();
    let main = the_hub(&fleet).await.main.unwrap();
    assert_eq!(main.state, MainCiState::Red);
    assert_eq!(main.merge.unwrap().number, 1881, "the red after the green");

    let again = a_fleet_holding_bug(&TempDir::new(), false);
    let forge = &again.vcs().main_ci;
    forge.head_is(Some(&sha('c')));
    forge_lists(
        &again,
        &[
            (1873, 'c', CiState::Failed),
            (1872, 'b', CiState::Failed),
            (1871, 'a', CiState::Failed),
        ],
    );
    forge.log_of("1873", "boom");
    again.turn().await.unwrap();
    let main = the_hub(&again).await.main.unwrap();
    assert_eq!(main.merge.unwrap().number, 1871, "the first of the reds");
    assert_eq!(main.red_commit.as_deref(), Some(sha('c').as_str()));
}

#[tokio::test]
async fn a_row_the_old_build_wrote_is_read_once_more_whatever_the_head() {
    let home = TempDir::new();
    {
        let fleet = a_fleet_holding_bug(&home, false);
        goes_red(&fleet, 1845);
        fleet.turn().await.unwrap();
        // What the build before this one left: a red with nothing decided and nothing unfinished.
        let mut old = kept(&fleet).await.unwrap();
        old.decided_commit = None;
        old.newer_running = 0;
        old.unfinished = 0;
        fleet.store().lock().await.record_main_ci(&old).unwrap();
    }
    let fleet = a_fleet_holding_bug(&home, false);
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(ONE));
    forge.runs_on(ONE, vec![run("ci", "11", CiState::Passed)]);
    let turned = fleet.turn().await.unwrap();
    assert_eq!(turned.main_changed[0].change, MainChange::WentGreen);
    let main = kept(&fleet).await.unwrap();
    assert_eq!(main.state, store::MainState::Green);
    assert_eq!(main.decided_commit.as_deref(), Some(ONE));
}

#[tokio::test]
async fn the_newest_finished_commit_decides_even_past_the_newest_six_all_running() {
    let home = TempDir::new();
    let fleet = a_fleet_holding_bug(&home, false);
    goes_red(&fleet, 1845);
    fleet.turn().await.unwrap();

    // Seven running (the head among them), and the eighth newest finished green.
    let forge = &fleet.vcs().main_ci;
    forge.head_is(Some(&sha('h')));
    let merges: Vec<(u64, char, CiState)> = "hgfedcb"
        .chars()
        .zip((1876..=1882).rev())
        .map(|(digit, number)| (number, digit, CiState::Pending))
        .chain([(1874, 'a', CiState::Passed)])
        .collect();
    forge_lists(&fleet, &merges);
    let turned = fleet.turn().await.unwrap();
    assert_eq!(turned.main_changed[0].change, MainChange::WentGreen);
    let hub = the_hub(&fleet).await;
    let main = hub.main.unwrap();
    assert_eq!(main.state, MainCiState::Green);
    assert!(main.failed.is_empty() && main.red_commit.is_none());
    assert_eq!(main.checking.len(), 7, "every newer run still going");
    assert_eq!(hub.merged.len(), 5, "Recently landed keeps its five");

    // Settled and running ones are cached: nothing but the running are asked again.
    let before = forge.times_asked_for_runs();
    fleet.turn().await.unwrap();
    assert_eq!(forge.times_asked_for_runs(), before + 7);
}
