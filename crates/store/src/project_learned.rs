//! What Fleet learned for a project, kept apart from the project's file so the file holds only
//! what a person could have written.
//!
//! **The word is the wire's and is not looked up here**: Fleet names it and decides when the note
//! still stands. Deleting a project drops its notes with it, in [`Store::delete_project`].

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    /// Keep that `key` of `project` was learned as `word`, replacing an earlier note for the same key.
    pub fn keep_project_learned(&mut self, project: &str, key: &str, word: &str, at: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO project_learned (project, key, word, learned_at) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (project, key) DO UPDATE SET word = excluded.word, learned_at = excluded.learned_at",
                params![project, key, word, at],
            )
            .map(|_| ())
            .map_err(fault("keeping what a project learned"))
            .map_err(WriteError::Database)
    }

    /// The word `key` of `project` was learned as, `None` where no note stands.
    pub fn project_learned(&self, project: &str, key: &str) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row("SELECT word FROM project_learned WHERE project = ?1 AND key = ?2", params![project, key], |row| row.get(0))
            .optional()
            .map_err(fault("reading what a project learned"))
            .map_err(WriteError::Database)
    }

    /// Drop the note for `key` of `project`. A key with no note is left as it was.
    pub fn clear_project_learned(&mut self, project: &str, key: &str) -> Result<(), WriteError> {
        self.conn
            .execute("DELETE FROM project_learned WHERE project = ?1 AND key = ?2", params![project, key])
            .map(|_| ())
            .map_err(fault("clearing what a project learned"))
            .map_err(WriteError::Database)
    }
}
