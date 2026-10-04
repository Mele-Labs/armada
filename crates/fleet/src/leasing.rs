//! A Job's worktree is the pool slot it leased: leased at dispatch, recorded
//! on the Job, looked up after, and given back when the Job ends — or, for a
//! completed Job, when a person clears it.
//! `docs/concepts/fleet.md`, *Worktree slots*.
//!
//! **Looked up, and asked whether it is still the Job's.** A slot is reused,
//! so the record says where the Job's worktree was and the pool's record
//! beside the slot says whether it still is. A Job cut before the pool has no
//! slot recorded and keeps the path its handle derives.

use adapter_traits::{
    AgentHarness, Delivery, SlotChange, SlotHeld, SlotKept, SlotPool, SlotReading, SlotRefused,
    SlotStanding, Vcs, WorkProduct, WorktreeSpec, WorktreeSpecRefused,
};
use api::Refusal;
use core_model::{Component, Envelope, FieldValue, Job, JobStatus, Level};
use ipc::{ChangeSlotPool, ManifestId, SlotAct, SlotPoolChanged, WireError};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::repositories::Served;

/// The base a Manifest that names none is leased from, as `armada worktree`
/// leases it.
const BASE_UNSTATED: &str = "main";

/// A slot the pool does not have, or none named. A 422.
pub(crate) const NO_SUCH_SLOT: &str = "fleet.no_such_slot";
/// A slot removed while something holds it. A 409, as are the five after.
const SLOT_HELD: &str = "fleet.slot_held";
/// A slot removed whose holder is gone and left work in it.
const SLOT_STRANDED: &str = "fleet.slot_stranded";
/// A slot removed while a lease or a release is under way on it.
pub(crate) const SLOT_BUSY: &str = "fleet.slot_busy";
/// A slot removed that is a directory and not a checkout: a person's to clear.
const SLOT_NOT_A_CHECKOUT: &str = "fleet.slot_not_a_checkout";
/// A slot removed whose checkout holds uncommitted files.
const SLOT_DIRTY: &str = "fleet.slot_dirty";
/// The pool's one slot, removed.
const SLOT_LAST: &str = "fleet.slot_last";
/// git refused, and this is what it said. A 500.
pub(crate) const SLOT_UNCHANGED: &str = "fleet.slot_unchanged";

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

