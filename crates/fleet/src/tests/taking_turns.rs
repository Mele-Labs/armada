//! The merge line survives a Fleet restart at every point a landing can be cut
//! short. A restart here is a Fleet dropped and another assembled over the same
//! store; a kill is a turn aborted mid-gate with its turn row left held by a
//! process that is gone.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use config::Manifest;
use core_model::{JobId, JobStatus};
use store::{Blame, LineState, TurnHolder};
use testkit::{Delivered, FakeWorkProduct, Merging};

use crate::daemon::Fleet;
use crate::gate::CheckBudget;
use crate::noticing::Noticing;
use crate::tests::daemon::{fittings, one};
use crate::tests::merging::{at_the_gate_having_delivered, Fixture, PULL_REQUEST};
use crate::tests::pushing_onto_base::{
    counted, holding_the_work_under, moved, workflow, worktree_of,
};
use crate::tests::tmp::TempDir;

/// A Check runs its command with no shell, so a script is handed to `sh`.
fn manifest(script: &str) -> String {
    let check = format!("sh -c {script:?}");
    format!("version: 1\nid: 01FIXTUREMANIFEST\nmerge_by: push\nchecks:\n  suite:\n    run: {check:?}\n")
}

/// A Check that holds the turn, once `armed` is in the worktree, until
/// `release` is: bounded, so an aborted turn leaves nothing running for long.
const HOLDS: &str = "if [ -e armed ]; then touch started; i=0; \
                     while [ ! -e release ] && [ $i -lt 100 ]; do sleep 0.1; i=$((i+1)); done; fi";

/// What `ps` says of a pid nothing holds, so the holder reads as gone.
fn a_dead_fleet() -> TurnHolder {
    TurnHolder {
        run: String::from("01DEADFLEETRUN00000000000"),
        pid: u32::MAX,
        started: String::from("long ago"),
    }
}

async fn root_of(fleet: &Fixture, job: &JobId) -> String {
    let job = fleet.load(job).await.expect("the Job");
    fleet.served_by(&job).expect("served").root().to_string()
}

/// A press that joined the line and died before anything else happened.
async fn pressed_and_died(fleet: &Fixture, job: &JobId, root: &str) -> i64 {
    let at = fleet.now();
    fleet
        .store()
        .lock()
        .await
        .join_the_line(
            root,
            job,
            "01NONCE0000000000000000000",
            "human",
            PULL_REQUEST,
            &at,
        )
        .expect("joins")
        .id
}

async fn entry(fleet: &Fixture, id: i64) -> store::LineEntry {
    fleet
        .store()
        .lock()
        .await
        .line_entry(id)
        .expect("reads")
        .expect("kept")
}

async fn status(fleet: &Fixture, job: &JobId) -> JobStatus {
    fleet.load(job).await.expect("the Job").status()
}

/// **Queued, then restarted**: the next Fleet finds the entry where it was and
/// lands it with nobody pressing.
#[tokio::test]
async fn an_entry_queued_before_a_restart_is_landed_by_the_next_fleet() {
    let home = TempDir::new();
    let first = holding_the_work_under(&home, &manifest("true"));
    let job = at_the_gate_having_delivered(&first, &home).await;
    let root = root_of(&first, &job).await;
    let id = pressed_and_died(&first, &job, &root).await;
    drop(first);

    let next = holding_the_work_under(&home, &manifest("true"));
    assert!(next.drive_the_line(&root).await.expect("a turn"));

    assert_eq!(status(&next, &job).await, JobStatus::CompletedSuccess);
    let landed = entry(&next, id).await;
    assert_eq!((landed.state, landed.finished), (LineState::Landed, true));
    assert_eq!(
        counted(&next, |it| matches!(it, Delivered::MergedByPush { .. })),
        1
    );
}

