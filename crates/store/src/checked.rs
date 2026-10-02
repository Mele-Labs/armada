//! The tree a Job's Checks last passed on, which is the only tree `merge_by:
//! push` lands. `docs/concepts/manifest.md`, *How work lands*.
//!
//! **A column on `jobs`**, for [`crate::delivery`]'s reason: one value per Job,
//! read beside the Job and nowhere else. Overwritten by every later pass.

use core_model::JobId;

use crate::error::{fault, LoadJobError, RowError, WriteError};
use crate::open::Store;

/// Version 88 — the tree a Job's Checks last passed on.
///
/// Null is every Job whose Checks have passed on nothing yet, and every Job
/// before this existed; both are read as unchecked, so their next push runs
/// the Checks first.
pub(crate) const V88: &str = r#"
ALTER TABLE jobs ADD COLUMN checked_tree TEXT;
"#;

impl Store {
    /// Write down the tree a run of the Job's Checks passed on.
    pub fn record_checked_tree(&mut self, job_id: &JobId, tree: &str) -> Result<(), WriteError> {
        let updated = self
            .conn
            .execute(
                "UPDATE jobs SET checked_tree = ?2 WHERE job_id = ?1",
                (job_id.as_str(), tree),
            )
            .map_err(fault("recording the tree a Job's Checks passed on"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: job_id.clone(),
            });
        }
        Ok(())
    }

    /// The tree the Job's Checks last passed on, or `None` where they have
    /// passed on none.
    pub fn checked_tree_for(&self, job_id: &JobId) -> Result<Option<String>, LoadJobError> {
        self.conn
            .query_row(
                "SELECT checked_tree FROM jobs WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| row.get(0),
            )
            .or_else(|why| match why {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(LoadJobError::Unreadable(RowError::Database(fault(
                    "reading the tree a Job's Checks passed on",
                )(
                    other
                )))),
            })
    }
}
