//! The casts of the votes a Job's pages declare. `docs/concepts/artifact.md`, *Page votes*.
//!
//! **Every cast is kept**, apart from the page's attachment, which a new revision rewrites whole. The newest per
//! vote is the one Fleet serves.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One cast.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptVote {
    /// The page's artifact id: its path.
    pub artifact: String,
    pub revision: u32,
    /// The vote's id.
    pub vote: String,
    /// `pick`, `rank` or `score`.
    pub form: String,
    /// The cast as the wire carries it, JSON.
    pub result: String,
    pub at: String,
}

impl Store {
    /// Every cast on `session`'s pages, oldest first.
    pub fn page_votes(&self, session: &str) -> Result<Vec<KeptVote>, WriteError> {
        let doing = "reading the votes on a job's pages";
        let mut statement = self
            .conn
            .prepare("SELECT artifact, revision, vote, form, result, at FROM page_votes WHERE session_id = ?1 ORDER BY rowid")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([session], |row| {
                Ok(KeptVote { artifact: row.get(0)?, revision: row.get(1)?, vote: row.get(2)?, form: row.get(3)?, result: row.get(4)?, at: row.get(5)? })
            })
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// Keep one cast.
    pub fn keep_page_vote(&mut self, session: &str, kept: &KeptVote) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO page_votes (session_id, artifact, revision, vote, form, result, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![session, kept.artifact, kept.revision, kept.vote, kept.form, kept.result, kept.at],
            )
            .map(drop)
            .map_err(fault("keeping a vote on a job's page"))
            .map_err(WriteError::Database)
    }
}
