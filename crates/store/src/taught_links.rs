//! The link rules a person taught: words that match a pattern become a link, in every Session.
//!
//! **Nothing here checks a rule.** Whether the pattern compiles and the address is a web address
//! is Fleet's reading of a teaching; this keeps what it was handed and hands it back.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One taught link rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaughtLink {
    /// `None` until it is kept.
    pub id: Option<i64>,
    pub pattern: String,
    /// `{match}` stands for the matched words.
    pub address: String,
    /// The words it was taught from.
    pub example: String,
    /// The repository's key, or `None` for every repository.
    pub repository: Option<String>,
    /// Milliseconds since the epoch.
    pub made_at: i64,
}

impl Store {
    /// Keep a rule. Its id.
    pub fn save_taught_link(&mut self, link: &TaughtLink) -> Result<i64, WriteError> {
        self.conn
            .query_row(
                "INSERT INTO taught_links (pattern, address, example, repository, made_at)
                 VALUES (?1, ?2, ?3, ?4, ?5) RETURNING id",
                params![link.pattern, link.address, link.example, link.repository, link.made_at],
                |row| row.get(0),
            )
            .map_err(fault("keeping a link rule a person taught"))
            .map_err(WriteError::Database)
    }

    /// The rules for `repository` and for every repository, oldest first.
    pub fn taught_links_for(&self, repository: &str) -> Result<Vec<TaughtLink>, WriteError> {
        let mut statement = self
            .conn
            .prepare(
                "SELECT id, pattern, address, example, repository, made_at FROM taught_links
                 WHERE repository IS NULL OR repository = ?1 ORDER BY made_at, id",
            )
            .map_err(fault("reading the link rules a person taught"))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([repository], |row| {
                Ok(TaughtLink {
                    id: Some(row.get(0)?),
                    pattern: row.get(1)?,
                    address: row.get(2)?,
                    example: row.get(3)?,
                    repository: row.get(4)?,
                    made_at: row.get(5)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault("reading the link rules a person taught"))
            .map_err(WriteError::Database)?;
        Ok(rows)
    }

    /// Forget a rule. Whether there was one.
    pub fn forget_taught_link(&mut self, id: i64) -> Result<bool, WriteError> {
        self.conn
            .execute("DELETE FROM taught_links WHERE id = ?1", [id])
            .map(|gone| gone > 0)
            .map_err(fault("forgetting a link rule a person taught"))
            .map_err(WriteError::Database)
    }
}
