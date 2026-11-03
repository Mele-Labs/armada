//! The stop each Job was told about and the stop passed over. `docs/concepts/session/what-a-job-is-doing.md`.

use rusqlite::OptionalExtension;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// What is kept of a Job's stops. A stop is named by when its turn ended.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptStop {
    /// The stop the nudge was about, and when it went. Clear once anything else woke the Job.
    pub told: Option<(String, String)>,
    /// The nudge reached nobody: it was muted, withheld or refused.
    pub unheard: bool,
    /// The stop never read as one: the person's Stop press, or one standing when this was first kept.
    pub passed_over: Option<String>,
}

impl Store {
    /// Every live session, hosted or in a terminal.
    pub fn live_session_ids(&self) -> Result<Vec<String>, WriteError> {
        let doing = "reading the live sessions";
        let mut statement =
            self.conn.prepare("SELECT id FROM sessions WHERE state = 'live'").map_err(fault(doing)).map_err(WriteError::Database)?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0)).map_err(fault(doing)).map_err(WriteError::Database)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }

    pub fn session_stop(&self, session: &str) -> Result<KeptStop, WriteError> {
        self.conn
            .query_row(
                "SELECT told_stop, told_at, unheard, passed_over FROM stop_nudges WHERE session_id = ?1",
                [session],
                |row| {
                    let told: (Option<String>, Option<String>) = (row.get(0)?, row.get(1)?);
                    Ok(KeptStop { told: told.0.zip(told.1), unheard: row.get(2)?, passed_over: row.get(3)? })
                },
            )
            .optional()
            .map(Option::unwrap_or_default)
            .map_err(fault("reading a Job's stop"))
            .map_err(WriteError::Database)
    }

    /// Keep that `session` was told about the stop `stop` at `at`.
    pub fn keep_stop_told(&mut self, session: &str, stop: &str, at: &str, unheard: bool) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO stop_nudges (session_id, told_stop, told_at, unheard) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (session_id) DO UPDATE SET told_stop = ?2, told_at = ?3, unheard = ?4",
                (session, stop, at, unheard),
            )
            .map(|_| ())
            .map_err(fault("keeping that a Job was told of its stop"))
            .map_err(WriteError::Database)
    }

    /// Something else woke `session`, so the stop after it gets a nudge of its own.
    pub fn clear_stop_told(&mut self, session: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE stop_nudges SET told_stop = NULL, told_at = NULL, unheard = 0 WHERE session_id = ?1 AND told_stop IS NOT NULL",
                [session],
            )
            .map(|_| ())
            .map_err(fault("clearing that a Job was told of its stop"))
            .map_err(WriteError::Database)
    }

    /// Never read `stop` as a stop of `session`.
    pub fn pass_over_stop(&mut self, session: &str, stop: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO stop_nudges (session_id, passed_over) VALUES (?1, ?2)
                 ON CONFLICT (session_id) DO UPDATE SET passed_over = ?2",
                (session, stop),
            )
            .map(|_| ())
            .map_err(fault("passing over a Job's stop"))
            .map_err(WriteError::Database)
    }
}
