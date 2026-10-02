//! `merge_by: push` over a base that moved: Fleet brings the branch up, gates
//! it again and pushes once more, the way `armada land` does.
//!
//! The pushes and the merge are scripted, as in `merging_by_push`; what `git`
//! does is asserted in `adapters`. The Checks are real commands, so a red is a
//! command that exited non-zero rather than a row the test wrote.

use std::path::{Path, PathBuf};
use std::time::Duration;

use adapter_traits::{NotMerged, WorktreeSpec};
use adapters::onto_base::ROUNDS;
use config::{Manifest, ResolvedWorkflow, Roster, WorkflowDef};
use core_model::{JobId, JobStatus};
use testkit::{Delivered, FakeWorkProduct, Merging};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::noticing::Noticing;
use crate::tests::daemon::{fittings, one};
use crate::tests::merging::{at_the_gate_having_delivered, Fixture};
use crate::tests::tmp::TempDir;

/// `suite` is red exactly where a file named `red` stands in the worktree, so
/// a case turns the merged tree red by writing one after the first gate.
const MANIFEST: &str = "version: 1\nid: 01FIXTUREMANIFEST\nmerge_by: push\n\
                        checks:\n  suite:\n    run: \"test ! -e red\"\n";

/// Implement gated on every Manifest Check, then a hand-off held for a person
/// that sends the work out and declares none — every shipped coding
/// workflow's shape.
fn workflow(manifest: &Manifest) -> ResolvedWorkflow {
    let def = WorkflowDef::parse(
        Path::new("fixture.yml"),
        "version: 1\nworkflow_id: fixture-workflow\nname: fixture\nstructure: linear\n\
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
    let manifest = Manifest::parse(Path::new("armada.yml"), MANIFEST).expect("a manifest");
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.starting().workflows = one(workflow(&manifest));
    fittings.starting().manifest = manifest;
    fittings.noticing = Noticing::every(Duration::ZERO);
    Fleet::assembled(fittings)
}

async fn worktree_of(fleet: &Fixture, home: &TempDir, job_id: &JobId) -> PathBuf {
    let handle = fleet.load(job_id).await.expect("the Job").handle();
    let spec = WorktreeSpec::for_job(&home.path().to_string_lossy(), &handle).expect("a spec");
    PathBuf::from(spec.worktree_path())
}

fn moved() -> Merging {
    Merging::Refuses(NotMerged::BaseMoved {
        said: String::from("main moved on origin between the fetch and the push"),
    })
}

fn counted(fleet: &Fixture, which: impl Fn(&Delivered) -> bool) -> usize {
    fleet
        .vcs()
        .delivered()
        .iter()
        .filter(|it| which(it))
        .count()
}

fn wire_code(fleet: &Fixture, refused: Adrift) -> (String, String) {
    match fleet.refusal(refused) {
        api::Refusal::IllegalMove(wire) => (wire.code.to_string(), wire.message.to_string()),
        other => panic!("the person's to answer, so a conflict: {other:?}"),
    }
}

async fn still_at_the_gate(fleet: &Fixture, job_id: &JobId) {
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
