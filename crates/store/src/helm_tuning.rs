//! The model and effort a person chose for a Helm conversation.
//!
//! **Not `helm_sessions`' row**: starting fresh forgets the session, and what
//! a person chose to run on is not part of what they started over.

use rusqlite::OptionalExtension;

use crate::error::{fault, DatabaseFault, WriteError};
use crate::open::Store;

impl Store {
    /// `(model, effort)` as chosen, each `None` for the machine's own.
    pub fn helm_tuning(
        &self,
        conversation: &str,
    ) -> Result<(Option<String>, Option<String>), DatabaseFault> {
        self.conn
            .query_row(
                "SELECT model, effort FROM helm_tuning WHERE conversation = ?1",
                [conversation],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map(Option::unwrap_or_default)
            .map_err(fault("reading a Helm conversation's model and effort"))
    }

    /// Keep the choice; two `None`s remove the row.
    pub fn keep_helm_tuning(
        &mut self,
        conversation: &str,
        model: Option<&str>,
        effort: Option<&str>,
    ) -> Result<(), WriteError> {
        let written = if model.is_none() && effort.is_none() {
            self.conn.execute(
                "DELETE FROM helm_tuning WHERE conversation = ?1",
                [conversation],
            )
        } else {
            self.conn.execute(
                "INSERT INTO helm_tuning (conversation, model, effort) VALUES (?1, ?2, ?3)
                 ON CONFLICT (conversation)
                 DO UPDATE SET model = excluded.model, effort = excluded.effort",
                (conversation, model, effort),
            )
        };
        written
            .map(|_| ())
            .map_err(fault("keeping a Helm conversation's model and effort"))
            .map_err(WriteError::Database)
    }
}
