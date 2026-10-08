//! `merge_by: push` over a base that moved: Fleet brings the branch up, gates
//! it again and pushes once more.
//!
//! The pushes and the merge are scripted, as in `merging_by_push`; what `git`
//! does is asserted in `adapters`. The Checks are real commands, so a red is a
//! command that exited non-zero rather than a row the test wrote.

use std::path::{Path, PathBuf};
use std::time::Duration;

use adapter_traits::{KeptCurrent, Landing, NotMerged, Rendering};
use adapters::onto_base::ROUNDS;
use config::{Manifest, ResolvedWorkflow, Roster, WorkflowDef};
use core_model::{JobId, JobStatus};
use testkit::{Delivered, FakeWorkProduct, Merging};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::noticing::Noticing;
use crate::tests::daemon::{fittings, one};
use crate::tests::merging::{at_the_gate_having_delivered, Fixture, PULL_REQUEST};
use crate::tests::tmp::TempDir;

/// `suite` is red exactly where a file named `red` stands in the worktree, so
/// a case turns the merged tree red by writing one after the first gate.
const MANIFEST: &str = "version: 1\nid: 01FIXTUREMANIFEST\nmerge_by: push\n\
                        checks:\n  suite:\n    run: \"test ! -e red\"\n";

/// Implement gated on every Manifest Check, then a hand-off held for a person
/// that sends the work out and declares none — every shipped coding
/// workflow's shape.
pub(super) fn workflow(manifest: &Manifest) -> ResolvedWorkflow {
    let def = WorkflowDef::parse(
        Path::new("fixture.yml"),
        "version: 1\nworkflow_id: fixture-workflow\nname: fixture\n\
         steps:\n  - id: implement\n    label: \"Implement\"\n    \
         evidence: {submitted: {type: diff}}\n    mechanical_checks:\n      \
         - type: every_manifest_check\n      - type: diff_nonempty\n    \
         delivers: false\n    advance_gate: auto\n  - id: summarise\n    \
         label: \"Summarise\"\n    evidence: {submitted: {type: facts_note}}\n    \
         delivers: true\n    advance_gate: human_always\n",
        &Roster::offering_nothing(),
    )
    .unwrap_or_else(|refused| panic!("the fixture workflow did not parse: {refused}"));
    ResolvedWorkflow::resolve(&def, manifest).expect("every name declared")
}

fn holding_the_work(home: &TempDir) -> Fixture {
    holding_the_work_under(home, MANIFEST)
}

pub(super) fn holding_the_work_under(home: &TempDir, text: &str) -> Fixture {
    let manifest = Manifest::parse(Path::new("armada.yml"), text).expect("a manifest");
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.starting().workflows = one(workflow(&manifest));
    fittings.starting().manifest = manifest;
    fittings.noticing = Noticing::every(Duration::ZERO);
    Fleet::assembled(fittings)
}

pub(super) async fn worktree_of(fleet: &Fixture, home: &TempDir, job_id: &JobId) -> PathBuf {
    let job = fleet.load(job_id).await.expect("the Job");
    let spec = crate::tests::daemon::spec_held(home, &job).expect("a spec");
    PathBuf::from(spec.worktree_path())
}

pub(super) fn moved() -> Merging {
    Merging::Refuses(NotMerged::BaseMoved {
        said: String::from("main moved on origin between the fetch and the push"),
    })
}

pub(super) fn counted(fleet: &Fixture, which: impl Fn(&Delivered) -> bool) -> usize {
    fleet
        .vcs()
        .delivered()
        .iter()
        .filter(|it| which(it))
        .count()
}

pub(super) fn wire_code(fleet: &Fixture, refused: Adrift) -> (String, String) {
    match fleet.refusal(refused) {
        api::Refusal::IllegalMove(wire) => (wire.code.to_string(), wire.message.to_string()),
        other => panic!("the person's to answer, so a conflict: {other:?}"),
    }
}

pub(super) async fn still_at_the_gate(fleet: &Fixture, job_id: &JobId) {
    let held = fleet.load(job_id).await.expect("the Job is there");
    assert_eq!(held.status(), JobStatus::AwaitingReview);
}

