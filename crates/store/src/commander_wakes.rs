//! What woke a Job's Commander, kept so a fact wakes it once. `#2167`.
//!
//! **In the store and not in Fleet's memory**, so a restart cannot wake a
//! Commander again for a stall it was woken for before. The row is the wake:
//! Fleet claims it first and wakes after, so a crash between the two loses a
//! wake rather than repeating one.

use core_model::JobId;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One problem and the run of it a wake is for.
///
/// `trigger` and `step` say what the problem is, and `attempt` which run of the
/// step met it. A Job-level trouble has no step and passes it empty; a cap has
/// none either and passes the cap in force as `attempt`, so raising it re-arms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommanderProblem {
    pub trigger: String,
    pub step: String,
    pub attempt: u64,
}

/// What recording a wake found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommanderWake {
    /// This fact has been seen, and its wake belongs to the first time.
    Already,
    /// A fact not seen before. `run` is how many wakes in a row on this trigger
    /// and step have found the Job exactly as it was, counting this one.
    New { run: u32 },
}

impl Store {
    /// How many wakes have been noted for this trigger on this step, whatever
    /// the attempt. For a trouble that does not move the step's attempt, so
    /// that its next fact is not read as one already seen.
    pub fn commander_wakes_noted(
        &self,
        job_id: &JobId,
        trigger: &str,
        step: &str,
    ) -> Result<u64, crate::error::RowError> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM job_commander_wakes \
                 WHERE job_id = ?1 AND problem = ?2 AND step = ?3",
                (job_id.as_str(), trigger, step),
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count as u64)
            .map_err(|why| {
                crate::error::RowError::Database(fault("counting a Commander's wakes")(why))
            })
    }

    /// Write down that a Commander is to be woken for this fact, unless it has
    /// been. `standing` is the Job as it is now, in whatever words the caller
    /// compares it by: a wake whose `standing` differs from the last one's on
    /// the same trigger and step starts the count again.
    pub fn note_commander_wake(
        &mut self,
        job_id: &JobId,
        problem: &CommanderProblem,
        standing: &str,
    ) -> Result<CommanderWake, WriteError> {
        let database = |step| move |why| WriteError::Database(fault(step)(why));
        let tx = self
            .conn
            .transaction()
            .map_err(database("starting a Commander wake"))?;
        let inserted = tx
            .execute(
                "INSERT OR IGNORE INTO job_commander_wakes \
                 (job_id, problem, step, attempt, standing, run) VALUES (?1, ?2, ?3, ?4, ?5, 0)",
                (
                    job_id.as_str(),
                    &problem.trigger,
                    &problem.step,
                    problem.attempt as i64,
                    standing,
                ),
            )
            .map_err(|why| match why {
                rusqlite::Error::SqliteFailure(err, _)
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    WriteError::NoSuchJob {
                        job_id: job_id.clone(),
                    }
                }
                other => WriteError::Database(fault("recording a Commander wake")(other)),
            })?;
        if inserted == 0 {
            return Ok(CommanderWake::Already);
        }
        let seq = tx.last_insert_rowid();
        let before: Option<(String, i64)> = match tx.query_row(
            "SELECT standing, run FROM job_commander_wakes \
             WHERE job_id = ?1 AND problem = ?2 AND step = ?3 AND seq < ?4 \
             ORDER BY seq DESC LIMIT 1",
            (job_id.as_str(), &problem.trigger, &problem.step, seq),
            |row| Ok((row.get(0)?, row.get(1)?)),
        ) {
            Ok(found) => Some(found),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(other) => return Err(database("reading the last Commander wake")(other)),
        };
        let run = match before {
            Some((was, run)) if was == standing => run as u32 + 1,
            _ => 1,
        };
        tx.execute(
            "UPDATE job_commander_wakes SET run = ?1 WHERE seq = ?2",
            (run as i64, seq),
        )
        .map_err(database("counting a Commander wake"))?;
        tx.commit().map_err(database("committing a Commander wake"))?;
        Ok(CommanderWake::New { run })
    }
}
