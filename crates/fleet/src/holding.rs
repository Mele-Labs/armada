//! What Fleet is holding disk for, and giving back the part that is provably
//! safe to take.
//!
//! # Provably safe, which is not the same as safe
//!
//! Five tests, and a worktree passes all five or none of it is automatic: the
//! Job is terminal, the base already reaches its branch, nothing in its
//! checkout is uncommitted, nobody is piloting it, and no Job still to run
//! depends on it. **Failing one does not make a worktree unsafe** — it makes it
//! not provably safe, a different claim, so every reason is carried on
//! [`Holding::held`] for a person to decide on. That surface is `#385`.
//!
//! # One derivation, shared with the delete
//!
//! `adapters::standing` is the reading, and it is the one `adapters::reclaim`
//! takes on its way to deleting a branch. Nothing here forms its own opinion
//! about what merged means, and nothing matches a path against a pattern:
//! every candidate is a Job the store handed out, reached through the
//! `WorktreeSpec` its record names — a pool slot it still holds, or the path a
//! Job cut before the pool derives. A hand-run `git branch -D` over
//! `armada/*` destroyed nine branches belonging to no Job.
//!
//! # What it took goes on the Job
//!
//! A sweep on a timer nobody reads means the first surprise is a worktree gone
//! with no record of who took it.
use std::sync::Arc;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct, WorktreeSpec};
use adapters::{BranchStanding, Reclaimed, Standing, WorktreeStanding};
use core_model::{
    DependencyDirection, Job, JobId, JobStatus, Timestamp,
};

use crate::daemon::Fleet;
use crate::leasing::reclaimed_spec_in;
use crate::repositories::Served;

/// How often Fleet asks what it could give back.
///
/// A newtype rather than a bare `Duration`, like [`Noticing`], so the value
/// cannot be handed to something measuring a different quantity.
///
/// [`Noticing`]: crate::Noticing
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reclaiming(Duration);

impl Reclaiming {
    pub const fn every(interval: Duration) -> Reclaiming {
        Reclaiming(interval)
    }

    pub const fn interval(&self) -> Duration {
        self.0
    }
}

/// One test a worktree did not pass, and therefore one reason Fleet is still
/// holding it.
///
/// **Five variants for the five tests, plus the two ways the reading itself can
/// fail.** A reading that failed is held for the same reason an unmerged branch
/// is: unanswered and safe must never read alike, because only one of them can
/// be taken back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Held {
    /// The Job is still moving, so it may still need its worktree.
    NotTerminal { status: JobStatus },
    /// **A person is at an unrestricted toolset in it** — `#367`. The worktree
    /// belongs to the engineer, and it is not Fleet's to offer either.
    Piloted,
    /// The branch holds commits the base cannot reach. `commits` of them, and
    /// `tip` is what the branch is recoverable from.
    Unmerged {
        base: String,
        commits: usize,
        tip: String,
    },
    /// Nothing here could say what the branch would be merged into.
    BaseUnanswered { why: String },
    /// Files written and never committed. **The test the other four cannot
    /// make**: uncommitted work leaves a branch level with its base, so every
    /// merged-ness reading says the checkout is disposable.
    Uncommitted { files: Vec<String> },
    /// Somebody locked the checkout, which is a person saying not yet.
    Locked { reason: String },
    /// A Job that depends on this one has not finished, so it may still need
    /// what this one wrote.
    DependedOn { by: Vec<JobId> },
    /// git would not say what is in the checkout.
    Unreadable { why: String },
}

