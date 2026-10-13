//! What a Job's Commander has cost it. `#2166`.
//!
//! **A table of its own, so the cap cannot be spent by the agent that raises
//! it.** A Job's cap is compared with [`Store::spend_for`], which sums the
//! Drones' rows. A Commander's turns are in no row of those, so no figure here
//! can hold a Job back from starting a Drone, and the Job's cost a person reads
//! is the two added (`fleet::serving`).

use core_model::JobId;

use crate::error::{fault, LoadJobError, RowError, WriteError};
use crate::open::Store;

impl Store {
    /// Write down what one Commander turn cost. **Keyed by the turn**, so
    /// recording the same turn again replaces its row and inflates nothing.
    ///
    /// `None` is a turn that ended without naming a price, which is not a price
    /// of nothing: [`Store::commander_cost_for`] sums the rows that named one.
    pub fn record_commander_spend(
        &mut self,
        job_id: &JobId,
        turn_id: &str,
        cost_micros: Option<u64>,
        turns: u64,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO job_commander_spend (job_id, turn_id, cost_micros, turns) \
                 VALUES (?1, ?2, ?3, ?4) \
                 ON CONFLICT (job_id, turn_id) DO UPDATE SET \
                 cost_micros = excluded.cost_micros, turns = excluded.turns",
                (
                    job_id.as_str(),
                    turn_id,
                    cost_micros.map(|micros| micros as i64),
                    turns as i64,
                ),
            )
            .map(|_| ())
            .map_err(|why| match why {
                rusqlite::Error::SqliteFailure(err, _)
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    WriteError::NoSuchJob {
                        job_id: job_id.clone(),
                    }
                }
                other => WriteError::Database(fault("recording what a Commander spent")(other)),
            })
    }

    /// What every turn of this Job's Commander cost, in millionths of a dollar
    /// and added up. Nought for a Job whose Commander was never woken.
    pub fn commander_cost_for(&self, job_id: &JobId) -> Result<u64, LoadJobError> {
        self.conn
            .query_row(
                "SELECT COALESCE(SUM(cost_micros), 0) FROM job_commander_spend WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| row.get::<_, i64>(0),
            )
            .map(|micros| micros as u64)
            .map_err(|why| {
                LoadJobError::Unreadable(RowError::Database(fault(
                    "reading what a Job's Commander spent",
                )(why)))
            })
    }
}
