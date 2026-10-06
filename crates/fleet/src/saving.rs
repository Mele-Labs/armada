//! What a Clear does with a Job's uncommitted files: commit them to its branch,
//! then free the slot or remove the worktree. Beside `reclaiming.rs`, which
//! keeps the act whole; this holds only the saving.
//! `docs/concepts/fleet.md`, *Worktree slots*, and the decision record
//! `2026-10-05-a-branch-commit-is-enough-to-free-a-slot`.
//!
//! Nothing is pushed. A sweep never comes here: it gives back only what is
//! provably safe, and a tree with uncommitted files is not.

use adapter_traits::{
    AgentHarness, Delivery, SlotKept, SlotStanding, Vcs, WorkProduct, WorktreeSpec,
};
use core_model::{Component, Envelope, FieldValue, Job, JobId, Level};

use crate::adrift::Adrift;
use crate::daemon::Fleet;
use crate::leasing::pool_of;

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
            Ok(parked) => match parked.commit {
                Some(commit) => {
                    let saved = SavedWork {
                        branch: parked.branch,
                        commit,
                        files: parked.files,
                    };
                    self.noted_saved(job.id(), &saved, Some(&name));
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
            },
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
            self.noted_saved(job, &saved, None);
            saved
        }))
    }

    fn noted_saved(&self, job: &JobId, saved: &SavedWork, slot: Option<&str>) {
        let mut envelope = Envelope::new(
            self.now(),
            Level::Info,
            Component::Fleet,
            self.run().clone(),
            "a Clear committed the Job's uncommitted files to its branch",
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