/// One Job's worktree, and every test it failed.
///
/// **An empty `held` is the whole of the safety claim.** A sweep acts on that
/// and nothing else, so a test added to this module is a test the sweep starts
/// applying with no second place to change.
///
/// **The checkout and the branch are carried, not left to be derived again.**
/// [`holding_of`] already holds the `WorktreeSpec` that produced them,
/// and a caller that re-derived a path from a Job id would be the second
/// derivation this module exists to avoid — the one that disagrees the day
/// `WorktreeSpec` changes. `#385` draws both: a path is what a person goes and
/// looks at, and a branch is what the commits are recoverable from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Holding {
    pub job: JobId,
    /// What the Job's worktree and branch are named — its handle. **Read off
    /// the same Job the tests were applied to**, for `title`'s reason and one
    /// more: deriving it later would mean a store read on a path a sweep walks
    /// once per held worktree.
    pub handle: String,
    /// What the Job is called, and where it got to. **Read off the same Job
    /// the tests were applied to**, so a row cannot describe one Job and be
    /// held for another's reasons.
    pub title: String,
    pub status: JobStatus,
    /// When Armada last moved anything on this Job — the latest step move, or
    /// the creation instant where no step has moved.
    ///
    /// **It is not when the files in the checkout were last written**, and
    /// nothing here can be: `git status --porcelain` answers names and not
    /// times. What it is good for is the reading `#385` asks for on uncommitted
    /// work — a checkout whose Job stopped four days ago has been sitting at
    /// least that long, and *twenty minutes* and *four days* are different
    /// decisions.
    pub last_moved: Timestamp,
    /// The checkout on disk, derived once.
    pub path: String,
    /// Whether that checkout is still there. A Job whose checkout is gone and
    /// whose branch is unmerged is still held, and `delete_branch` is its act.
    pub on_disk: bool,
    /// The branch the Job derived. Named even where it is already gone.
    pub branch: String,
    /// Empty where every test passed.
    pub held: Vec<Held>,
}

impl Holding {
    /// Whether Fleet may take this one back without asking anybody.
    pub fn provably_safe(&self) -> bool {
        self.held.is_empty()
    }

    /// Whether a surface may offer this one to a person at all.
    ///
    /// **Piloted is the one that must not be offered**, for `#367`'s reason: a
    /// person is working in it right now, and an act that is drawn is an act
    /// somebody eventually clicks.
    pub fn offerable(&self) -> bool {
        !self.held.contains(&Held::Piloted)
    }
}

/// One Job whose disk came back on this turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GaveBack {
    pub job: JobId,
    pub reclaimed: Reclaimed,
}

/// What one sweep took, over the two kinds of disk Fleet holds.
///
/// **The base checkouts are not Jobs and cannot be reported as ones.** A base
/// checkout belongs to a commit, is shared by every Job on it, and passes none
/// of the five tests below — none of them is a question about it. So it is a
/// second list rather than a sixth reason, and `crate::basing` holds the one
/// test it does have: the base has moved past this commit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Swept {
    pub jobs: Vec<GaveBack>,
    /// The commits whose shared base checkouts were removed.
    pub bases: Vec<String>,
}

/// Where one Job's worktree is and what git says is in it, or `None` where
/// there is nothing to give back. **The blocking half of a holding**: it
/// reads the pool and the checkout.
fn standing_of<V: Vcs>(
    vcs: &V,
    served: &Served,
    job: &Job,
) -> Option<(WorktreeSpec, Standing)> {
    let spec = reclaimed_spec_in(vcs, served, job).ok()?;
    let stands = adapters::standing(&spec, served.manifest().base()).ok()?;
    (!stands.empty_handed()).then_some((spec, stands))
}

/// Every test one Job's worktree failed.
///
/// **The status tests come before the git ones and the git reading is taken
/// anyway**, because a person looking at a piloted Job's worktree wants to
/// know what is in it. What the order buys is that a Job whose worktree git
/// cannot be asked about still reports the reasons that do not need git.
fn holding_of(
    job: &Job,
    board: &[&Job],
    (spec, stands): (WorktreeSpec, Standing),
) -> Option<Holding> {
    let mut held = Vec::new();
    match job.status() {
        JobStatus::Piloted => held.push(Held::Piloted),
        status if !status.is_terminal() => held.push(Held::NotTerminal { status }),
        _ => {}
    }
    match &stands.branch {
        BranchStanding::Ahead { base, commits, tip } => held.push(Held::Unmerged {
            base: base.clone(),
            commits: *commits,
            tip: tip.clone(),
        }),
        BranchStanding::Unanswered { why, .. } => {
            held.push(Held::BaseUnanswered { why: why.clone() })
        }
        BranchStanding::Absent | BranchStanding::Merged { .. } => {}
    }
    match &stands.worktree {
        WorktreeStanding::Dirty { files } => held.push(Held::Uncommitted {
            files: files.clone(),
        }),
        WorktreeStanding::Locked { reason } => held.push(Held::Locked {
            reason: reason.clone(),
        }),
        WorktreeStanding::Unreadable { why } => {
            held.push(Held::Unreadable { why: why.clone() })
        }
        WorktreeStanding::Absent | WorktreeStanding::Clean => {}
    }
    let waiting = dependents_still_running(job.id(), board);
    if !waiting.is_empty() {
        held.push(Held::DependedOn { by: waiting });
    }
    Some(Holding {
        handle: job.handle(),
        job: job.id().clone(),
        title: job.title().as_str().to_string(),
        status: job.status(),
        last_moved: last_moved(job),
        path: spec.worktree_path(),
        on_disk: stands.worktree != WorktreeStanding::Absent,
        branch: spec.branch(),
        held,
    })
}

