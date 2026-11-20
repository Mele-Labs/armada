//! The one Job the person prioritised. A single row, so a second mark cannot be kept, and the id is
//! kept when that Job ends: the reader leaves an ended Job unprioritised.

use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    /// The prioritised Job's id, or `None` where none is. It may name a Job that has since ended.
    pub fn prioritised_session(&self) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row("SELECT session_id FROM session_priority", [], |row| row.get(0))
            .map_err(fault("reading which job is prioritised"))
            .map_err(WriteError::Database)
    }

    /// Prioritise this Job in place of any other, or none with `None`.
    pub fn keep_prioritised_session(&mut self, session_id: Option<&str>) -> Result<(), WriteError> {
        self.conn
            .execute("UPDATE session_priority SET session_id = ?1", [session_id])
            .map(|_| ())
            .map_err(fault("keeping which job is prioritised"))
            .map_err(WriteError::Database)
    }
}
