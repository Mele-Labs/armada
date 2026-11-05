//! The line for a slot, as Fleet keeps it across a restart. Written by `fleet`'s `queueing` as a
//! Job joins and leaves it; read once, as Fleet starts.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One Job in a repository's line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptInLine {
    pub session_id: String,
    pub manifest_id: String,
    /// Lowest is served first.
    pub place: i64,
    pub since: String,
}

fn failed(doing: &'static str) -> impl FnOnce(rusqlite::Error) -> WriteError {
    move |cause| WriteError::Database(fault(doing)(cause))
}

impl Store {
    /// Keep `one` in its line, at its place. A Job kept again takes the place named last.
    pub fn keep_in_slot_line(&mut self, one: &KeptInLine) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO slot_line (session_id, manifest_id, place, since) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (session_id) DO UPDATE SET manifest_id = excluded.manifest_id, place = excluded.place",
                params![one.session_id, one.manifest_id, one.place, one.since],
            )
            .map(|_| ())
            .map_err(failed("keeping a Job in the line for a slot"))
    }

    /// `session` is in no line now.
    pub fn leave_slot_line(&mut self, session: &str) -> Result<(), WriteError> {
        self.conn
            .execute("DELETE FROM slot_line WHERE session_id = ?1", [session])
            .map(|_| ())
            .map_err(failed("taking a Job out of the line for a slot"))
    }

    /// Every line, each repository's in its order.
    pub fn slot_lines(&self) -> Result<Vec<KeptInLine>, WriteError> {
        let doing = "reading the lines for a slot";
        let mut statement = self
            .conn
            .prepare("SELECT session_id, manifest_id, place, since FROM slot_line ORDER BY manifest_id, place, since")
            .map_err(failed(doing))?;
        let rows = statement
            .query_map([], |row| Ok(KeptInLine { session_id: row.get(0)?, manifest_id: row.get(1)?, place: row.get(2)?, since: row.get(3)? }))
            .map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }
}