/// When Armada last moved anything on this Job.
///
/// **The latest step move, and `created_at` where no step has one.** There is
/// no `updated_at` on a Job — the field lives on the step rows, which is where
/// every move is written — so the Job's own answer is the maximum over them.
/// A Job with no steps has never moved, and its creation instant is the only
/// true thing left to say.
fn last_moved(job: &Job) -> Timestamp {
    job.steps()
        .iter()
        .map(|step| step.updated_at())
        .max()
        .unwrap_or_else(|| job.created_at())
        .clone()
}

/// The Jobs that named this one and have not finished.
///
/// **`depends_on` in the reverse direction**, which is the one direction
/// `crate::coupling` does not walk: it asks what one Job is waiting on, and
/// this asks who is waiting on it. A dependent that has itself reached a
/// terminal will never run again and needs nothing; one that has not may still
/// need what this Job wrote, and the work is on disk rather than in the record.
fn dependents_still_running(job: &JobId, board: &[&Job]) -> Vec<JobId> {
    board
        .iter()
        .filter(|other| !other.status().is_terminal())
        .filter(|other| {
            other
                .dependencies()
                .iter()
                .any(|edge| edge.direction == DependencyDirection::DependsOn && &edge.peer == job)
        })
        .map(|other| other.id().clone())
        .collect()
}

/// What became of the branch. **The tip is carried on every arm that has one**,
/// because a deleted branch is recoverable from its SHA and from nothing else.
pub(crate) fn said_of_branch(branch: &adapters::BranchGone) -> String {
    use adapters::BranchGone::*;
    match branch {
        Deleted { branch, tip } => format!("deleted {branch} at {tip}"),
        Absent { branch } => format!("no branch {branch}"),
        Kept {
            branch,
            tip,
            base,
            commits,
        } => format!("kept {branch} at {tip}: {commits} commits {base} cannot reach"),
        KeptUnanswered { branch, tip, why } => format!("kept {branch} at {tip}: {why}"),
        NotDeleted { branch, why } => format!("{branch} was not deleted: {why}"),
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// [`worktrees_held`](Fleet::worktrees_held) over Jobs the caller already
    /// loaded, so a caller asking for more than this reads the store once.
    ///
    /// **The git half runs off the runtime and side by side.** Each Job's
    /// checkout is its own handful of processes and none depends on another,
    /// so asking costs the slowest one rather than the sum.
    pub(crate) async fn held_among(&self, jobs: Arc<Vec<Job>>) -> Vec<Holding> {
        let mine: Vec<(usize, Served)> = jobs
            .iter()
            .enumerate()
            // **This repository's Jobs only.** One store serves the machine and
            // a `WorktreeSpec` is derived from *this* Fleet's root, so another
            // Manifest's Job would name a path under a repository it never ran
            // in. `armada clean` selects on the same column for the same reason.
            .filter_map(|(at, job)| Some((at, self.served_by(job).ok()?)))
            .collect();
        let board: Vec<&Job> = mine.iter().map(|(at, _)| &jobs[*at]).collect();
        let (vcs, asked) = (Arc::clone(self.vcs()), Arc::clone(&jobs));
        let read = tokio::task::spawn_blocking(move || {
            adapters::concurrently(&mine, |(at, served)| {
                standing_of(vcs.as_ref(), served, &asked[*at])
            })
        })
        .await
        .expect("git panicked reading the worktrees held");
        board
            .iter()
            .zip(read)
            .filter_map(|(job, read)| holding_of(job, &board, read?))
            .collect()
    }
}
