//! An Epic's wave, past what `tests::epic` drives: corrected and dropped before
//! the press, released only whole, withdrawn when the plan runs again, and a
//! parent that finishes on its members' merges waiting for them. Spike 022,
//! slice 6.

use std::time::Duration;

use adapter_traits::Landing as Settled;
use api::Queries;
use core_model::{CompleteWhen, JobId, JobStatus, Landing};
use testkit::{FakeHarness, FakeJudge, FakeVcs, FakeWorkProduct};

use crate::daemon::Fleet;
use crate::noticing::Noticing;
use crate::tests::daemon::{fittings, worktree_directory};
use crate::tests::epic::{
    a_wave_of_one, asking, document, epic, planning, planning_a_wave, presented_the_plan,
    released_the_wave, rolling_up, said, the_wave_ran, wrote,
};
use crate::tests::tmp::TempDir;
use crate::tests::tools::submitted_by_the_one;

type Fixture = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

/// A member's pull request, as the fake forge opens it.
const PULL_REQUEST: &str = "https://forge.invalid/armada/pull/1";

async fn finishes_on_merges(fleet: &Fixture, job: &JobId) {
    let landing = Landing {
        complete_when: CompleteWhen::AllMembersLanded,
        ..Landing::as_ever()
    };
    fleet
        .store()
        .lock()
        .await
        .set_landing(job, &landing)
        .expect("kept");
}