/// **Killed mid-gate**: the turn is aborted while a Check runs, leaving the
/// entry waiting and the turn held. The next Fleet takes the turn over, clears
/// the worktree first, and lands the entry once.
#[tokio::test]
async fn a_turn_killed_mid_gate_leaves_its_entry_and_the_next_fleet_takes_it_up() {
    let home = TempDir::new();
    let first = Arc::new(holding_the_work_under(&home, &manifest(HOLDS)));
    let job = at_the_gate_having_delivered(&first, &home).await;
    let root = root_of(&first, &job).await;
    let worktree = worktree_of(&first, &home, &job).await;
    std::fs::write(worktree.join("armed"), "").expect("armed");
    first.vcs().pushing_in_turn(vec![moved(), Merging::Takes]);
    let press = tokio::spawn({
        let (first, job) = (Arc::clone(&first), job.clone());
        async move { first.merge_pull_request(&job).await }
    });
    until(|| worktree.join("started").exists()).await;

    press.abort();
    let _ = press.await;
    std::fs::write(worktree.join("release"), "").expect("let the Check go");

    let held = first.store().lock().await.turn_held(&root).expect("reads");
    assert_eq!(held.expect("still held").run, first.run().as_str());
    let waiting = first
        .store()
        .lock()
        .await
        .waiting_in_line(&root)
        .expect("reads");
    assert_eq!(waiting.len(), 1, "the entry was not lost");
    let at = first.now();
    assert!(first
        .store()
        .lock()
        .await
        .take_over_turn(&root, first.run().as_str(), &a_dead_fleet(), &at)
        .expect("writes"));
    drop(first);

    let next = holding_the_work_under(&home, &manifest(HOLDS));
    assert!(next.drive_the_line(&root).await.expect("a turn"));

    assert_eq!(status(&next, &job).await, JobStatus::CompletedSuccess);
    assert_eq!(entry(&next, waiting[0].id).await.state, LineState::Landed);
    assert_eq!(
        counted(&next, |it| matches!(it, Delivered::SettledWorktree { .. })),
        1
    );
    assert_eq!(
        next.store().lock().await.turn_held(&root).expect("reads"),
        None
    );
}

/// **Killed after the push**: the base already holds the branch, so the entry
/// lands naming the merge and nothing is merged a second time.
#[tokio::test]
async fn a_turn_killed_after_its_push_lands_naming_the_merge_and_pushes_nothing_twice() {
    let home = TempDir::new();
    let first = holding_the_work_under(&home, &manifest("true"));
    let job = at_the_gate_having_delivered(&first, &home).await;
    let root = root_of(&first, &job).await;
    let id = pressed_and_died(&first, &job, &root).await;
    let at = first.now();
    first
        .store()
        .lock()
        .await
        .hold_turn(&root, &a_dead_fleet(), &at)
        .expect("held by the dead");
    drop(first);

    let next = holding_the_work_under(&home, &manifest("true"));
    next.vcs().merging(Merging::AlreadyMerged);
    assert!(next.drive_the_line(&root).await.expect("a turn"));

    let landed = entry(&next, id).await;
    assert_eq!(landed.state, LineState::Landed);
    assert!(landed.merge_commit.is_some(), "it names the merge");
    assert_eq!(
        next.vcs().times_pushed_onto_the_base(),
        1,
        "asked once, answered as held"
    );
    assert_eq!(status(&next, &job).await, JobStatus::CompletedSuccess);
}

/// **A landing the Fleet died before finishing**: the entry is landed and the
/// Job still at its gate; the next Fleet moves the Job on without pushing.
#[tokio::test]
async fn a_landing_not_finished_before_the_kill_is_finished_without_a_push() {
    let home = TempDir::new();
    let first = holding_the_work_under(&home, &manifest("true"));
    let job = at_the_gate_having_delivered(&first, &home).await;
    let root = root_of(&first, &job).await;
    let id = pressed_and_died(&first, &job, &root).await;
    let landed = store::Ended {
        state: LineState::Landed,
        kind: None,
        said: None,
        merge_commit: Some(String::from("3e9a7c1000000000000000000000000000000000")),
        base: Some(String::from("main")),
        blamed: None,
    };
    let (at, nonce) = (first.now(), entry(&first, id).await.nonce);
    first
        .store()
        .lock()
        .await
        .end_line_entry(id, &nonce, &landed, &at)
        .expect("writes");
    drop(first);

    let next = holding_the_work_under(&home, &manifest("true"));
    next.drive_the_line(&root).await.expect("a turn");

    assert_eq!(status(&next, &job).await, JobStatus::CompletedSuccess);
    assert!(entry(&next, id).await.finished);
    assert_eq!(next.vcs().times_pushed_onto_the_base(), 0);
}

