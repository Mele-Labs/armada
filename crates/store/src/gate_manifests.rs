//! Replacing a Job's gating Manifests once the diff says which gate it.
//!
//! **At dispatch the set is decided from what the Job is expected to write;
//! at the gate it is decided from what it wrote.** The second replaces the
//! first whole, in one transaction, in the order given.

use core_model::{GateManifest, JobId};

use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    pub fn replace_gate_manifests(
        &mut self,
        job: &JobId,
        gates: &[GateManifest],
    ) -> Result<(), WriteError> {
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting the gate manifest replacement"))
            .map_err(WriteError::Database)?;
        tx.execute(
            "DELETE FROM job_manifests WHERE job_id = ?1",
            rusqlite::params![job.as_str()],
        )
        .map_err(fault("clearing the gate manifests"))
        .map_err(WriteError::Database)?;
        for (ordinal, gate) in gates.iter().enumerate() {
            let (outcome, not_run_reason) = gate.outcome.as_wire();
            tx.execute(
                "INSERT INTO job_manifests (job_id, ordinal, manifest_id, outcome, not_run_reason)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    job.as_str(),
                    ordinal as i64,
                    gate.manifest_id.as_str(),
                    outcome,
                    not_run_reason,
                ],
            )
            .map_err(fault("writing a gate manifest"))
            .map_err(WriteError::Database)?;
        }
        tx.commit()
            .map_err(fault("committing the gate manifests"))
            .map_err(WriteError::Database)
    }
}
