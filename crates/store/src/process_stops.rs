//! When a Job's agent stopped on its own, for the split of a Job's time. When Fleet was away is `fleet_outages.rs`.
//! `docs/concepts/session.md`.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

impl Store {
    /// A Job's agent process ended on its own, as opposed to Fleet letting it go.
    pub fn keep_session_stop(&mut self, session_id: &str, at: &str) -> Result<(), WriteError> {
        self.conn
            .execute("INSERT OR IGNORE INTO session_stops (session_id, at) VALUES (?1, ?2)", params![session_id, at])
            .map(|_| ())
            .map_err(failed("keeping that a job's process stopped"))
    }

    /// When a Job's process stopped on its own, oldest first.
    pub fn session_stops_of(&self, session_id: &str) -> Result<Vec<String>, WriteError> {
        let doing = "reading when a job's process stopped";
        let mut statement = self
            .conn
            .prepare("SELECT at FROM session_stops WHERE session_id = ?1 ORDER BY at")
            .map_err(failed(doing))?;
        statement
            .query_map([session_id], |row| row.get(0))
            .and_then(|rows| rows.collect::<Result<Vec<String>, _>>())
            .map_err(failed(doing))
    }
}