#[tokio::test]
async fn a_moved_base_is_merged_in_gated_again_and_landed() {
    let home = TempDir::new();
    let fleet = holding_the_work(&home);
    let job_id = at_the_gate_having_delivered(&fleet, &home).await;
    fleet.vcs().pushing_in_turn(vec![moved(), Merging::Takes]);

    let job = fleet
        .merge_pull_request(&job_id)
        .await
        .expect("brought up, green, and pushed");

    assert_eq!(job.status(), JobStatus::CompletedSuccess);
    assert_eq!(fleet.vcs().times_pushed_onto_the_base(), 2);
    assert_eq!(
        counted(&fleet, |it| matches!(it, Delivered::MergedTheBaseIn { .. })),
        1
    );
    assert_eq!(
        counted(&fleet, |it| matches!(it, Delivered::PutBack { .. })),
        0
    );
}

#[tokio::test]
async fn a_moved_base_that_conflicts_is_refused_and_nothing_is_pushed_again() {
    let home = TempDir::new();
    let fleet = holding_the_work(&home);
    let job_id = at_the_gate_having_delivered(&fleet, &home).await;
    fleet.vcs().pushing_in_turn(vec![moved()]);
    fleet
        .vcs()
        .merging_the_base_in_refuses(NotMerged::Conflicted {
            said: String::from("main conflicts with the branch in src/log.rs"),
        });

    let refused = fleet
        .merge_pull_request(&job_id)
        .await
        .expect_err("a conflict is the branch's to resolve");

    let (code, message) = wire_code(&fleet, refused);
    assert_eq!(code, "fleet.merge_conflicted");
    assert!(message.contains("src/log.rs"), "{message}");
    assert_eq!(fleet.vcs().times_pushed_onto_the_base(), 1);
    still_at_the_gate(&fleet, &job_id).await;
}

#[tokio::test]
async fn a_moved_base_that_turns_the_checks_red_is_refused_and_put_back() {
    let home = TempDir::new();
    let fleet = holding_the_work(&home);
    let job_id = at_the_gate_having_delivered(&fleet, &home).await;
    let worktree = worktree_of(&fleet, &home, &job_id).await;
    std::fs::write(worktree.join("red"), "the merged tree").expect("the marker");
    fleet.vcs().pushing_in_turn(vec![moved()]);

    let refused = fleet
        .merge_pull_request(&job_id)
        .await
        .expect_err("nothing red is pushed");

    let (code, message) = wire_code(&fleet, refused);
    assert_eq!(code, "fleet.merge_gate_failed");
    assert!(message.contains("suite"), "names the Check: {message}");
    assert_eq!(fleet.vcs().times_pushed_onto_the_base(), 1);
    assert_eq!(
        counted(&fleet, |it| matches!(it, Delivered::PutBack { .. })),
        1,
        "a branch left holding the base would land unread on the next press"
    );
    still_at_the_gate(&fleet, &job_id).await;
}

#[tokio::test]
async fn a_base_that_keeps_moving_is_refused_naming_the_rounds() {
    let home = TempDir::new();
    let fleet = holding_the_work(&home);
    let job_id = at_the_gate_having_delivered(&fleet, &home).await;
    fleet.vcs().merging(moved());

    let refused = fleet
        .merge_pull_request(&job_id)
        .await
        .expect_err("it gives up");

    let (code, message) = wire_code(&fleet, refused);
    assert_eq!(code, "fleet.merge_base_moved");
    assert!(message.contains(&format!("{ROUNDS} rounds")), "{message}");
    assert_eq!(
        counted(&fleet, |it| matches!(it, Delivered::MergedTheBaseIn { .. })),
        ROUNDS as usize
    );
    assert_eq!(
        fleet.vcs().times_pushed_onto_the_base(),
        ROUNDS as usize + 1
    );
    still_at_the_gate(&fleet, &job_id).await;
}

/// The sweep finds the pull request behind a moved `main` and merges it into
/// the branch, which it pushes without running a Check.
async fn kept_current_by_the_sweep(fleet: &Fixture) {
    let onto = "7d3e1f0000000000000000000000000000000000";
    fleet.vcs().move_ref_to("main", onto);
    fleet.vcs().now_kept_current(KeptCurrent::Rebased {
        onto: onto.to_string(),
        commits: 1,
    });
    fleet.vcs().now_landed(Landing::Open {
        url: String::from(PULL_REQUEST),
        rendering: Rendering::FromASupersededBase {
            pinned: String::from("67cb1b9e"),
            written_on: String::from("8c2ce681"),
        },
    });
    fleet.turn().await.expect("the sweep runs");
    assert_eq!(
        fleet.vcs().times_kept_current(),
        1,
        "the sweep merged it in"
    );
}