/// **The size a turn takes is kept**: a red halves it, and the next Fleet reads
/// what the last wrote.
#[tokio::test]
async fn the_size_a_turn_takes_survives_a_restart() {
    let home = TempDir::new();
    let first = holding_the_work_under(&home, &manifest("test ! -e red"));
    let job = at_the_gate_having_delivered(&first, &home).await;
    let root = root_of(&first, &job).await;
    std::fs::write(worktree_of(&first, &home, &job).await.join("red"), "").expect("red");
    first.vcs().pushing_in_turn(vec![moved()]);
    first.merge_pull_request(&job).await.expect_err("red");
    drop(first);

    let next = holding_the_work_under(&home, &manifest("test ! -e red"));
    let kept = next
        .store()
        .lock()
        .await
        .line_size(&root)
        .expect("reads")
        .expect("kept");
    assert_eq!((kept.size, kept.reason.as_str()), (4, "halved after a red"));
}

/// An entry whose Job left its gate is taken out of line, not landed.
#[tokio::test]
async fn an_entry_whose_job_left_its_gate_is_dropped_and_nothing_is_pushed() {
    let home = TempDir::new();
    let fleet = holding_the_work_under(&home, &manifest("true"));
    let job = at_the_gate_having_delivered(&fleet, &home).await;
    let root = root_of(&fleet, &job).await;
    let id = pressed_and_died(&fleet, &job, &root).await;
    fleet.reject(&job).await.expect("rejected");

    fleet.drive_the_line(&root).await.expect("a turn");

    assert_eq!(entry(&fleet, id).await.state, LineState::Stopped);
    assert_eq!(fleet.vcs().times_pushed_onto_the_base(), 0);
}

/// The base commit the base checkout stands at in these cases.
const BASE: &str = "5b4ec82700000000000000000000000000000000";

/// A base checkout on disk, so asking the base can run a Check there.
fn a_base_checkout(fleet: &Fixture, root: &str) -> std::path::PathBuf {
    fleet.vcs().move_ref_to("main", BASE);
    let spec = adapter_traits::BaseSpec::at(root, BASE).expect("a spec");
    std::fs::create_dir_all(spec.path()).expect("a base checkout");
    std::fs::write(spec.ready_marker(), "").expect("ready");
    std::path::PathBuf::from(spec.path())
}

/// **Green on the base, red then green on the branch**: the Job's log names
/// the Check and its log and says the turn goes on, and the rerun lands it.
#[tokio::test]
async fn a_check_the_base_passes_is_run_once_more_and_the_turn_goes_on() {
    let home = TempDir::new();
    let fleet = holding_the_work_under(&home, &manifest("if [ -e red ]; then rm red; exit 1; fi"));
    let job = at_the_gate_having_delivered(&fleet, &home).await;
    let root = root_of(&fleet, &job).await;
    a_base_checkout(&fleet, &root);
    std::fs::write(worktree_of(&fleet, &home, &job).await.join("red"), "").expect("red");
    fleet.vcs().pushing_in_turn(vec![moved(), Merging::Takes]);

    let landed = fleet.merge_pull_request(&job).await.expect("it lands");

    assert_eq!(landed.status(), JobStatus::CompletedSuccess);
    let log = log_of_the_job(&fleet, &job).await;
    assert!(
        log.contains("suite failed and the base is green for it, so the turn runs it once more"),
        "{log}"
    );
}

/// **Red on the base too**: the failure is main's, nothing is rerun, and the
/// entry records whose it was.
#[tokio::test]
async fn a_check_the_base_fails_too_is_the_bases_and_is_recorded_so() {
    let home = TempDir::new();
    let fleet = holding_the_work_under(&home, &manifest("test ! -e red"));
    let job = at_the_gate_having_delivered(&fleet, &home).await;
    let root = root_of(&fleet, &job).await;
    std::fs::write(a_base_checkout(&fleet, &root).join("red"), "").expect("red base");
    std::fs::write(worktree_of(&fleet, &home, &job).await.join("red"), "").expect("red");
    fleet.vcs().pushing_in_turn(vec![moved()]);

    let refused = fleet.merge_pull_request(&job).await.expect_err("main's");

    assert!(
        refused.to_string().contains("the base's to fix"),
        "{refused}"
    );
    let ended = fleet
        .store()
        .lock()
        .await
        .line_entry(1)
        .expect("reads")
        .expect("kept");
    assert_eq!(ended.blamed, Some(Blame::Base));
}

