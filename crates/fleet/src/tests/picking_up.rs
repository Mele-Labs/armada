//! Pick up on a Finding: the slot's work is stashed, the slot is freed, and a
//! Job that continues from the slot's branch is proposed. The stash itself is
//! `rescuing`'s; the proposer is a stand-in model, as in `proposing`.

use std::sync::Arc;
use std::time::Duration;

use core_model::Job;
use ipc::RescueAct;
use store::{KeptRescue, RescueState, RescueVerdict};
use testkit::{FakeJudge, FakeWorkProduct};

use super::rescuing::{code, ended, on_the_wire, press, rescuing_on, root, stranded, Rescued};
use crate::tests::daemon::{a_proposal_for, fittings};
use crate::tests::proposing::a_catalogue;
use crate::tests::tmp::TempDir;

const A_BRANCH: &str = "fleet/half-done";

/// A Fleet whose Scouts are the stand-in and whose proposer answers `bug`.
fn picking(home: &TempDir) -> Arc<Rescued> {
    let mut fitted = fittings(home, FakeWorkProduct::changed(&[]));
    fitted.starting().workflows = a_catalogue()
        .into_iter()
        .map(|workflow| (workflow.id().clone(), workflow))
        .collect();
    fitted.judge = Arc::new(FakeJudge::saying(
        "workflow: bug\ntitle: Finish the parser\nbecause: work left half done",
    ));
    Arc::new(rescuing_on(home, fitted))
}

/// Slot 2 stranded on the branch, read, and its Finding answered.
async fn read(home: &TempDir, fleet: &Arc<Rescued>) {
    stranded(home, fleet, Some(A_BRANCH));
    Arc::clone(fleet)
        .rescue_slot(press(RescueAct::Start), None)
        .await
        .expect("started");
    ended(fleet).await;
}

async fn jobs(fleet: &Rescued) -> Vec<Job> {
    fleet
        .store()
        .lock()
        .await
        .load_all_jobs()
        .expect("read")
        .jobs
}

