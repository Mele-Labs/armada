//! The dispatches the Operator holds: the person's work, kept whole until a Job is started on it or it is
//! folded into a live one. **The body is Fleet's, opaque here.**

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One held dispatch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptDispatch {
    pub id: String,
    pub body: String,
    /// Whether it waits on his go-ahead, which a restart leaves standing.
    pub asked: bool,
}

impl Store {
    /// Keep the dispatch `id`, `body` whole. Kept once: a second under the same id is not written.
    pub fn keep_operator_dispatch(&mut self, id: &str, body: &str, at: &str) -> Result<(), WriteError> {
        self.conn
            .execute("INSERT OR IGNORE INTO operator_dispatches (id, body, created_at) VALUES (?1, ?2, ?3)", params![id, body, at])
            .map(|_| ())
            .map_err(fault("keeping a dispatch the Operator holds"))
            .map_err(WriteError::Database)
    }

    /// The dispatch `id`, where it is still held.
    pub fn operator_dispatch(&self, id: &str) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row("SELECT body FROM operator_dispatches WHERE id = ?1", [id], |row| row.get(0))
            .optional()
            .map_err(fault("reading a dispatch the Operator holds"))
            .map_err(WriteError::Database)
    }

    /// Every dispatch held, oldest first.
    pub fn operator_dispatches(&self) -> Result<Vec<KeptDispatch>, WriteError> {
        let doing = "reading the dispatches the Operator holds";
        let mut statement = self
            .conn
            .prepare("SELECT id, body, asked FROM operator_dispatches ORDER BY created_at, id")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([], |row| Ok(KeptDispatch { id: row.get(0)?, body: row.get(1)?, asked: row.get::<_, i64>(2)? != 0 }))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// Mark the dispatch `id` as waiting on his go-ahead.
    pub fn operator_dispatch_asked(&mut self, id: &str) -> Result<(), WriteError> {
        self.conn
            .execute("UPDATE operator_dispatches SET asked = 1 WHERE id = ?1", [id])
            .map(|_| ())
            .map_err(fault("marking a dispatch asked"))
            .map_err(WriteError::Database)
    }

    /// The dispatch `id`, taken: read and removed, so the one that acts on it acts once. `None` where it is gone.
    pub fn take_operator_dispatch(&mut self, id: &str) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row("DELETE FROM operator_dispatches WHERE id = ?1 RETURNING body", [id], |row| row.get(0))
            .optional()
            .map_err(fault("taking a dispatch the Operator holds"))
            .map_err(WriteError::Database)
    }
}