/// The pool, cutting a new lease's branch from `from` where a person chose a
/// branch at approval — spike 022, slice 4 — and from the base where nobody
/// did. Only the lease asks this; every other question of the pool is the
/// pool's own, measured from the base.
pub(crate) fn pool_cut_from(served: &Served, from: Option<&core_model::Branch>) -> SlotPool {
    let Some(from) = from else {
        return pool_of(served);
    };
    let manifest = served.manifest();
    let keep = manifest
        .seed()
        .map(|seed| seed.paths().to_vec())
        .unwrap_or_default();
    SlotPool::of(
        served.root(),
        manifest.worktrees().get(),
        from.as_str(),
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

/// One slot of a served repository's pool, and the title of the Job holding
/// it where one does and the store still has it.
pub(crate) struct PoolSlot {
    pub(crate) manifest: String,
    pub(crate) base: String,
    pub(crate) reading: SlotReading,
    pub(crate) job_title: Option<String>,
    /// What it holds, where it is stranded.
    pub(crate) stranded: Option<adapter_traits::StrandedWork>,
    /// What a rescue Scout read of it, where one has.
    pub(crate) rescue: Option<store::KeptRescue>,
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

    /// Every served repository's pool, slot by slot: what Bridge's Cleanup
    /// draws, read the way `armada worktree --status` reads it.
    pub(crate) async fn pool_slots(&self) -> Result<Vec<PoolSlot>, Adrift> {
        let (loaded, _) = self.every_job().await?;
        let title = |id: &str| {
            loaded
                .jobs
                .iter()
                .find(|job| job.id().as_str() == id)
                .map(|job| job.title().as_str().to_string())
        };
        let mut kept = self.store().lock().await.rescues().unwrap_or_default();
        let mut slots = Vec::new();
        for served in self.repositories().served() {
            let manifest = served.manifest().id().as_str().to_string();
            let pool = pool_of(&served);
            for reading in self.vcs().slot_pool(&pool) {
                let job_title = match &reading.held {
                    SlotHeld::Job(id) => title(id),
                    _ => None,
                };
                let stranded = match &reading.held {
                    SlotHeld::Stranded(_) => self.vcs().stranded_work(&pool, reading.slot).ok(),
                    _ => None,
                };
                // **A Finding is of the commit it read.** One whose slot has
                // moved on, or is no longer stranded, is not shown.
                let at = kept.iter().position(|one| {
                    one.manifest_id == manifest
                        && one.slot == reading.slot
                        && stranded
                            .as_ref()
                            .is_some_and(|work| work.commit == one.commit)
                });
                let rescue = at.map(|at| kept.swap_remove(at));
                slots.push(PoolSlot {
                    manifest: manifest.clone(),
                    base: pool.base().to_string(),
                    reading,
                    job_title,
                    stranded,
                    rescue,
                });
            }
        }
        Ok(slots)
    }

    /// A person's change to a repository's pool, from Cleanup's bay grid. The
    /// pool writes it beside the slots, so `armada worktree lease` honours it
    /// too and it outlives Fleet.
    pub(crate) fn change_slot_pool(
        &self,
        asked: ChangeSlotPool,
        manifest_id: Option<&ManifestId>,
    ) -> Result<SlotPoolChanged, Refusal> {
        let served = self.served_named(manifest_id)?;
        let raised = |code: &str, said: String| WireError::raised(code, said, self.run_id());
        let change = match (asked.act, asked.slot) {
            (SlotAct::Add, _) => SlotChange::Add,
            (SlotAct::Remove, Some(n)) => SlotChange::Remove(n),
            (SlotAct::Close, Some(n)) => SlotChange::Close(n),
            (SlotAct::Open, Some(n)) => SlotChange::Open(n),
            (_, None) => {
                return Err(Refusal::Unacceptable(raised(
                    NO_SUCH_SLOT,
                    String::from("no slot named"),
                )))
            }
        };
        let slot = self
            .vcs()
            .change_slot_pool(&pool_of(&served), change)
            .map_err(|why| match why {
                SlotRefused::NoSuchSlot(n) => {
                    Refusal::Unacceptable(raised(NO_SUCH_SLOT, format!("no slot-{n}")))
                }
                SlotRefused::Held(by) => {
                    Refusal::IllegalMove(raised(SLOT_HELD, format!("held by {by}")))
                }
                SlotRefused::Stranded(what) => {
                    Refusal::IllegalMove(raised(SLOT_STRANDED, format!("stranded, holding {what}")))
                }
                SlotRefused::Busy => {
                    Refusal::IllegalMove(raised(SLOT_BUSY, String::from("a lease is under way")))
                }
                SlotRefused::NotACheckout => Refusal::IllegalMove(raised(
                    SLOT_NOT_A_CHECKOUT,
                    String::from("not a checkout"),
                )),
                SlotRefused::Dirty(files) => Refusal::IllegalMove(raised(
                    SLOT_DIRTY,
                    format!(
                        "{} uncommitted, first {}",
                        files.len(),
                        files.first().map(String::as_str).unwrap_or_default()
                    ),
                )),
                SlotRefused::LastSlot => {
                    Refusal::IllegalMove(raised(SLOT_LAST, String::from("the last slot")))
                }
                SlotRefused::Vcs(said) => Refusal::Fault(raised(SLOT_UNCHANGED, said)),
            })?;
        Ok(SlotPoolChanged {
            manifest_id: ManifestId::from(served.manifest().id()),
            slot,
        })
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

    /// At a terminal status. **A completed Job holds its slot until a person
    /// clears it**, the owner's decision, so Show again and anything else
    /// reading its tree still finds it; the slot is marked so `--status` says
    /// so. Any other end gives it back now.
    pub(crate) async fn slot_at_the_end(&self, job: &Job) {
        if job.status() != JobStatus::CompletedSuccess {
            self.released_slot(job).await;
            return;
        }
        let (Some(slot), Ok(served)) = (job.worktree_slot(), self.served_by(job)) else {
            return;
        };
        self.vcs()
            .mark_slot_completed(&pool_of(&served), slot, job.id().as_str());
    }

    /// Whether this is a completed Job still holding its slot, which only a
    /// person clearing it gives back — never the sweep.
    pub(crate) fn held_until_cleared(&self, job: &Job) -> bool {
        job.status() == JobStatus::CompletedSuccess
            && self
                .served_by(job)
                .ok()
                .and_then(|served| self.tree_spec(&served, job))
                .is_some_and(|spec| spec.slot().is_some())
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
