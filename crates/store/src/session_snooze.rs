//! When a snoozed Job comes back. A table of its own, so `keep_hosting` and `keep_session`, which
//! replace a whole row, leave it be. `docs/concepts/session/snoozing-a-job.md`.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    /// When this Job's snooze ends, if it is snoozed. RFC3339.
    pub fn session_snooze(&self, session_id: &str) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row("SELECT until FROM session_snooze WHERE session_id = ?1", [session_id], |row| row.get(0))
            .optional()
            .map_err(fault("reading when a job's snooze ends"))
            .map_err(WriteError::Database)
    }

    /// Snooze the Job until `until`, or wake it with `None`. Whether that changed what was kept.
    pub fn keep_session_snooze(&mut self, session_id: &str, until: Option<&str>) -> Result<bool, WriteError> {
        if self.session_snooze(session_id)?.as_deref() == until {
            return Ok(false);
        }
        let done = match until {
            Some(until) => self.conn.execute(
                "INSERT OR REPLACE INTO session_snooze (session_id, until) VALUES (?1, ?2)",
                params![session_id, until],
            ),
            None => self.conn.execute("DELETE FROM session_snooze WHERE session_id = ?1", [session_id]),
        };
        done.map(|_| true).map_err(fault("keeping a job's snooze")).map_err(WriteError::Database)
    }

    /// Every snoozed Job, with the instant its snooze ends. RFC3339.
    pub fn sessions_snoozed(&self) -> Result<Vec<(String, String)>, WriteError> {
        let doing = "listing the snoozed jobs";
        let mut statement = self
            .conn
            .prepare("SELECT session_id, until FROM session_snooze ORDER BY until, session_id")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }
}