/// **The hole the sweep left:** its merge made a head no Check read, and the
/// branch already holds the base, so nothing about the base says to gate it.
/// The press runs them over that head first, and red pushes nothing.
#[tokio::test]
async fn a_branch_the_sweep_kept_current_is_checked_before_it_is_pushed() {
    let home = TempDir::new();
    let fleet = holding_the_work(&home);
    let job_id = at_the_gate_having_delivered(&fleet, &home).await;
    kept_current_by_the_sweep(&fleet).await;
    let worktree = worktree_of(&fleet, &home, &job_id).await;
    std::fs::write(worktree.join("red"), "the merged tree").expect("the marker");

    let refused = fleet
        .merge_pull_request(&job_id)
        .await
        .expect_err("a head no Check passed on is not pushed");

    let (code, message) = wire_code(&fleet, refused);
    assert_eq!(code, "fleet.merge_gate_failed");
    assert!(message.contains("suite"), "names the Check: {message}");
    assert_eq!(
        counted(&fleet, |it| matches!(it, Delivered::PutBack { .. })),
        0,
        "the sweep's merge is on the remote branch already, so it is not undone"
    );
    still_at_the_gate(&fleet, &job_id).await;
}

/// Green over the sweep's head, it lands.
#[tokio::test]
async fn a_branch_the_sweep_kept_current_lands_once_its_checks_pass() {
    let home = TempDir::new();
    let fleet = holding_the_work(&home);
    let job_id = at_the_gate_having_delivered(&fleet, &home).await;
    kept_current_by_the_sweep(&fleet).await;

    let job = fleet
        .merge_pull_request(&job_id)
        .await
        .expect("checked, green, and pushed");

    assert_eq!(job.status(), JobStatus::CompletedSuccess);
    assert_eq!(
        counted(&fleet, |it| matches!(it, Delivered::MergedTheBaseIn { .. })),
        0,
        "the branch held the base already"
    );
    assert_eq!(
        counted(&fleet, |it| matches!(
            it,
            Delivered::ReadTheUncheckedHead { .. }
        )),
        1,
        "its head was read for the Checks"
    );
}

/// **A head the Checks passed on is not checked twice.** The marker would turn
/// them red, so landing says they did not run again.
#[tokio::test]
async fn a_head_the_checks_passed_on_lands_without_running_them_again() {
    let home = TempDir::new();
    let fleet = holding_the_work(&home);
    let job_id = at_the_gate_having_delivered(&fleet, &home).await;
    let worktree = worktree_of(&fleet, &home, &job_id).await;
    std::fs::write(worktree.join("red"), "never read").expect("the marker");

    let job = fleet
        .merge_pull_request(&job_id)
        .await
        .expect("the gated head lands as it is");

    assert_eq!(job.status(), JobStatus::CompletedSuccess);
    assert_eq!(fleet.vcs().times_pushed_onto_the_base(), 1);
    assert_eq!(
        counted(&fleet, |it| matches!(
            it,
            Delivered::ReadTheUncheckedHead { .. }
        )),
        0
    );
}

/// **`forge` is as it was**: the sweep's merge goes to the forge, whose own
/// checks are what stand in the path, and Fleet runs none at the press.
#[tokio::test]
async fn under_forge_a_branch_the_sweep_kept_current_merges_through_the_forge() {
    let home = TempDir::new();
    let fleet = holding_the_work_under(&home, &MANIFEST.replace("merge_by: push\n", ""));
    let job_id = at_the_gate_having_delivered(&fleet, &home).await;
    kept_current_by_the_sweep(&fleet).await;
    let worktree = worktree_of(&fleet, &home, &job_id).await;
    std::fs::write(worktree.join("red"), "never read").expect("the marker");
    fleet.vcs().now_landed(Landing::Merged {
        url: String::from(PULL_REQUEST),
    });

    let job = fleet.merge_pull_request(&job_id).await.expect("it merges");

    assert_eq!(job.status(), JobStatus::CompletedSuccess);
    assert_eq!(fleet.vcs().times_asked_to_merge(), 1);
    assert_eq!(fleet.vcs().times_pushed_onto_the_base(), 0);
}