/// **Corrected and dropped before the press.** An edit saves without
/// releasing, a dropped member leaves the wave, a press naming it is refused
/// whole, and the one that releases what is held releases the member as edited.
#[tokio::test]
async fn a_member_edited_before_the_press_is_released_as_edited_and_a_dropped_one_is_not() {
    let home = TempDir::new();
    let (fleet, job) = planning_a_wave(&home).await;
    let kept = fleet
        .sub_dispatch(&job, &asking("port the parser"))
        .await
        .expect("proposed");
    let dropped = fleet
        .sub_dispatch(&job, &asking("port the printer"))
        .await
        .expect("proposed");
    worktree_directory(&home, &fleet.load(&kept).await.expect("the child"));

    let edit: ipc::EditJob = ipc::decode("an edit", br#"{"title": "Port the parser whole"}"#)
        .expect("Edit this Job's body");
    let edited = fleet.edit_proposal(&kept, &edit).await.expect("saved");
    assert_eq!(
        edited.status(),
        JobStatus::AwaitingApproval,
        "saving is not releasing"
    );
    fleet.kill_job(&dropped).await.expect("Hold to drop");
    presented_the_plan(&fleet, &job).await;

    let refused = fleet
        .approve_wave(
            &job,
            &ipc::ApproveWave {
                jobs: vec![ipc::JobId::from(&kept), ipc::JobId::from(&dropped)],
            },
        )
        .await
        .expect_err("the dropped Job is not in the wave");
    assert!(refused.to_string().contains(dropped.as_str()), "{refused}");
    assert_eq!(
        fleet.load(&kept).await.expect("read").status(),
        JobStatus::AwaitingApproval,
        "a refused press moves nothing"
    );

    released_the_wave(&fleet, &job, &[&kept]).await;
    let released = fleet.load(&kept).await.expect("read");
    assert_eq!(released.status(), JobStatus::Queued);
    assert_eq!(released.title().as_str(), "Port the parser whole");
    assert_eq!(
        fleet.load(&dropped).await.expect("read").status(),
        JobStatus::Killed
    );
}

/// **One member is released with its wave**, never alone, whether the press
/// carries a body or not.
#[tokio::test]
async fn approving_one_member_alone_is_refused() {
    let home = TempDir::new();
    let (fleet, job) = planning_a_wave(&home).await;
    let child = fleet
        .sub_dispatch(&job, &asking("port the parser"))
        .await
        .expect("proposed");

    let alone = fleet
        .approve(&child)
        .await
        .expect_err("released with its wave");
    assert!(alone.to_string().contains("released whole"), "{alone}");
    let as_left = fleet
        .approve_as_left(&child, &ipc::ApproveDispatch::default())
        .await
        .expect_err("with a body too");
    assert!(as_left.to_string().contains("released whole"), "{as_left}");
    assert_eq!(
        fleet.load(&child).await.expect("read").status(),
        JobStatus::AwaitingApproval
    );
}

/// **A plan sent back proposes again**, and what it proposed the first time
/// and nobody released is withdrawn before its Drone starts, so the gate never
/// holds two copies of one wave.
#[tokio::test]
async fn a_plan_that_runs_again_withdraws_what_it_proposed_and_nobody_released() {
    let home = TempDir::new();
    let (fleet, job) = planning_a_wave(&home).await;
    let first = fleet
        .sub_dispatch(&job, &asking("port the parser"))
        .await
        .expect("proposed");
    presented_the_plan(&fleet, &job).await;

    fleet
        .request_changes(&job, &said("split the parser in two"))
        .await
        .expect("the plan is sent back");
    fleet.turn().await.expect("a Drone is put back on the plan");

    let standing = fleet.load(&job).await.expect("read");
    assert_eq!(standing.current_step_id(), Some(&planning()));
    assert_eq!(
        fleet.load(&first).await.expect("read").status(),
        JobStatus::Killed,
        "withdrawn by Fleet before the plan proposes again"
    );
}

/// **A parent that finishes on its members' merges waits for them**, and the
/// merge's instant is read off the forge and served beside it.
#[tokio::test]
async fn a_parent_finishing_on_merges_waits_until_its_members_pull_request_merged() {
    let home = TempDir::new();
    let mut fitted = fittings(&home, FakeWorkProduct::changed(&["docs/plan.md"]));
    fitted.starting().workflows = [epic(), crate::tests::epic::a_piece()]
        .into_iter()
        .map(|workflow| (workflow.id().clone(), workflow))
        .collect();
    fitted.judge = std::sync::Arc::new(FakeJudge::with_no_objection());
    fitted.noticing = Noticing::every(Duration::ZERO);
    let fleet: Fixture = Fleet::assembled(fitted);
    let job = fleet
        .propose(crate::tests::daemon::a_proposal_for(
            "run the milestone",
            "epic",
        ))
        .await
        .expect("a Job at the approval gate");
    let id = job.id().clone();
    worktree_directory(&home, &job);
    wrote(
        &home,
        &job,
        ".armada/artifacts/plan.md",
        "# Wave 1\n\nOne piece.\n",
    );
    crate::tests::admitted::dispatched(&fleet, &id)
        .await
        .expect("it dispatches");
    finishes_on_merges(&fleet, &id).await;

    let child = a_wave_of_one(&fleet, &id, &home).await;
    fleet.turn().await.expect("the child is admitted");
    submitted_by_the_one(&fleet, crate::tests::epic::note("The piece is done."))
        .await
        .expect("the child reports");
    fleet
        .turn()
        .await
        .expect("the child finishes and opens its pull request");
    assert_eq!(
        fleet.load(&child).await.expect("read").status(),
        JobStatus::CompletedSuccess
    );
    fleet.turn().await.expect("a turn passes");
    let held = fleet.load(&id).await.expect("read");
    assert_eq!(
        held.status(),
        JobStatus::Queued,
        "its member finished and has not landed, so the roll-up waits"
    );

    fleet.vcs().now_merged_at(
        Settled::Merged {
            url: PULL_REQUEST.to_string(),
        },
        "2026-10-03T09:00:00Z",
    );
    fleet.turn().await.expect("the sweep reads the merge");
    let row = fleet
        .get_job(ipc::JobId::from(&child))
        .await
        .expect("the member reads");
    assert_eq!(
        row.delivery
            .and_then(|delivery| delivery.merged_at)
            .map(|at| at.as_str().to_string()),
        Some("2026-10-03T09:00:00Z".to_string()),
        "when it merged, as the forge said"
    );
    fleet.turn().await.expect("the parent is admitted");
    let back = fleet.load(&id).await.expect("read");
    assert_eq!(back.current_step_id(), Some(&rolling_up()));
}

/// **The last approval waits for every member to land.** A parent set to
/// finish on its members' merges is refused at its roll-up while one has not
/// merged — here set after the wave ran, so the roll-up is reached first.
#[tokio::test]
async fn a_parent_finishing_on_merges_is_not_finished_while_a_member_is_unmerged() {
    let home = TempDir::new();
    let (fleet, job) = planning_a_wave(&home).await;
    let child = a_wave_of_one(&fleet, &job, &home).await;
    the_wave_ran(&fleet).await;
    wrote(
        &home,
        &fleet.load(&job).await.expect("the Job"),
        ".armada/artifacts/roll-up.md",
        "# Wave 1\n\nOne Job.\n",
    );
    submitted_by_the_one(&fleet, document("What the wave did."))
        .await
        .expect("the roll-up is reported");
    fleet.turn().await.expect("the human gate runs");
    finishes_on_merges(&fleet, &job).await;

    let refused = fleet
        .approve_review(&job)
        .await
        .expect_err("its member has not merged");
    assert!(refused.to_string().contains(child.as_str()), "{refused}");
    assert_eq!(
        fleet.load(&job).await.expect("read").status(),
        JobStatus::AwaitingReview
    );
}
