//! The questions Fleet put to a person about where a taught pattern opens in a repository.
//!
//! **One row a mod and repository, kept for good**, so a pattern is asked about once per
//! repository however many Sessions teach or read it, and a question put aside is not put again.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One question, as it was put.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptWordAsk {
    /// The Words mod, by name.
    pub mod_name: String,
    /// The repository's Manifest id.
    pub repository: String,
    pub session_id: String,
    pub asked_at: String,
}

impl Store {
    /// Put the question for `mod_name` in `repository` to `session_id`'s person. **Whether it was
    /// new**: a question already put, answered or not, is not put a second time.
    pub fn ask_word_address(&mut self, mod_name: &str, repository: &str, session_id: &str, at: &str) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO word_asks (mod_name, repository, session_id, asked_at) VALUES (?1, ?2, ?3, ?4)",
                params![mod_name, repository, session_id, at],
            )
            .map(|kept| kept > 0)
            .map_err(fault("keeping a question about where words open"))
            .map_err(WriteError::Database)
    }

    /// The questions put to `session_id`, oldest first.
    pub fn word_asks_of(&self, session_id: &str) -> Result<Vec<KeptWordAsk>, WriteError> {
        let doing = "reading the questions about where words open";
        let mut statement = self
            .conn
            .prepare(
                "SELECT mod_name, repository, session_id, asked_at FROM word_asks
                 WHERE session_id = ?1 ORDER BY asked_at, mod_name, repository",
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([session_id], |row| {
                Ok(KeptWordAsk { mod_name: row.get(0)?, repository: row.get(1)?, session_id: row.get(2)?, asked_at: row.get(3)? })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        Ok(rows)
    }

    /// The question for `mod_name` in `repository`, if one was put.
    pub fn word_ask(&self, mod_name: &str, repository: &str) -> Result<Option<KeptWordAsk>, WriteError> {
        let doing = "reading a question about where words open";
        let mut statement = self
            .conn
            .prepare("SELECT mod_name, repository, session_id, asked_at FROM word_asks WHERE mod_name = ?1 AND repository = ?2")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let mut rows = statement
            .query_map(params![mod_name, repository], |row| {
                Ok(KeptWordAsk { mod_name: row.get(0)?, repository: row.get(1)?, session_id: row.get(2)?, asked_at: row.get(3)? })
            })
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.next().transpose().map_err(fault(doing)).map_err(WriteError::Database)
    }
}