/// **A real failure**: green on the base, red twice on the branch, so it is the
/// branch's and is recorded so.
#[tokio::test]
async fn a_check_that_fails_again_with_the_base_green_is_the_branchs() {
    let home = TempDir::new();
    let fleet = holding_the_work_under(&home, &manifest("test ! -e red"));
    let job = at_the_gate_having_delivered(&fleet, &home).await;
    let root = root_of(&fleet, &job).await;
    a_base_checkout(&fleet, &root);
    std::fs::write(worktree_of(&fleet, &home, &job).await.join("red"), "").expect("red");
    fleet.vcs().pushing_in_turn(vec![moved()]);

    fleet
        .merge_pull_request(&job)
        .await
        .expect_err("the branch's");

    let ended = fleet
        .store()
        .lock()
        .await
        .line_entry(1)
        .expect("reads")
        .expect("kept");
    assert_eq!(ended.blamed, Some(Blame::Branch));
}

/// **Asked once per base commit**: a second turn on the same commit does not
/// run the Check on the base again.
#[tokio::test]
async fn the_base_is_asked_once_per_commit() {
    let home = TempDir::new();
    let fleet = holding_the_work_under(&home, &manifest("echo x >> asked; test ! -e red"));
    let job = at_the_gate_having_delivered(&fleet, &home).await;
    let root = root_of(&fleet, &job).await;
    let base = a_base_checkout(&fleet, &root);
    std::fs::write(worktree_of(&fleet, &home, &job).await.join("red"), "").expect("red");
    fleet.vcs().pushing_in_turn(vec![moved()]);
    fleet.merge_pull_request(&job).await.expect_err("red");
    fleet.vcs().pushing_in_turn(vec![moved()]);
    fleet.merge_pull_request(&job).await.expect_err("red again");

    let asked = std::fs::read_to_string(base.join("asked")).expect("it ran there");
    assert_eq!(asked.lines().count(), 1);
}

/// **A timeout on the base is not remembered**, so the next turn asks again.
/// The budget is a second and the Check sleeps past it, on the branch and on
/// the base alike; each turn that asks the base runs it there once.
#[tokio::test]
async fn a_timeout_on_the_base_is_not_cached() {
    let home = TempDir::new();
    let text = manifest("if [ -e red ]; then echo x >> asked; sleep 20; fi");
    let parsed = Manifest::parse(Path::new("armada.yml"), &text).expect("a manifest");
    let mut fittings = fittings(&home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.starting().workflows = one(workflow(&parsed));
    fittings.starting().manifest = parsed;
    fittings.noticing = Noticing::every(Duration::ZERO);
    fittings.budget = CheckBudget::of(Duration::from_secs(1));
    let fleet = Fleet::assembled(fittings);
    let job = at_the_gate_having_delivered(&fleet, &home).await;
    let root = root_of(&fleet, &job).await;
    let base = a_base_checkout(&fleet, &root);
    std::fs::write(base.join("red"), "").expect("the base runs it too");
    std::fs::write(worktree_of(&fleet, &home, &job).await.join("red"), "").expect("red");
    fleet.vcs().pushing_in_turn(vec![moved()]);
    fleet.merge_pull_request(&job).await.expect_err("timed out");
    fleet.vcs().pushing_in_turn(vec![moved()]);
    fleet
        .merge_pull_request(&job)
        .await
        .expect_err("timed out again");

    let asked = std::fs::read_to_string(base.join("asked")).expect("it ran there");
    assert_eq!(asked.lines().count(), 2, "asked again, not remembered");
    let ended = fleet
        .store()
        .lock()
        .await
        .line_entry(1)
        .expect("reads")
        .expect("kept");
    assert_eq!(ended.blamed, Some(Blame::Base));
}

async fn log_of_the_job(fleet: &Fixture, job: &JobId) -> String {
    let job = fleet.load(job).await.expect("the Job");
    let served = fleet.served_by(&job).expect("served");
    let path = crate::transcript::log_of(served.records_root(), &job.handle());
    std::fs::read_to_string(path).expect("the Job's log")
}

async fn until(ready: impl Fn() -> bool) {
    for _ in 0..400 {
        if ready() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the Check never started");
}
