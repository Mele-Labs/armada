//! An Epic's waves and how its members end: the pass that made each member,
//! when a member's pull request merged, and what finishes the parent. Spike
//! 022, slice 6.
//!
//! **The pass is the third column of `DispatchOrigin`**, beside the two
//! `crate::read` already rebuilds it from, so a member carries its wave on its
//! own row. Null on every child made before V101, which entered `queued`; a
//! member with a pass entered `awaiting_approval`, and the rebuild reads that
//! off the column rather than off the log.
//!
//! **The merge's instant is the forge's**, read on the same call that said it
//! merged. Null on a pull request settled before V101, and on one that closed.

use std::collections::BTreeMap;

use core_model::{JobId, Timestamp, Ulid};

use crate::error::{fault, LoadJobError, WriteError};
use crate::open::Store;

/// Version 101 — the pass that made a member, when its pull request merged,
/// and what finishes a Job. Nothing is backfilled: a null is a row written
/// before this, which reads as it did then.
pub(crate) const V101: &str = r#"
ALTER TABLE jobs ADD COLUMN dispatched_by_pass INTEGER
    CHECK (dispatched_by_pass IS NULL OR dispatched_by_pass > 0);

ALTER TABLE jobs ADD COLUMN delivery_merged_at TEXT
    CHECK (delivery_merged_at IS NULL OR trim(delivery_merged_at) <> '');

ALTER TABLE job_landing ADD COLUMN complete_when TEXT
    CHECK (complete_when IS NULL OR complete_when IN ('delivered', 'all_members_landed'));
"#;

impl Store {
    /// When this Job's pull request merged, as the forge said. **Its own
    /// `UPDATE`**, for `record_landed`'s reason, and written once: a merge
    /// never changes back.
    pub fn record_merged_at(&mut self, job_id: &JobId, at: &Timestamp) -> Result<(), WriteError> {
        let updated = self
            .conn
            .execute(
                "UPDATE jobs SET delivery_merged_at = ?2 WHERE job_id = ?1",
                (job_id.as_str(), at.as_str()),
            )
            .map_err(fault("recording when a pull request merged"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: job_id.clone(),
            });
        }
        Ok(())
    }

    /// When every merged pull request merged, keyed by Job: one query for the
    /// Board, for `landed_by_job`'s reason.
    pub fn merged_at_by_job(&self) -> Result<BTreeMap<JobId, Timestamp>, LoadJobError> {
        let reading = "reading when pull requests merged";
        let mut asking = self
            .conn
            .prepare(
                "SELECT job_id, delivery_merged_at FROM jobs WHERE delivery_merged_at IS NOT NULL",
            )
            .map_err(fault(reading))
            .map_err(LoadJobError::Database)?;
        let rows = asking
            .query_map((), |row| {
                Ok((
                    JobId::carried(Ulid::carried(row.get::<_, String>(0)?)),
                    Timestamp::from_rfc3339(row.get::<_, String>(1)?),
                ))
            })
            .map_err(fault(reading))
            .map_err(LoadJobError::Database)?;
        rows.collect::<Result<BTreeMap<_, _>, rusqlite::Error>>()
            .map_err(fault(reading))
            .map_err(LoadJobError::Database)
    }

    /// When this one Job's pull request merged, where it has.
    pub fn merged_at(&self, job_id: &JobId) -> Result<Option<Timestamp>, LoadJobError> {
        self.conn
            .query_row(
                "SELECT delivery_merged_at FROM jobs WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| row.get::<_, Option<String>>(0),
            )
            .or_else(|cause| match cause {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                cause => Err(cause),
            })
            .map(|at| at.map(Timestamp::from_rfc3339))
            .map_err(fault("reading when a pull request merged"))
            .map_err(LoadJobError::Database)
    }

    /// Which pass over `step` this Job is on, counted from one: the loop
    /// returns that landed on it, plus one.
    ///
    /// **Not `step_iteration`**, which counts the returns a step *caused*. An
    /// Epic's plan causes none; `roll_up` sends the work back to it, and each
    /// return it receives is a new wave.
    pub fn pass_over(
        &self,
        job_id: &JobId,
        step: &core_model::StepId,
    ) -> Result<u32, LoadJobError> {
        let returns: i64 = self
            .conn
            .query_row(
                "SELECT count(*) FROM job_events
                 WHERE job_id = ?1 AND step_id = ?2 AND returned_by IS NOT NULL
                   AND kind = 'step_transition'
                   AND state_from = 'advanced' AND state_to = 'running'",
                (job_id.as_str(), step.as_str()),
                |row| row.get(0),
            )
            .map_err(fault("counting the returns that landed on a step"))
            .map_err(LoadJobError::Database)?;
        Ok(u32::try_from(returns.max(0))
            .unwrap_or(u32::MAX)
            .saturating_add(1))
    }
}
