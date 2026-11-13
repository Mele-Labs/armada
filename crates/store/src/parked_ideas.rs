//! The ideas a dispatch was kept as, for later. **The body is Fleet's, opaque here.**

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One idea kept for later.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptIdea {
    pub id: String,
    pub body: String,
    pub created_at: String,
}

impl Store {
    /// Keep the idea `id`, `body` whole. Kept once: a second under the same id is not written.
    pub fn keep_parked_idea(&mut self, id: &str, body: &str, at: &str) -> Result<(), WriteError> {
        self.conn
            .execute("INSERT OR IGNORE INTO parked_ideas (id, body, created_at) VALUES (?1, ?2, ?3)", params![id, body, at])
            .map(|_| ())
            .map_err(fault("keeping an idea for later"))
            .map_err(WriteError::Database)
    }

    /// Every idea kept, oldest first.
    pub fn parked_ideas(&self) -> Result<Vec<KeptIdea>, WriteError> {
        let doing = "reading the ideas kept for later";
        let mut statement = self
            .conn
            .prepare("SELECT id, body, created_at FROM parked_ideas ORDER BY created_at, id")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([], |row| Ok(KeptIdea { id: row.get(0)?, body: row.get(1)?, created_at: row.get(2)? }))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// The idea `id` removed. Whether it was there.
    pub fn remove_parked_idea(&mut self, id: &str) -> Result<bool, WriteError> {
        self.conn
            .query_row("DELETE FROM parked_ideas WHERE id = ?1 RETURNING id", [id], |row| row.get::<_, String>(0))
            .optional()
            .map(|found| found.is_some())
            .map_err(fault("removing an idea kept for later"))
            .map_err(WriteError::Database)
    }
}
