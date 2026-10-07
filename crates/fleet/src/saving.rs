//! What a Clear does with a Job's uncommitted files: commit them to its branch,
//! then free the slot or remove the worktree. Beside `reclaiming.rs`, which
//! keeps the act whole; this holds only the saving. A Job that was killed or
//! failed gets the same park the moment it ends, so it never sits `kept`;
//! it lives here because `dispatch.rs` and `leasing.rs` have no room.
//! `docs/concepts/fleet.md`, *Worktree slots*, and the decision record
//! `2026-10-05-a-branch-commit-is-enough-to-free-a-slot`.
//!
//! Nothing is pushed. A sweep never comes here: it gives back only what is
//! provably safe, and a tree with uncommitted files is not.

use adapter_traits::{
    AgentHarness, Delivery, SlotKept, SlotParkRefused, SlotStanding, Vcs, WorkProduct, WorktreeSpec,
};
use api::Refusal;
use core_model::{Component, Envelope, FieldValue, Job, JobId, JobStatus, Level};
use ipc::{ChangeSlotPool, ManifestId, SlotPoolChanged, SlotReleased, WireError};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::leasing::{
    pool_of, NO_SUCH_SLOT, SLOT_BUSY, SLOT_HOLDER_CHANGED, SLOT_HOLDER_UNNAMED, SLOT_NOT_PARKABLE,
};
use crate::repositories::Served;

const CLEARED: &str = "a Clear committed the Job's uncommitted files to its branch";
const ENDED: &str = "the Job ended and its uncommitted files were committed to its branch";

/// The WIP commit a Clear made, on the Job's own branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedWork {
    pub branch: String,
    pub commit: String,
    pub files: Vec<String>,
}

/// What became of the slot a Job held when it was cleared.
pub(crate) enum Freed {
    /// The Job held none, or no longer does.
    NotHeld,
    Released,
    /// Its uncommitted files were committed to its branch first.
    Saved(SavedWork),
    /// The slot stays held, for this reason in git words.
    Kept(String),
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
    /// Give back the slot a finished Job holds, committing what is uncommitted
    /// in it to the Job's branch where the pool refuses it for that.
    ///
    /// **The pool's release goes first**, so a clean slot is given back exactly
    /// as it was before. A park is the fallback for any refusal and says so
    /// itself where it cannot help: a detached HEAD, the base branch, a busy
    /// slot.
    pub(crate) async fn freed_by_a_clear(&self, job: &Job) -> Freed {
        self.freed(job, CLEARED).await
    }

    /// A Job's slot at the moment it ends: a killed or failed Job's is parked
    /// where the pool refuses it for uncommitted files, as a Clear would, and
    /// any other end is the pool's plain release. Answers why it stays held.
    pub(crate) async fn released_or_saved(&self, job: &Job) -> Option<String> {
        if !matches!(job.status(), JobStatus::Killed | JobStatus::CompletedFailed) {
            return self.released_slot(job).await;
        }
        match self.freed(job, ENDED).await {
            Freed::Kept(why) => Some(why),
            _ => None,
        }
    }

    async fn freed(&self, job: &Job, saved_said: &str) -> Freed {
        let Some(slot) = job.worktree_slot() else {
            return Freed::NotHeld;
        };
        let Ok(served) = self.served_by(job) else {
            return Freed::NotHeld;
        };
        let pool = pool_of(&served);
        let id = job.id().as_str();
        if self.vcs().slot_standing(&pool, slot, id) != SlotStanding::Held {
            return Freed::NotHeld;
        }
        let name = format!("slot-{slot}");
        let Err(SlotKept(_)) = self.vcs().release_slot(&pool, slot, id) else {
            self.job_gave_back_slot(job).await;
            self.noted_slot(
                job,
                Level::Info,
                "the Job's slot was given back to the pool",
                &name,
                None,
            );
            return Freed::Released;
        };
        match self.vcs().park_slot(&pool, slot, id) {
            Ok(parked) => {
                self.job_gave_back_slot(job).await;
                match parked.commit {
                    Some(commit) => {
                        let saved = SavedWork {
                            branch: parked.branch,
                            commit,
                            files: parked.files,
                        };
                        self.noted_saved(job.id(), &saved, Some(&name), saved_said);
                        Freed::Saved(saved)
                    }
                    None => {
                        self.noted_slot(
                            job,
                            Level::Info,
                            "the Job's slot was given back to the pool",
                            &name,
                            None,
                        );
                        Freed::Released
                    }
                }
            }
            Err(refused) => {
                let why = refused.said();
                self.noted_slot(
                    job,
                    Level::Warn,
                    "the Job ended and its slot stays held, because its uncommitted files could not be committed",
                    &name,
                    Some(&why),
                );
                Freed::Kept(why)
            }
        }
    }

