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
            // Moving the time keeps the pause marker the first snooze kept.
            Some(until) => self.conn.execute(
                "INSERT INTO session_snooze (session_id, until) VALUES (?1, ?2) \
                 ON CONFLICT (session_id) DO UPDATE SET until = excluded.until",
                params![session_id, until],
            ),
            None => self.conn.execute("DELETE FROM session_snooze WHERE session_id = ?1", [session_id]),
        };
        done.map(|_| true).map_err(fault("keeping a job's snooze")).map_err(WriteError::Database)
    }

    /// The pause marker the snooze set when it paused the Job, if it did.
    pub fn snooze_paused_at(&self, session_id: &str) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row("SELECT paused_at FROM session_snooze WHERE session_id = ?1", [session_id], |row| row.get(0))
            .optional()
            .map(Option::flatten)
            .map_err(fault("reading whether a job's snooze paused it"))
            .map_err(WriteError::Database)
    }

    /// Keep the pause marker the snooze set, on a Job that is snoozed.
    pub fn keep_snooze_paused_at(&mut self, session_id: &str, paused_at: &str) -> Result<(), WriteError> {
        self.conn
            .execute("UPDATE session_snooze SET paused_at = ?2 WHERE session_id = ?1", params![session_id, paused_at])
            .map(|_| ())
            .map_err(fault("keeping that a job's snooze paused it"))
            .map_err(WriteError::Database)
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
