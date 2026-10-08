//! That a person asked the forge to merge a Job's pull request once its checks
//! pass. The sweep completes a Job at its gate only for a merge that was asked
//! for here; one merged some other way is left for a person to take.

use core_model::{JobId, Timestamp};

use crate::error::{fault, LoadJobError, WriteError};
use crate::open::Store;

impl Store {
    /// Record the ask. Written again on a second press, which changes nothing a reader sees.
    pub fn record_auto_merge_asked(&mut self, job_id: &JobId, at: &Timestamp) -> Result<(), WriteError> {
        let updated = self
            .conn
            .execute(
                "UPDATE jobs SET delivery_auto_merge_asked = ?2 WHERE job_id = ?1",
                (job_id.as_str(), at.as_str()),
            )
            .map_err(fault("recording that auto-merge was asked for"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: job_id.clone(),
            });
        }
        Ok(())
    }

    /// Whether the press asked for it. A Job the file does not hold reads as no.
    pub fn auto_merge_asked(&self, job_id: &JobId) -> Result<bool, LoadJobError> {
        self.conn
            .query_row(
                "SELECT delivery_auto_merge_asked IS NOT NULL FROM jobs WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| row.get::<_, bool>(0),
            )
            .or_else(|cause| match cause {
                rusqlite::Error::QueryReturnedNoRows => Ok(false),
                cause => Err(cause),
            })
            .map_err(fault("reading whether auto-merge was asked for"))
            .map_err(LoadJobError::Database)
    }
}
