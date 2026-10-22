//! The follow-ups a Session's agent keeps for itself, from the close of its replies.
//!
//! **One row a sentence.** The same sentence said again is not kept twice, and a settled one is not
//! kept again by a reply read a second time. A row is open until it is settled, and a settled row
//! stays: it is what holds the sentence off.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// A follow-up that is still open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptLater {
    /// `later-<n>`, which the Session's agent names to settle it.
    pub id: String,
    pub sentence: String,
    /// When the reply that said it ended. RFC3339.
    pub said_at: String,
}

fn id_of(n: i64) -> String {
    format!("later-{n}")
}

impl Store {
    /// Keep `sentence`, said when a reply ended at `said_at`. Whether it was new.
    pub fn keep_later(&mut self, session_id: &str, sentence: &str, said_at: &str) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO session_later (session_id, n, sentence, said_at)
                 VALUES (?1, (SELECT COALESCE(MAX(n), 0) + 1 FROM session_later WHERE session_id = ?1), ?2, ?3)",
                params![session_id, sentence, said_at],
            )
            .map(|kept| kept > 0)
            .map_err(fault("keeping a follow-up a job keeps for itself"))
            .map_err(WriteError::Database)
    }

    /// The open follow-ups of `session_id`, oldest first.
    pub fn later_of(&self, session_id: &str) -> Result<Vec<KeptLater>, WriteError> {
        let mut statement = self
            .conn
            .prepare("SELECT n, sentence, said_at FROM session_later WHERE session_id = ?1 AND how IS NULL ORDER BY n")
            .map_err(fault("reading the follow-ups a job keeps for itself"))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([session_id], |row| Ok(KeptLater { id: id_of(row.get(0)?), sentence: row.get(1)?, said_at: row.get(2)? }))
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault("reading the follow-ups a job keeps for itself"))
            .map_err(WriteError::Database)?;
        Ok(rows)
    }

    /// Settle the open follow-up `id` as `how` (`done`, `filed` or `dropped`) with `note`: the issue
    /// it was filed as, or the reason it was dropped. The sentence it was, or `None` where `id` names
    /// none open.
    pub fn settle_later(&mut self, session_id: &str, id: &str, how: &str, note: Option<&str>, at: &str) -> Result<Option<String>, WriteError> {
        let Some(n) = id.strip_prefix("later-").and_then(|n| n.parse::<i64>().ok()) else {
            return Ok(None);
        };
        self.conn
            .query_row(
                "UPDATE session_later SET how = ?3, note = ?4, settled_at = ?5
                 WHERE session_id = ?1 AND n = ?2 AND how IS NULL RETURNING sentence",
                params![session_id, n, how, note, at],
                |row| row.get(0),
            )
            .optional()
            .map_err(fault("settling a follow-up a job keeps for itself"))
            .map_err(WriteError::Database)
    }
}
