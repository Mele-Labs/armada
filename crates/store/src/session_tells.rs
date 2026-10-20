//! The last Fleet tell a Session was sent that its Job may turn down in prose, and the first reply after it.
//! `docs/spikes/tell-refusal-reader.md`.
//!
//! **One row a session, for its latest tell.** A tell sent again replaces the row, so a refusal is
//! always of the tell it followed. `first_end` is set once, by the first reply kept after the tell
//! ([`Store::keep_turn_end`]), which is the only reply read for a refusal.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// A tell, and the reply that first followed it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptTell {
    /// Which tell: `stranded_slot`, `context` or `subagents`.
    pub kind: String,
    /// When Fleet sent it. RFC3339.
    pub at: String,
    /// When the first reply after it ended, once there is one.
    pub first_end: Option<String>,
}

impl Store {
    /// Keep that `session_id` was sent a tell of `kind` at `at`, in place of any earlier one.
    pub fn keep_tell(&mut self, session_id: &str, kind: &str, at: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO session_tells (session_id, kind, at, first_end) VALUES (?1, ?2, ?3, NULL)
                 ON CONFLICT (session_id) DO UPDATE SET kind = ?2, at = ?3, first_end = NULL",
                params![session_id, kind, at],
            )
            .map(|_| ())
            .map_err(fault("keeping a tell a job was sent"))
            .map_err(WriteError::Database)
    }

    /// The last tell the session was sent, if any.
    pub fn tell_of(&self, session_id: &str) -> Result<Option<KeptTell>, WriteError> {
        self.conn
            .query_row(
                "SELECT kind, at, first_end FROM session_tells WHERE session_id = ?1",
                [session_id],
                |row| Ok(KeptTell { kind: row.get(0)?, at: row.get(1)?, first_end: row.get(2)? }),
            )
            .optional()
            .map_err(fault("reading the tell a job was last sent"))
            .map_err(WriteError::Database)
    }
}
