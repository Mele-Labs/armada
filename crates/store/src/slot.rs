//! Which pool slot a Job's worktree is.
//!
//! A slot is reused, so a Job's worktree can no longer be derived from the Job;
//! it is recorded when the lease is taken and looked up after.
//! `docs/contracts/system-architecture.md` has the layout.

use core_model::Job;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// Version 92 — the slot a Job leased.
///
/// **Null, and no backfill.** Every row before this column is a Job cut before
/// the pool, whose worktree is still at the path its handle derives, and null
/// is what says so. `Store::record_slot` is the one writer.
pub(crate) const V92: &str = r#"
ALTER TABLE jobs ADD COLUMN worktree_slot INTEGER;
"#;

impl Store {
    /// Write the slot the Job's worktree is. No event, for `record_branch`'s
    /// reason: nothing in the log describes a worktree.
    pub fn record_slot(&mut self, job: &Job) -> Result<(), WriteError> {
        let Some(slot) = job.worktree_slot() else {
            return Ok(());
        };
        let updated = self
            .conn
            .execute(
                "UPDATE jobs SET worktree_slot = ?2 WHERE job_id = ?1",
                (job.id().as_str(), i64::from(slot)),
            )
            .map_err(fault("recording the worktree slot"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: job.id().clone(),
            });
        }
        Ok(())
    }
}
