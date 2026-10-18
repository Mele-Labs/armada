//! Two small per-session rows that `keep_session` would replace whole: the plan's limit a session
//! hit and when it resets, and what fills its context window. `docs/concepts/session.md`.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    /// When the limit this session hit resets, if it is held for one. RFC3339.
    pub fn session_limit(&self, session_id: &str) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row(
                "SELECT resets_at FROM session_limit WHERE session_id = ?1",
                [session_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(fault("reading when a job's limit resets"))
            .map_err(WriteError::Database)
    }

    /// Hold the session for a limit resetting at `resets_at`. Whether that changed what was kept.
    pub fn keep_session_limit(&mut self, session_id: &str, resets_at: &str) -> Result<bool, WriteError> {
        if self.session_limit(session_id)?.as_deref() == Some(resets_at) {
            return Ok(false);
        }
        self.conn
            .execute(
                "INSERT OR REPLACE INTO session_limit (session_id, resets_at) VALUES (?1, ?2)",
                params![session_id, resets_at],
            )
            .map(|_| true)
            .map_err(fault("keeping when a job's limit resets"))
            .map_err(WriteError::Database)
    }

    /// Let the session go. Whether it was held.
    pub fn clear_session_limit(&mut self, session_id: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("DELETE FROM session_limit WHERE session_id = ?1", [session_id])
            .map(|gone| gone > 0)
            .map_err(fault("clearing a job's limit"))
            .map_err(WriteError::Database)
    }

    /// Every session held for a limit, with the instant it resets. RFC3339.
    pub fn sessions_limited(&self) -> Result<Vec<(String, String)>, WriteError> {
        let doing = "listing the jobs held for a limit";
        let mut statement = self
            .conn
            .prepare("SELECT session_id, resets_at FROM session_limit ORDER BY resets_at, session_id")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>()
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }
}

impl Store {
    /// The breakdown of a session's context window, as the wire spells it.
    pub fn session_context(&self, session_id: &str) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row(
                "SELECT breakdown FROM session_context WHERE session_id = ?1",
                [session_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(fault("reading what fills a job's context"))
            .map_err(WriteError::Database)
    }

    /// Keep the latest breakdown. Whether it differs from the one held.
    pub fn keep_session_context(&mut self, session_id: &str, breakdown: &str) -> Result<bool, WriteError> {
        if self.session_context(session_id)?.as_deref() == Some(breakdown) {
            return Ok(false);
        }
        self.conn
            .execute(
                "INSERT OR REPLACE INTO session_context (session_id, breakdown) VALUES (?1, ?2)",
                params![session_id, breakdown],
            )
            .map(|_| true)
            .map_err(fault("keeping what fills a job's context"))
            .map_err(WriteError::Database)
    }
}
