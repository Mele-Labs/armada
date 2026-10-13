//! Which bay a Job held, and when it took it and gave it back.
//!
//! **A row a stint, appended.** The session ledger keeps a Job's slot as one row
//! a slot, so a paused Job that comes back to the slot it left has overwritten
//! its first stint. This is the history a Job's trail reads: "was in bay 3,
//! 09:10 to 11:40". Written where Fleet takes and gives back a slot
//! (`crates/fleet/src/ledgering.rs`); no row for a Job that never held one.

use core_model::JobId;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One stretch of a Job in one bay. Instants are RFC3339, stamped by Fleet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptBay {
    pub slot: u32,
    pub since: String,
    /// Absent while the Job still holds the bay.
    pub until: Option<String>,
}

impl Store {
    /// The Job took `slot` at `at`. **Idempotent**: a stint already open in this
    /// slot is left as it is, and one open in another is closed at `at`, since a
    /// Job is in one bay at a time.
    pub fn bay_taken(&mut self, job_id: &JobId, slot: u32, at: &str) -> Result<(), WriteError> {
        let wrap = |why| WriteError::Database(fault("recording a Job's bay")(why));
        let id = job_id.as_str();
        let held: Option<i64> = self
            .conn
            .query_row(
                "SELECT slot FROM job_bays WHERE job_id = ?1 AND until IS NULL",
                (id,),
                |row| row.get(0),
            )
            .map(Some)
            .or_else(|why| match why {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
            .map_err(wrap)?;
        if held == Some(i64::from(slot)) {
            return Ok(());
        }
        let transaction = self.conn.transaction().map_err(wrap)?;
        transaction
            .execute(
                "UPDATE job_bays SET until = ?2 WHERE job_id = ?1 AND until IS NULL",
                (id, at),
            )
            .map_err(wrap)?;
        transaction
            .execute(
                "INSERT INTO job_bays (job_id, slot, since) VALUES (?1, ?2, ?3)",
                (id, i64::from(slot), at),
            )
            .map_err(|why| match why {
                rusqlite::Error::SqliteFailure(err, _)
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    WriteError::NoSuchJob {
                        job_id: job_id.clone(),
                    }
                }
                other => wrap(other),
            })?;
        transaction.commit().map_err(wrap)
    }

    /// The Job gave its bay back at `at`. Nothing is wrong with a Job that held none.
    pub fn bay_given_back(&mut self, job_id: &JobId, at: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE job_bays SET until = ?2 WHERE job_id = ?1 AND until IS NULL",
                (job_id.as_str(), at),
            )
            .map(|_| ())
            .map_err(|why| WriteError::Database(fault("giving a Job's bay back")(why)))
    }

    /// Every stint of the Job, oldest first.
    pub fn bays_of(&self, job_id: &JobId) -> Result<Vec<KeptBay>, WriteError> {
        let wrap = |why| WriteError::Database(fault("reading a Job's bays")(why));
        let mut statement = self
            .conn
            .prepare("SELECT slot, since, until FROM job_bays WHERE job_id = ?1 ORDER BY id")
            .map_err(wrap)?;
        let rows = statement
            .query_map((job_id.as_str(),), |row| {
                Ok(KeptBay {
                    slot: u32::try_from(row.get::<_, i64>(0)?).unwrap_or(0),
                    since: row.get(1)?,
                    until: row.get(2)?,
                })
            })
            .map_err(wrap)?;
        rows.collect::<Result<_, _>>().map_err(wrap)
    }
}
