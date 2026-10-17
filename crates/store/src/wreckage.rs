//! A store damaged the way a migration damages one, for the tests that have to
//! recover from it.
//!
//! **Compiled only under the `wreckage` feature**, which nothing but `testkit`
//! turns on. The shipping binary therefore has no method that damages a store,
//! and `rusqlite` stays scoped to this crate rather than reaching the crate
//! whose test needs the damage.

use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    /// Take the frozen workflow off a stored Job.
    ///
    /// This is the state V7 left four real rows in: present, still naming their
    /// Job and their Manifest, and unrebuildable. Reproducing it is the only
    /// way to test what clears it.
    pub fn unfreeze_a_jobs_workflow(&mut self, job_id: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE jobs SET workflow = NULL WHERE job_id = ?1",
                (job_id,),
            )
            .map_err(fault("unfreezing a workflow"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// How many runs `check_runs` holds for `repository`.
    pub fn check_runs_kept(&self, repository: &core_model::ManifestId) -> Result<u64, WriteError> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM check_runs WHERE repository = ?1",
                (repository.as_str(),),
                |row| row.get(0),
            )
            .map_err(fault("counting kept Check runs"))
            .map_err(WriteError::Database)
    }

    /// Take the Check run history away, so the next append fails.
    pub fn without_the_check_run_history(&mut self) -> Result<(), WriteError> {
        self.conn
            .execute("DROP TABLE check_runs", ())
            .map_err(fault("dropping the Check run history"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// A pilot already under way on a Job, the row a take over used to write.
    /// Nothing writes one now, so a test of the exits has to plant it.
    pub fn plant_a_pilot(
        &mut self,
        job: &core_model::JobId,
        reason: core_model::PilotReason,
        at: &core_model::Timestamp,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO job_pilots (job_id, reason, marked_run, marked_at, \
                 piloted_at) VALUES (?1, ?2, '', ?3, ?3)",
                (job.as_str(), reason.as_wire(), at.as_str()),
            )
            .map_err(fault("planting a pilot"))
            .map_err(WriteError::Database)?;
        Ok(())
    }
}
