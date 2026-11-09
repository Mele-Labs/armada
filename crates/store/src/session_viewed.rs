//! The last instant the person was in a Job, as a client told it (`view_session`). Fleet's clock, so
//! two clients' visits compare. `docs/practices/protocol/a-jobs-row-mark.md`.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    /// When he was last in the Job, RFC 3339. Absent where he never was.
    pub fn session_viewed(&self, session_id: &str) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row("SELECT viewed_at FROM session_viewed WHERE session_id = ?1", [session_id], |row| row.get(0))
            .optional()
            .map_err(fault("reading when the person was last in a job"))
            .map_err(WriteError::Database)
    }

    /// He was in the Job at `at`. An earlier instant than the one kept changes nothing.
    pub fn keep_session_viewed(&mut self, session_id: &str, at: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO session_viewed (session_id, viewed_at) VALUES (?1, ?2)
                 ON CONFLICT (session_id) DO UPDATE SET viewed_at = MAX(viewed_at, excluded.viewed_at)",
                params![session_id, at],
            )
            .map(|_| ())
            .map_err(fault("keeping when the person was last in a job"))
            .map_err(WriteError::Database)
    }
}
