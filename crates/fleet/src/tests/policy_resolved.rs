//! What a gate's policies resolved to, kept on the run that passed it. #1683.
//!
//! **The file is real and re-read**, `tests::freezing`'s way, because the claim
//! is about a policy that moves under a running Fleet: the earlier run keeps
//! what it was gated under, and the next run reads the new words.

use std::path::PathBuf;

use config::{Manifest, Reloads};
use core_model::JobId;
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::gate::Ruling;
use crate::tests::admitted::{dispatched, started};
use crate::tests::daemon::{
    a_proposal, diff_evidence, fittings, one, two_steps_gated_on_a_manifest_rule,
    worktree_directory,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const HEAD: &str = "version: 1\nid: 01FIXTUREMANIFEST\n";

/// A Fleet gating `implement` on `manifest_rule:review_gate`, over an
/// `armada.yml` on disk, and the handle that re-reads it.
fn over(home: &TempDir, says: &str) -> (Fixture, PathBuf, Reloads) {
    let dir = home.path().join("manifest");
    std::fs::create_dir_all(&dir).expect("a directory for the file");
    let file = dir.join("armada.yml");
    std::fs::write(&file, format!("{HEAD}{says}")).expect("the file");
    let (manifest, reloads) = Manifest::reloadable(&file).expect("the file loads");
    let mut fittings = fittings(home, FakeWorkProduct::changed(&["src/log.rs"]));
    fittings.starting().manifest = manifest;
    fittings.starting().workflows = one(two_steps_gated_on_a_manifest_rule(
        "implement",
        "review_gate",
        None,
    ));
    (Fleet::assembled(fittings), file, reloads)
}

/// `implement`'s runs as `get_job` serves them: each run's ordinal and what
/// its gate resolved to, as the two words cross.
async fn served(fleet: &Fixture, job: &JobId) -> Vec<(u32, Option<(String, String)>)> {
    let detail = api::Queries::get_job(fleet, ipc::JobId::from(job))
        .await
        .expect("a Job that exists");
    detail
        .steps
        .iter()
        .find(|step| step.step_id.as_str() == "implement")
        .expect("the step is on the wire")
        .attempts
        .iter()
        .map(|run| {
            (
                run.attempt,
                run.resolved
                    .as_ref()
                    .map(|was| (was.auto_merge.clone(), was.review_gate.clone())),
            )
        })
        .collect()
}

fn words(auto_merge: &str, review_gate: &str) -> Option<(String, String)> {
    Some((auto_merge.to_string(), review_gate.to_string()))
}

/// **The earlier run keeps what it was gated under, and the next run reads the
/// new words.** Both policies are recorded, not only `review_gate`, which is
/// the one that gated: the owner's decision of 1 Oct 2026.
///
/// The second save moves both, and `review_gate: auto_if_judge_passes` still
/// holds this judgeless step, so both runs reach the gate and stop there.
#[tokio::test]
async fn each_run_keeps_what_its_gate_resolved_to_after_the_manifest_moves() {
    let home = TempDir::new();
    let (fleet, file, reloads) = over(
        &home,
        "review_gate: human_always\nauto_merge: checks-pass\n",
    );
    let job = fleet
        .propose(a_proposal("fix the off-by-one"))
        .await
        .expect("a Job at the approval gate");
    worktree_directory(&home, &job);
    dispatched(&fleet, job.id()).await.expect("it dispatches");
    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the Drone reports its diff");
    let turned = fleet.turn().await.expect("the gate runs");
    assert!(
        matches!(turned.ruled(), Some(Ruling::HeldForReview { .. })),
        "human_always held the step: {:?}",
        turned.ruled()
    );
    assert_eq!(
        served(&fleet, job.id()).await,
        vec![(1, words("checks-pass", "human_always"))],
        "the run that passed the gate says what both policies resolved to"
    );

    std::fs::write(
        &file,
        format!("{HEAD}review_gate: auto_if_judge_passes\nauto_merge: always\n"),
    )
    .expect("the save");
    reloads.reread().expect("the save loads");
    fleet
        .request_changes(
            job.id(),
            &crate::resume::Redirection::saying("once more").expect("a note"),
        )
        .await
        .expect("sent back from the gate");
    started(&fleet, job.id())
        .await
        .expect("a fresh Drone on the same step");

    assert_eq!(
        served(&fleet, job.id()).await,
        vec![(1, words("checks-pass", "human_always")), (2, None)],
        "a run that has not reached its gate has resolved nothing, and reads absent"
    );

    submitted_by_the_one(&fleet, diff_evidence())
        .await
        .expect("the next Drone reports its diff");
    let turned = fleet.turn().await.expect("the gate runs again");
    assert!(
        matches!(turned.ruled(), Some(Ruling::HeldForReview { .. })),
        "a judgeless step holds however the policy resolved: {:?}",
        turned.ruled()
    );
    assert_eq!(
        served(&fleet, job.id()).await,
        vec![
            (1, words("checks-pass", "human_always")),
            (2, words("always", "auto_if_judge_passes")),
        ],
        "the first run still reads what it resolved to, and the second the new words"
    );
}
