//! The Checks a Session's agent ran with `armada check` in its slot, one row a run. `docs/concepts/session.md`.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One run, without its log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptSessionCheck {
    pub id: i64,
    pub session_id: String,
    pub manifest_id: String,
    pub slot: u32,
    pub name: String,
    /// `running`, `passed` or `failed`.
    pub state: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub took_ms: Option<u64>,
}

const COLUMNS: &str = "id, session_id, manifest_id, slot, name, state, started_at, ended_at, took_ms";

fn kept(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptSessionCheck> {
    Ok(KeptSessionCheck {
        id: row.get(0)?,
        session_id: row.get(1)?,
        manifest_id: row.get(2)?,
        slot: row.get(3)?,
        name: row.get(4)?,
        state: row.get(5)?,
        started_at: row.get(6)?,
        ended_at: row.get(7)?,
        took_ms: row.get::<_, Option<i64>>(8)?.map(|ms| ms.max(0) as u64),
    })
}

impl Store {
    /// A run begins. Answers its id.
    pub fn start_session_check(
        &mut self,
        session_id: &str,
        manifest_id: &str,
        slot: u32,
        name: &str,
        at: &str,
    ) -> Result<i64, WriteError> {
        let doing = "keeping a Session's Check run";
        self.conn
            .execute(
                "INSERT INTO session_check_runs (session_id, manifest_id, slot, name, state, started_at)
                 VALUES (?1, ?2, ?3, ?4, 'running', ?5)",
                params![session_id, manifest_id, slot, name, at],
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        Ok(self.conn.last_insert_rowid())
    }

    /// A run ends. **False where it was not running**, so an end is kept once.
    pub fn end_session_check(
        &mut self,
        run: i64,
        state: &str,
        at: &str,
        took_ms: u64,
        log: &str,
    ) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE session_check_runs
                 SET state = ?2, ended_at = ?3, took_ms = ?4, log = ?5
                 WHERE id = ?1 AND state = 'running'",
                params![run, state, at, took_ms as i64, log],
            )
            .map(|changed| changed > 0)
            .map_err(fault("ending a Session's Check run"))
            .map_err(WriteError::Database)
    }

    pub fn session_check(&self, run: i64) -> Result<Option<KeptSessionCheck>, WriteError> {
        self.conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM session_check_runs WHERE id = ?1"),
                [run],
                kept,
            )
            .optional()
            .map_err(fault("reading a Session's Check run"))
            .map_err(WriteError::Database)
    }

    /// A run's log, `None` where there is no such run.
    pub fn session_check_log(&self, run: i64) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row("SELECT log FROM session_check_runs WHERE id = ?1", [run], |row| {
                row.get(0)
            })
            .optional()
            .map_err(fault("reading a Session's Check log"))
            .map_err(WriteError::Database)
    }

    /// The newest `most` runs, newest first. `manifest_id` narrows to one repository's.
    pub fn session_checks(
        &self,
        manifest_id: Option<&str>,
        most: u32,
    ) -> Result<Vec<KeptSessionCheck>, WriteError> {
        let doing = "reading the Sessions' Check runs";
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM session_check_runs
                 WHERE (?1 IS NULL OR manifest_id = ?1) ORDER BY id DESC LIMIT ?2"
            ))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map(params![manifest_id, most], kept)
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>()
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }
}
