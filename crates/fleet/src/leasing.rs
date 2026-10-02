//! A Job's worktree is the pool slot it leased: leased at dispatch, recorded
//! on the Job, looked up after, and given back when the Job ends.
//! `docs/concepts/fleet.md`, *Worktree slots*.
//!
//! **Looked up, and asked whether it is still the Job's.** A slot is reused,
//! so the record says where the Job's worktree was and the pool's record
//! beside the slot says whether it still is. A Job cut before the pool has no
//! slot recorded and keeps the path its handle derives.

use adapter_traits::{
    AgentHarness, Delivery, SlotKept, SlotPool, SlotStanding, Vcs, WorkProduct, WorktreeSpec,
    WorktreeSpecRefused,
};
use core_model::{Component, Envelope, FieldValue, Job, Level};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::repositories::Served;

/// The base a Manifest that names none is leased from, as `armada worktree`
/// leases it.
const BASE_UNSTATED: &str = "main";

/// The served repository's pool, as its root Manifest sizes it.
pub(crate) fn pool_of(served: &Served) -> SlotPool {
    let manifest = served.manifest();
    let keep = manifest
        .seed()
        .map(|seed| seed.paths().to_vec())
        .unwrap_or_default();
    SlotPool::of(
        served.root(),
        manifest.worktrees().get(),
        manifest.base().unwrap_or(BASE_UNSTATED),
        keep,
    )
}

/// Where a Job's record says its worktree is: the slot it leased, or the
/// derived path of a Job cut before the pool. **Not whether the Job still
/// holds the slot** — [`Fleet::job_tree`] asks that.
pub(crate) fn spec_of(root: &str, job: &Job) -> Result<WorktreeSpec, WorktreeSpecRefused> {
    let spec = WorktreeSpec::for_job(root, &job.handle())?;
    Ok(match job.worktree_slot() {
        Some(slot) => spec.in_slot(slot),
        None => spec,
    })
}

/// A Job's worktree, as the lookup finds it.
pub(crate) enum JobTree {
    /// The Job's own: its derived path, or a slot it still holds.
    Here(WorktreeSpec),
    /// The record names a slot the Job no longer holds, and why.
    Lost { slot: u32, why: String },
}

impl JobTree {
    pub(crate) fn here(self) -> Option<WorktreeSpec> {
        match self {
            JobTree::Here(spec) => Some(spec),
            JobTree::Lost { .. } => None,
        }
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
    /// The Job's worktree, looked up: where its record says, and whether a slot
    /// it names is still its own.
    pub(crate) fn job_tree(
        &self,
        served: &Served,
        job: &Job,
    ) -> Result<JobTree, WorktreeSpecRefused> {
        let spec = spec_of(served.root(), job)?;
        let Some(slot) = spec.slot() else {
            return Ok(JobTree::Here(spec));
        };
        let why = match self
            .vcs()
            .slot_standing(&pool_of(served), slot, job.id().as_str())
        {
            SlotStanding::Held => return Ok(JobTree::Here(spec)),
            SlotStanding::HeldBy(who) => format!("{who} holds it now"),
            SlotStanding::Free => String::from("it was given back to the pool"),
            SlotStanding::Gone => String::from("it is not on disk"),
        };
        Ok(JobTree::Lost { slot, why })
    }

    /// [`job_tree`](Fleet::job_tree) for a reader that only wants a worktree
    /// that is the Job's: `None` for a slot it lost, and for an id that names
    /// no directory.
    pub(crate) fn tree_spec(&self, served: &Served, job: &Job) -> Option<WorktreeSpec> {
        self.job_tree(served, job).ok()?.here()
    }

    /// What a reclaim, a branch delete or the sweep acts on: the Job's own
    /// worktree, or — where its slot is no longer its own — only its branch,
    /// at the derived path nothing is at, so the slot is never read or touched.
    pub(crate) fn reclaimed_spec(
        &self,
        served: &Served,
        job: &Job,
    ) -> Result<WorktreeSpec, Adrift> {
        let unworkable = |cause| Adrift::Unworkable {
            job: job.id().clone(),
            cause,
        };
        match self.job_tree(served, job).map_err(unworkable)? {
            JobTree::Here(spec) => Ok(spec),
            JobTree::Lost { .. } => {
                WorktreeSpec::for_job(served.root(), &job.handle()).map_err(unworkable)
            }
        }
    }

    /// Whether a Job that has never had a worktree would find no slot free in
    /// its repository's pool. **One answer for admission and the Board**, as
    /// `volume_is_short` is. A Job already holding one is never short.
    pub(crate) fn slot_is_short(&self, job: &Job) -> bool {
        if job.branch().is_some() || job.worktree_slot().is_some() {
            return false;
        }
        let Ok(served) = self.served_by(job) else {
            return false;
        };
        !self.vcs().slot_open(&pool_of(&served), job.id().as_str())
    }

    /// Write the slot the worktree is. No event, for `branded`'s reason.
    pub(crate) async fn slotted(&self, job: &Job, slot: u32) -> Result<Job, Adrift> {
        let job = job.in_slot(slot);
        self.store()
            .lock()
            .await
            .record_slot(&job)
            .map_err(Adrift::Writing)?;
        Ok(job)
    }

    /// Give a Job's slot back now it has ended, by the pool's rules. Refused,
    /// it stays held, the Job's log and `armada worktree --status` say why, and
    /// so does the answer.
    pub(crate) async fn released_slot(&self, job: &Job) -> Option<String> {
        let slot = job.worktree_slot()?;
        let served = self.served_by(job).ok()?;
        let pool = pool_of(&served);
        let id = job.id().as_str();
        if self.vcs().slot_standing(&pool, slot, id) != SlotStanding::Held {
            return None;
        }
        let (level, said, why) = match self.vcs().release_slot(&pool, slot, id) {
            Ok(()) => (
                Level::Info,
                "the Job's slot was given back to the pool",
                None,
            ),
            Err(SlotKept(why)) => (
                Level::Warn,
                "the Job ended and its slot stays held, because the pool would not take it back",
                Some(why),
            ),
        };
        let mut envelope = Envelope::new(
            self.now(),
            level,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(job.id().as_ulid().clone())
        .with_field("slot", FieldValue::Str(format!("slot-{slot}")));
        if let Some(why) = &why {
            envelope = envelope.with_field("because", FieldValue::Str(why.clone()));
        }
        self.noted_in_the_log(job.id(), &envelope);
        why
    }
}
