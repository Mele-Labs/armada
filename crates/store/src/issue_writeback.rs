//! What Fleet has written back on the issue a Job came from. #91,
//! `docs/concepts/tracker.md`.
//!
//! **Kept so nothing is said twice.** A comment is not idempotent on the
//! tracker, so a restart that forgot it had said a Job started would say it
//! again; what was said is a row, and the next pass reads it.

use core_model::{IssueSource, JobId, Timestamp, Ulid};

use crate::error::{fault, LoadJobError, RowError, WriteError};
use crate::open::Store;

/// What has been written on one Job's issue so far. [`Default`] is nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WrittenBack {
    /// The issue was told a Job started on it.
    pub started: bool,
    /// The pull request the issue was last told of.
    pub pull_request: Option<String>,
    /// Armada's label the issue carries, by its stored word: `in_progress` or
    /// `set_aside`. `None` is none of Armada's.
    pub label: Option<String>,
    /// Nothing is left to write: the Job ended and its label is off.
    pub settled: bool,
    /// The last write the tracker refused, in its words, and when.
    pub failed: Option<String>,
    pub failed_at: Option<Timestamp>,
}

/// One Job whose issue may be owed a write, with what the write is read from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteBackOwed {
    pub job_id: JobId,
    pub number: u64,
    pub title: String,
    /// The Job's status as the store holds it.
    pub status: String,
    pub source: IssueSource,
    /// The pull request the Job opened, where it opened one.
    pub pull_request: Option<String>,
    pub kept: WrittenBack,
}

fn unreadable(cause: rusqlite::Error) -> LoadJobError {
    LoadJobError::Unreadable(RowError::Database(fault(
        "reading what was written back on issues",
    )(cause)))
}

impl Store {
    /// Every Job that came from an issue and is not settled, oldest first.
    pub fn issue_writebacks_open(&self) -> Result<Vec<WriteBackOwed>, LoadJobError> {
        let mut asked = self
            .conn
            .prepare(
                "SELECT s.job_id, j.number, j.title, j.status, s.reference, s.url, s.read_at,
                        s.moved_at, j.delivery_pull_request, w.started, w.pull_request,
                        w.label, w.failed, w.failed_at
                 FROM job_issue_sources AS s
                 JOIN jobs AS j ON j.job_id = s.job_id
                 LEFT JOIN job_issue_writeback AS w ON w.job_id = s.job_id
                 WHERE w.settled IS NULL OR w.settled = 0
                 ORDER BY j.created_at, s.job_id",
            )
            .map_err(unreadable)?;
        let rows = asked
            .query_map((), |row| {
                Ok(WriteBackOwed {
                    job_id: JobId::carried(Ulid::carried(row.get::<_, String>(0)?)),
                    number: row.get::<_, i64>(1)?.max(0) as u64,
                    title: row.get(2)?,
                    status: row.get(3)?,
                    source: IssueSource {
                        reference: row.get(4)?,
                        url: row.get(5)?,
                        read_at: Timestamp::from_rfc3339(row.get::<_, String>(6)?),
                        moved_at: row.get::<_, Option<String>>(7)?.map(Timestamp::from_rfc3339),
                    },
                    pull_request: row.get(8)?,
                    kept: WrittenBack {
                        started: row.get::<_, Option<i64>>(9)?.unwrap_or(0) != 0,
                        pull_request: row.get(10)?,
                        label: row.get(11)?,
                        settled: false,
                        failed: row.get(12)?,
                        failed_at: row.get::<_, Option<String>>(13)?.map(Timestamp::from_rfc3339),
                    },
                })
            })
            .map_err(unreadable)?;
        rows.collect::<Result<Vec<_>, rusqlite::Error>>()
            .map_err(unreadable)
    }

    /// What has been written on `job_id`'s issue, where anything has.
    pub fn issue_written_back(&self, job_id: &JobId) -> Result<Option<WrittenBack>, LoadJobError> {
        self.conn
            .query_row(
                "SELECT started, pull_request, label, settled, failed, failed_at
                 FROM job_issue_writeback WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| {
                    Ok(WrittenBack {
                        started: row.get::<_, i64>(0)? != 0,
                        pull_request: row.get(1)?,
                        label: row.get(2)?,
                        settled: row.get::<_, i64>(3)? != 0,
                        failed: row.get(4)?,
                        failed_at: row.get::<_, Option<String>>(5)?.map(Timestamp::from_rfc3339),
                    })
                },
            )
            .map(Some)
            .or_else(|why| match why {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(unreadable(other)),
            })
    }

    /// Keep what has been written on `job_id`'s issue, whole.
    pub fn record_written_back(
        &mut self,
        job_id: &JobId,
        kept: &WrittenBack,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO job_issue_writeback
                     (job_id, started, pull_request, label, settled, failed, failed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT (job_id) DO UPDATE SET
                     started = excluded.started, pull_request = excluded.pull_request,
                     label = excluded.label, settled = excluded.settled,
                     failed = excluded.failed, failed_at = excluded.failed_at",
                rusqlite::params![
                    job_id.as_str(),
                    i64::from(kept.started),
                    kept.pull_request,
                    kept.label,
                    i64::from(kept.settled),
                    kept.failed,
                    kept.failed_at.as_ref().map(Timestamp::as_str),
                ],
            )
            .map_err(fault("keeping what was written back on an issue"))
            .map_err(WriteError::Database)?;
        Ok(())
    }
}
