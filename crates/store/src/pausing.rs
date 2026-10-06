//! A Job's pause marker, and the slot it gave up in the same breath.
//! `docs/concepts/job.md`, *Pausing*.
//!
//! **One writer for both columns.** A Job whose slot is cleared but whose
//! marker is not yet written reads as one cut before the pool, whose worktree
//! is at a derived path nothing is at; so the two land in one statement.

use core_model::{Job, Pause, PausedBy, Timestamp};
use rusqlite::Row;

use crate::error::{fault, RowError, WriteError};
use crate::open::Store;
use crate::row::{column, maybe};

/// Version 111 — the marker on a paused Job.
///
/// **Null, and no backfill.** Every row before this column is a Job nobody has
/// paused. `pause_by` is the marker's presence, and the other two are only read
/// where it is set. `Store::record_pause` is the one writer.
pub(crate) const V111: &str = r#"
ALTER TABLE jobs ADD COLUMN pause_by TEXT CHECK (pause_by IN ('person', 'fleet'));
ALTER TABLE jobs ADD COLUMN pause_at TEXT;
ALTER TABLE jobs ADD COLUMN pause_resuming INTEGER NOT NULL DEFAULT 0 CHECK (pause_resuming IN (0, 1));
"#;

/// The marker as the row holds it, set on `job` where there is one.
pub(crate) fn read_pause(row: &Row<'_>, job: Job) -> Result<Job, RowError> {
    let Some(by) = maybe(row, "pause_by")? else {
        return Ok(job);
    };
    let malformed = |detail: &str| RowError::MalformedColumn {
        table: "jobs",
        column: "pause_by",
        detail: detail.to_string(),
    };
    let by = PausedBy::from_stored(&by).ok_or_else(|| malformed("neither person nor fleet"))?;
    let at = maybe(row, "pause_at")?.ok_or_else(|| malformed("a pause with no instant"))?;
    let resuming: i64 = row
        .get("pause_resuming")
        .map_err(column("jobs", "pause_resuming"))?;
    Ok(job.with_pause(Pause {
        by,
        at: Timestamp::from_rfc3339(at),
        resuming: resuming != 0,
    }))
}

impl Store {
    /// Write the Job's pause marker and the slot it holds, whichever each is:
    /// both null on a Job just parked, both set on one just reseated. No event,
    /// for `record_slot`'s reason.
    pub fn record_pause(&mut self, job: &Job) -> Result<(), WriteError> {
        let pause = job.pause();
        let updated = self
            .conn
            .execute(
                "UPDATE jobs SET worktree_slot = ?2, pause_by = ?3, pause_at = ?4,
                        pause_resuming = ?5
                 WHERE job_id = ?1",
                (
                    job.id().as_str(),
                    job.worktree_slot().map(i64::from),
                    pause.map(|pause| pause.by.as_str()),
                    pause.map(|pause| pause.at.as_str().to_string()),
                    pause.is_some_and(|pause| pause.resuming),
                ),
            )
            .map_err(fault("recording a job's pause"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: job.id().clone(),
            });
        }
        Ok(())
    }
}
