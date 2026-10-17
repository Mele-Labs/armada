//! The apparatus Board's claim is asserted against, and none of it asserts
//! anything.
//!
//! Separate from `mod.rs` for the reason `focus.rs` is: M1's bench answers
//! "did this Job pass its gates" and Board asks what the record of that answer
//! says to somebody reading it.
//!
//! **Nothing here decides anything.** Every conversion below is `ipc`'s own —
//! `JobSummary::of`, [`JobDetail::of`], `CheckRun::of`, `Judged::of` and the
//! `From` impl over a gaming flag. What this file adds is the *assembly*: which
//! step's rulings go into which [`StepFacts`], which is what
//! `fleet::wire::step_facts` does over a store. **That function is `pub(crate)`
//! and cannot be called from here**, so what is asserted is that a gate's
//! values survive the conversion, not that Fleet hands the right ones in.
//!
//! **The round trip is part of the apparatus.** Every assertion in `board.rs`
//! is made against a value that has been through [`ipc::encode`] and
//! [`ipc::decode`], so a field a `skip_serializing_if` drops on the way out
//! fails there rather than passing here.

use core_model::{Branch, DroneStanding, Job, Refusals, Standing, Stuck, TransitionReason};
use ipc::{JobDelivery, JobDetail, JobList, StepFacts};

use super::Run;

/// Put the Job on the branch its worktree names.
///
/// **What `fleet::dispatch::branded` does, minus the store write.** A Job's
/// branch is written when its worktree is made and not at creation, so a Job
/// that has never been dispatched legitimately has none — which is the absence
/// `board.rs` asserts on the approval-gate row.
pub fn on_its_branch(run: &mut Run) {
    let branch = Branch::new(run.worktree.branch()).expect("a worktree names its branch");
    run.job = run.job.on_branch(branch);
}


/// The four facts a stopped Job is classified against.
///
/// **Every one of them is outside the record**, which is why `Stuck::of` takes
/// them: the slot, the filesystem, the per-step Check rows and Fleet's own
/// workflow set. A Job whose worktree is still there and whose Drone has gone
/// is the ordinary shape at an escalation, and it is what this bench holds.
pub fn standing(checks_passed: bool) -> Standing {
    Standing {
        drone: DroneStanding::Gone,
        worktree_on_disk: true,
        checks_passed,
        workflow_held: true,
        checks_rerunning: false,
    }
}

/// One Job in full, as `get_job` would answer it.
///
/// The six arguments this passes and the six it does not are the whole of what
/// `board.rs` can and cannot prove — see its header. `Stuck` is computed by
/// `core-model` from the Job rather than described here, so nothing in this
/// file can claim a recourse the domain would not offer.
pub fn detail(job: &Job, reason: Option<&TransitionReason>, steps: &[StepFacts]) -> JobDetail {
    let stuck = Stuck::of(job, reason, standing(true), Refusals::none(), None);
    JobDetail::of(
        job,
        reason,
        None,
        None,
        None,
        None,
        None,
        steps,
        None,
        None,
        None,
        stuck.as_ref(),
        None,
        None,
        None,
        None,
    )
}

/// The same, for a Job whose branch went somewhere.
///
/// **A separate function rather than a thirteenth argument to
/// [`detail`].** Every other Job this bench builds has an empty delivery, and
/// threading `None` through four call sites to reach one is how a fixture stops
/// saying what it is for.
pub fn delivered(
    job: &Job,
    reason: Option<&TransitionReason>,
    steps: &[StepFacts],
    delivery: JobDelivery,
) -> JobDetail {
    let stuck = Stuck::of(job, reason, standing(true), Refusals::none(), None);
    JobDetail::of(
        job,
        reason,
        None,
        None,
        None,
        None,
        None,
        steps,
        None,
        None,
        None,
        stuck.as_ref(),
        None,
        Some(delivery),
        None,
        None,
    )
}

/// The value as a Board receives it: encoded, sent, and read back.
pub fn received_detail(detail: &JobDetail) -> JobDetail {
    let body = ipc::encode(detail).expect("a detail that serialises");
    ipc::decode("a Job detail", body.as_bytes()).expect("a detail that reads back")
}

/// The list as a Board receives it, and the bytes it arrived in.
///
/// The bytes are handed back beside the value because absent and
/// present-and-null decode identically: the only way to assert that a Job with
/// no branch omits the field rather than sending `null` is to look at what went
/// over.
pub fn received_list(list: &JobList) -> (JobList, String) {
    let body = ipc::encode(list).expect("a list that serialises");
    let read = ipc::decode("a Job list", body.as_bytes()).expect("a list that reads back");
    (read, body)
}
