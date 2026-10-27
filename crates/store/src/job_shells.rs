//! The shell each Job's agent last typed into or read. A shell is never written down and a restart
//! ends it, so this row is what says the agent's next shell is a replacement.

use core_model::Timestamp;
use rusqlite::OptionalExtension;

use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    /// The run id of the shell `session_id`'s agent last typed into or read, absent where it never has.
    pub fn job_shell_seen(&self, session_id: &str) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row("SELECT run_id FROM job_shell_seen WHERE session_id = ?1", [session_id], |row| row.get(0))
            .optional()
            .map_err(fault("reading the shell a job's agent last saw"))
            .map_err(WriteError::Database)
    }

    /// `session_id`'s agent has now typed into or read the shell `run_id`.
    pub fn keep_job_shell_seen(&mut self, session_id: &str, run_id: &str, at: &Timestamp) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO job_shell_seen (session_id, run_id, seen_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT (session_id) DO UPDATE SET run_id = excluded.run_id, seen_at = excluded.seen_at",
                (session_id, run_id, at.as_str()),
            )
            .map(|_| ())
            .map_err(fault("keeping the shell a job's agent last saw"))
            .map_err(WriteError::Database)
    }
}
