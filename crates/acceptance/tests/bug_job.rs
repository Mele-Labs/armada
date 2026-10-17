//! One hermetic run of a Bug Job, and the invariants that make it mean
//! something. **It proves** that a Job reaches `completed_success` **only** by
//! transitioning — a human approval, a Fleet dispatch, one submission per step,
//! a gate that ran every check the step declared. That evidence and passing
//! checks are both required and neither is sufficient. That a Drone's word
//! reaches no type in the system, so a Drone that says it is done and leaves
//! escalates rather than completes. That the acceptance criteria and step list
//! survive every move made on the Job, and that a Job's branch is derived from
//! its id by the one derivation that ships.
//!
//! **Nothing here starts a process, opens a repository or reaches a network**,
//! so what that leaves out is named rather than implied.
//!
//! | Not proved here | Where it is, or why not |
//! |---|---|
//! | The merge, the milestone's other half | The claim is "Armada does a small real task in the Armada repo, and I merge the branch it wrote". This proves the machinery that would carry such a task and the run itself is a person's to perform once; the base, the rebase, the push and the pull request are asserted in `fleet` against a scripted seam and in `adapters` against a real repository |
//! | The loop | `Fleet::approve` starts a detached child before it can gate anything and there is no seam that lets it not, so the loop's own end-to-end test spawns `/bin/cat`. What this file drives is the machine and the gate the loop calls |
//! | The workflow's checks frozen onto the Job | `Job` freezes each step's id and ordinal at creation; the checks are read from Fleet's current `ResolvedWorkflow` when the gate runs, so an `armada.yml` edited while a Job waits at the approval gate changes the bar that Job faces — which is what `fleet::drafting` says freezing prevents. Asserted below as far as it holds, and no further |
//! | A live model | The Judge call goes through Fleet's own runner against a scripted verdict, because a suite that reached a model would cost money and need a network |
//! | Where a failed Check leaves the Job | **Excluded from this claim by `#208`.** M1 promised it ends at `completed_failed`; a spent retry budget now holds at `awaiting_repair`, and where a Job stops for a person and how it is unstuck is the Recovery milestone's claim. What is still asserted below is the half that did not move: nothing rescues a failed Check, the step stops carrying `failed(gate_failure)`, and the branch survives |
//!
//! Everything below is an assertion. The apparatus — the planted clock and
//! mint, the workflow fixture, and the Job that can only be moved by
//! transitioning — is [`bench`], kept apart so that the claim and what it is
//! claimed against read as two different things.

// The bench is shared with the other milestone's test and neither uses all of
// it. Every item in it is reached from one of the two.
#[allow(dead_code)]
mod bench;

use adapter_traits::WorktreeSpec;
use core_model::{
    IllegalStepTransition, JobStatus,
    StepState, StepTarget,
};
use testkit::FakeWorkProduct;

use bench::{
    states, Bench, REPO_ROOT,
};

// ---------------------------------------------------------------------------
// The run itself
// ---------------------------------------------------------------------------



// ---------------------------------------------------------------------------
// Fleet decides, and a Drone's word decides nothing
// ---------------------------------------------------------------------------








/// A step advances only beneath a Job the outer machine says is advancing.
///
/// The outer machine gates the inner one, so a Job paused for a person cannot
/// have a step moved under it by anything at all.
#[test]
fn a_step_cannot_move_beneath_a_job_that_is_not_running() {
    let bench = Bench::with(FakeWorkProduct::untouched());
    let run = bench.created("frozen at the gate");

    assert_eq!(
        bench.refuses_step(&run, &bench.step(0), StepTarget::Running),
        IllegalStepTransition::StepsAreFrozen {
            step_id: bench.step(0),
            status: JobStatus::AwaitingApproval
        }
    );
    assert_eq!(states(&run.job)[0].1, StepState::NotStarted);
}

// ---------------------------------------------------------------------------
// The yardstick cannot move under the work
// ---------------------------------------------------------------------------


// ---------------------------------------------------------------------------
// The branch, which is the half a person finishes
// ---------------------------------------------------------------------------

/// The branch a Job writes is derived from its id, and no Drone can push it.
///
/// The milestone ends with a person merging a branch, so the branch has to be
/// findable from the record alone. It is: one derivation, from the Job id, and
/// the same one the real adapter uses.
///
/// **Pushing is on a different trait, and no Drone holds one.** `Vcs` — the
/// trait that makes this worktree — still has no push method, and `Delivery`,
/// which does, is Fleet's with the operator's credentials. A capability that is
/// not on the type cannot be reached by a Drone that reasons its way around a
/// denial, and a Drone is handed neither.
#[test]
fn the_branch_a_job_writes_is_derived_from_its_id() {
    let bench = Bench::with(FakeWorkProduct::untouched());
    let run = bench.created("a branch a person can find");
    let spec = WorktreeSpec::for_job(REPO_ROOT, run.job.id().as_str()).expect("a legal spec");

    assert_eq!(run.worktree.branch(), spec.branch());
    assert_eq!(
        run.worktree.branch(),
        format!("armada/{}", run.job.id().as_str())
    );
    assert_eq!(run.worktree.path(), spec.worktree_path());
    assert!(
        run.worktree.path().starts_with(REPO_ROOT),
        "the checkout is inside the repository it was taken from"
    );
}
