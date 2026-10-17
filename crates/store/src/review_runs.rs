//! What an agent said about one pull request, kept per commit. `docs/concepts/pull-review.md`,
//! *Agent runs*.
//!
//! **The result is opaque text here**: the JSON of the answer `fleet` validated, because the shape
//! of an answer is the wire's and not this crate's. `state` is text for the same reason `kind` is.

use rusqlite::{OptionalExtension, Row};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One row of `review_runs`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptReviewRun {
    pub manifest_id: String,
    pub number: u64,
    pub head: String,
    pub kind: String,
    /// The key: a definition's id, or a question's words lower cased and single spaced.
    pub name: String,
    /// What the wire calls the run: the id, or the question as it was first asked.
    pub label: String,
    pub state: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub failed: Option<String>,
    pub result: Option<String>,
    pub cost_micros: Option<u64>,
    /// The bay a running row leased, and who holds it, until it is given back.
    pub slot: Option<u32>,
    pub holder: Option<String>,
}

const COLUMNS: &str = "manifest_id, number, head, kind, name, label, state, started_at, finished_at, \
                       failed, result, cost_micros, slot, holder";

fn kept(row: &Row<'_>) -> rusqlite::Result<KeptReviewRun> {
    Ok(KeptReviewRun {
        manifest_id: row.get(0)?,
        number: row.get::<_, i64>(1)?.try_into().unwrap_or_default(),
        head: row.get(2)?,
        kind: row.get(3)?,
        name: row.get(4)?,
        label: row.get(5)?,
        state: row.get(6)?,
        started_at: row.get(7)?,
        finished_at: row.get(8)?,
        failed: row.get(9)?,
        result: row.get(10)?,
        cost_micros: row.get::<_, Option<i64>>(11)?.and_then(|micros| micros.try_into().ok()),
        slot: row.get::<_, Option<i64>>(12)?.and_then(|slot| slot.try_into().ok()),
        holder: row.get(13)?,
    })
}

impl Store {
    /// Keep a run, replacing the row at its key.
    pub fn keep_review_run(&mut self, run: &KeptReviewRun) -> Result<(), WriteError> {
        self.conn
            .execute(
                &format!(
                    "INSERT OR REPLACE INTO review_runs ({COLUMNS}) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"
                ),
                rusqlite::params![
                    run.manifest_id,
                    i64::try_from(run.number).unwrap_or(i64::MAX),
                    run.head,
                    run.kind,
                    run.name,
                    run.label,
                    run.state,
                    run.started_at,
                    run.finished_at,
                    run.failed,
                    run.result,
                    run.cost_micros.and_then(|micros| i64::try_from(micros).ok()),
                    run.slot.map(i64::from),
                    run.holder,
                ],
            )
            .map(|_| ())
            .map_err(fault("keeping a review run"))
            .map_err(WriteError::Database)
    }

    /// The run at one key, if there is one.
    pub fn review_run(
        &self,
        manifest_id: &str,
        number: u64,
        head: &str,
        kind: &str,
        name: &str,
    ) -> Result<Option<KeptReviewRun>, WriteError> {
        self.conn
            .query_row(
                &format!(
                    "SELECT {COLUMNS} FROM review_runs \
                     WHERE manifest_id = ?1 AND number = ?2 AND head = ?3 AND kind = ?4 AND name = ?5"
                ),
                rusqlite::params![manifest_id, i64::try_from(number).unwrap_or(i64::MAX), head, kind, name],
                kept,
            )
            .optional()
            .map_err(fault("reading a review run"))
            .map_err(WriteError::Database)
    }

    /// Every run kept or going for one pull request, newest started first. The same key at an older
    /// head is a run of its own.
    pub fn review_runs(&self, manifest_id: &str, number: u64) -> Result<Vec<KeptReviewRun>, WriteError> {
        let unreadable = |why: rusqlite::Error| WriteError::Database(fault("reading review runs")(why));
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM review_runs WHERE manifest_id = ?1 AND number = ?2 \
                 ORDER BY started_at DESC, head, kind, name"
            ))
            .map_err(unreadable)?;
        let rows = statement
            .query_map(rusqlite::params![manifest_id, i64::try_from(number).unwrap_or(i64::MAX)], kept)
            .map_err(unreadable)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(unreadable)
    }

    /// Every run that has not ended, across every pull request. A restart ends them.
    pub fn review_runs_going(&self) -> Result<Vec<KeptReviewRun>, WriteError> {
        let unreadable = |why: rusqlite::Error| WriteError::Database(fault("reading going review runs")(why));
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM review_runs WHERE state IN ('queued', 'running')"))
            .map_err(unreadable)?;
        let rows = statement.query_map([], kept).map_err(unreadable)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(unreadable)
    }

    /// Forget a run at every head it ran on. One that is not kept is not an error. A search is kept at
    /// the one head there is, `""`.
    pub fn forget_review_run(&mut self, manifest_id: &str, number: u64, kind: &str, name: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "DELETE FROM review_runs WHERE manifest_id = ?1 AND number = ?2 AND kind = ?3 AND name = ?4",
                rusqlite::params![manifest_id, i64::try_from(number).unwrap_or(i64::MAX), kind, name],
            )
            .map(|_| ())
            .map_err(fault("forgetting a review run"))
            .map_err(WriteError::Database)
    }
}