/// The Job a pick up proposed, once its proposer has answered.
async fn proposed(fleet: &Rescued) -> Job {
    for _ in 0..1000 {
        if let Some(job) = jobs(fleet)
            .await
            .into_iter()
            .find(|job| job.status() == core_model::JobStatus::AwaitingApproval)
        {
            return job;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("no Job was proposed");
}

/// **Pick up stashes the work, frees the slot and proposes a Job cut from the
/// branch**, whose request is the Finding's items as bare facts. The Job waits
/// at the approval gate, like any other.
#[tokio::test]
async fn a_pick_up_frees_the_slot_and_proposes_a_job_from_its_branch() {
    let home = TempDir::new();
    let fleet = picking(&home);
    read(&home, &fleet).await;

    let answered = Arc::clone(&fleet)
        .rescue_slot(press(RescueAct::PickUp), None)
        .await
        .expect("picked up");
    assert_eq!(answered.branch.as_deref(), Some(A_BRANCH));
    assert!(
        answered.committed.is_some(),
        "the uncommitted file was committed"
    );
    assert!(matches!(
        fleet.vcs().rescued_slots().as_slice(),
        [(2, adapter_traits::SlotRescue::Stash { .. })]
    ));
    let slot = on_the_wire(&fleet).await;
    assert!(matches!(slot.held, ipc::SlotHolding::Free));
    assert_eq!(slot.rescue, None, "the Finding goes with the slot's work");

    let job = proposed(&fleet).await;
    assert_eq!(job.title().as_str(), "Finish the parser");
    assert_eq!(
        job.facts().as_str(),
        "Continue the work on branch fleet/half-done.\n\nLeft to do:\n\
         - src/parser.rs has no parse_expr\n- The lexer test is not written"
    );
    let landing = fleet.landing_of(job.id()).await;
    assert_eq!(
        landing.from_ref.as_ref().map(|branch| branch.as_str()),
        Some(A_BRANCH)
    );
    assert_eq!(landing.target, None, "it still lands in the base");
}

/// A kept Job's slot is picked up like a stranded one, and ends the Job's claim.
#[tokio::test]
async fn a_pick_up_of_a_kept_jobs_slot_frees_it() {
    let home = TempDir::new();
    let fleet = picking(&home);
    fleet.vcs().keep_slot(
        &root(&home),
        2,
        "01JOBKILLED",
        super::rescuing::work(Some("armada/killed")),
        "+fn parse() {}\n",
    );
    let src = std::path::PathBuf::from(adapter_traits::slot_path(&root(&home), 2)).join("src");
    std::fs::create_dir_all(&src).expect("a source directory");
    std::fs::write(src.join("parser.rs"), "fn parse() {}\n").expect("written");
    std::fs::write(src.join("lexer.rs"), "fn lex() {}\n").expect("written");
    Arc::clone(&fleet)
        .rescue_slot(press(RescueAct::Start), None)
        .await
        .expect("started");
    ended(&fleet).await;

    Arc::clone(&fleet)
        .rescue_slot(press(RescueAct::PickUp), None)
        .await
        .expect("picked up");
    assert!(matches!(
        on_the_wire(&fleet).await.held,
        ipc::SlotHolding::Free
    ));
    let job = proposed(&fleet).await;
    let landing = fleet.landing_of(job.id()).await;
    assert_eq!(
        landing.from_ref.as_ref().map(|branch| branch.as_str()),
        Some("armada/killed")
    );
}

/// Every refusal leaves the slot as it was and proposes nothing.
#[tokio::test]
async fn a_refused_pick_up_changes_nothing() {
    let home = TempDir::new();
    let fleet = picking(&home);

    // No Scout has read it.
    stranded(&home, &fleet, Some(A_BRANCH));
    let refused = Arc::clone(&fleet)
        .rescue_slot(press(RescueAct::PickUp), None)
        .await
        .expect_err("nothing read");
    assert_eq!(code(&refused), "fleet.rescue_unread");

    // Leftovers need no more work.
    let manifest = fleet.pool_slots().await.expect("read")[0].manifest.clone();
    let mut finding = KeptRescue {
        manifest_id: manifest,
        slot: 2,
        state: RescueState::Answered,
        commit: String::from("abc1234def"),
        uncommitted: true,
        cut: 0,
        read: Vec::new(),
        searched: Vec::new(),
        verdict: Some(RescueVerdict::Scraps),
        items: vec![String::from("A scratch file")],
        summary: None,
        why: None,
        cost_micros: None,
    };
    fleet
        .store()
        .lock()
        .await
        .keep_rescue(&finding)
        .expect("kept");
    let refused = Arc::clone(&fleet)
        .rescue_slot(press(RescueAct::PickUp), None)
        .await
        .expect_err("scraps");
    assert_eq!(code(&refused), "fleet.rescue_nothing_left");

    // Unfinished, on no branch: the stash refuses, and nothing is proposed.
    finding.verdict = Some(RescueVerdict::Unfinished);
    fleet
        .store()
        .lock()
        .await
        .keep_rescue(&finding)
        .expect("kept");
    stranded(&home, &fleet, None);
    let refused = Arc::clone(&fleet)
        .rescue_slot(press(RescueAct::PickUp), None)
        .await
        .expect_err("no branch");
    assert_eq!(code(&refused), "fleet.rescue_on_no_branch");

    assert!(fleet.vcs().rescued_slots().is_empty());
    assert!(matches!(
        on_the_wire(&fleet).await.held,
        ipc::SlotHolding::Stranded { .. }
    ));
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(jobs(&fleet).await.is_empty());
}

/// **A proposal that names no branch is the proposal it always was**: no
/// landing is kept, so it is cut from the base. One that names a branch keeps
/// it as where the worktree starts, and a blank one is none.
#[tokio::test]
async fn a_proposal_continues_from_a_branch_only_where_it_names_one() {
    let home = TempDir::new();
    let fleet = picking(&home);

    let plain = fleet
        .propose(a_proposal_for("Plain", "bug"))
        .await
        .expect("proposed");
    assert_eq!(
        fleet.landing_of(plain.id()).await,
        core_model::Landing::as_ever()
    );

    let mut blank = a_proposal_for("Blank", "bug");
    blank.continue_from = Some(String::from("  "));
    let blank = fleet.propose(blank).await.expect("proposed");
    assert_eq!(
        fleet.landing_of(blank.id()).await,
        core_model::Landing::as_ever()
    );

    let mut named = a_proposal_for("Named", "bug");
    named.continue_from = Some(String::from("fleet/half-done"));
    let named = fleet.propose(named).await.expect("proposed");
    let landing = fleet.landing_of(named.id()).await;
    assert_eq!(
        landing.from_ref.as_ref().map(|branch| branch.as_str()),
        Some("fleet/half-done")
    );
    assert_eq!(landing.target, None);
}
