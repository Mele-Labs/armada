//! A repair Drone a Job's Commander has out. `#2170`.
//!
//! **Kept, not remembered.** The hold that stops a second repair, and a person's
//! restart, while one is out was Fleet's memory alone, and a Fleet that restarted
//! lost it with the Drone: the Job stayed held with nothing running. The row is
//! written before the Drone starts and removed once what it left has been dealt
//! with, so whichever Fleet reads the store next finds a repair that ended
//! without being heard.

use core_model::JobId;

use crate::error::{fault, RowError, WriteError};
use crate::open::Store;

/// A repair sent and not yet dealt with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepairOut {
    pub job_id: JobId,
    /// The step the repair puts back, or the first step of a Job that broke
    /// before any began.
    pub step: String,
    /// What broke, in the word the Job's log carries.
    pub broke: String,
    pub drone_id: String,
}

impl Store {
    /// Note that a repair is out for this Job. A Job has one at a time, so a
    /// second replaces the first: the first's Drone is no longer the one the
    /// hold is for.
    pub fn note_repair_out(&mut self, repair: &RepairOut) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO job_commander_repairs (job_id, step, broke, drone_id) \
                 VALUES (?1, ?2, ?3, ?4)",
                (
                    repair.job_id.as_str(),
                    &repair.step,
                    &repair.broke,
                    &repair.drone_id,
                ),
            )
            .map(|_| ())
            .map_err(|why| match why {
                rusqlite::Error::SqliteFailure(err, _)
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    WriteError::NoSuchJob {
                        job_id: repair.job_id.clone(),
                    }
                }
                other => WriteError::Database(fault("noting a repair Drone out")(other)),
            })
    }

    /// Give the Job's row back, if it is still this Drone's. **By the Drone and
    /// not the Job**: the Commander the repair's end woke may have sent another
    /// before the first one's row is given back, and that row is not this one's
    /// to remove. Nothing is wrong with a Job that has none.
    pub fn repair_dealt_with(&mut self, job_id: &JobId, drone_id: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "DELETE FROM job_commander_repairs WHERE job_id = ?1 AND drone_id = ?2",
                (job_id.as_str(), drone_id),
            )
            .map(|_| ())
            .map_err(|why| WriteError::Database(fault("giving a repair Drone back")(why)))
    }

    /// Every repair still out, in the order the Jobs were written.
    pub fn repairs_out(&self) -> Result<Vec<RepairOut>, RowError> {
        let database = |why| RowError::Database(fault("reading the repair Drones out")(why));
        let mut statement = self
            .conn
            .prepare("SELECT job_id, step, broke, drone_id FROM job_commander_repairs ORDER BY rowid")
            .map_err(database)?;
        let rows = statement
            .query_map((), |row| {
                Ok(RepairOut {
                    job_id: JobId::carried(core_model::Ulid::carried(row.get::<_, String>(0)?)),
                    step: row.get(1)?,
                    broke: row.get(2)?,
                    drone_id: row.get(3)?,
                })
            })
            .map_err(database)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(database)
    }
}