    /// Commit what is uncommitted in a Job's own worktree to its branch before
    /// a reclaim removes it, which would take the files with it.
    pub(crate) fn saved_before_removal(
        &self,
        spec: &WorktreeSpec,
        job: &JobId,
    ) -> Result<Option<SavedWork>, Adrift> {
        let saved = adapters::leasing::save_worktree(spec, job.as_str()).map_err(|refused| {
            Adrift::WorktreeNotSaved {
                job: job.clone(),
                why: refused.said(),
            }
        })?;
        Ok(saved.map(|committed| {
            let saved = SavedWork {
                branch: spec.branch(),
                commit: committed.commit,
                files: committed.files,
            };
            self.noted_saved(job, &saved, None, CLEARED);
            saved
        }))
    }

    /// `change_slot_pool`'s `release`: commit what an agent session holds
    /// uncommitted in a slot to its branch and give the slot back, the same
    /// park a Clear makes for a Job's. **Only for the holder the person was
    /// shown**, so a slot re-leased since is refused. The session's process is
    /// never touched.
    pub(crate) fn session_released(
        &self,
        served: &Served,
        asked: &ChangeSlotPool,
    ) -> Result<SlotPoolChanged, Refusal> {
        let raised = |code: &str, said: String| WireError::raised(code, said, self.run_id());
        let (Some(slot), Some(holder)) = (asked.slot, asked.holder.as_deref()) else {
            return Err(Refusal::Unacceptable(raised(
                if asked.slot.is_none() {
                    NO_SUCH_SLOT
                } else {
                    SLOT_HOLDER_UNNAMED
                },
                String::from("a release names the slot and the holder it was shown"),
            )));
        };
        let parked = self
            .vcs()
            .release_session_slot(&pool_of(served), slot, holder)
            .map_err(|why| {
                let said = why.said();
                match why {
                    SlotParkRefused::HolderChanged(_)
                    | SlotParkRefused::HeldByAnother(_)
                    | SlotParkRefused::NotLeased => {
                        Refusal::IllegalMove(raised(SLOT_HOLDER_CHANGED, said))
                    }
                    SlotParkRefused::Busy => Refusal::IllegalMove(raised(SLOT_BUSY, said)),
                    _ => Refusal::IllegalMove(raised(SLOT_NOT_PARKABLE, said)),
                }
            })?;
        let saved = parked.commit.map(|commit| ipc::ReclaimedSaved {
            commit,
            files: parked.files,
        });
        Ok(SlotPoolChanged {
            manifest_id: ManifestId::from(served.manifest().id()),
            slot,
            released: Some(SlotReleased {
                branch: parked.branch,
                saved,
            }),
        })
    }

    fn noted_saved(&self, job: &JobId, saved: &SavedWork, slot: Option<&str>, said: &str) {
        let mut envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(job.as_ulid().clone())
        .with_field("branch", FieldValue::Str(saved.branch.clone()))
        .with_field("commit", FieldValue::Str(saved.commit.clone()))
        .with_field("files", FieldValue::Str(saved.files.join(", ")));
        if let Some(slot) = slot {
            envelope = envelope.with_field("slot", FieldValue::Str(slot.to_string()));
        }
        self.noted_in_the_log(job, &envelope);
    }

    fn noted_slot(&self, job: &Job, level: Level, said: &str, slot: &str, because: Option<&str>) {
        let mut envelope = Envelope::new(
            self.now(),
            level,
            Component::Fleet,
            self.run().clone(),
            said,
        )
        .in_job(job.id().as_ulid().clone())
        .with_field("slot", FieldValue::Str(slot.to_string()));
        if let Some(because) = because {
            envelope = envelope.with_field("because", FieldValue::Str(because.to_string()));
        }
        self.noted_in_the_log(job.id(), &envelope);
    }
}
