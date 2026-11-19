//! The Reviews lists last read from the forge, one row a project. `docs/concepts/pull-review.md`.
//!
//! **The lists are opaque text here**: the JSON of the wire's `PullReviewLists`, because the shape of an
//! answer is the wire's and not this crate's. A row an older build wrote in another shape is `fleet`'s to
//! find unreadable and treat as nothing kept.

use rusqlite::OptionalExtension;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One row of `review_lists_kept`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptReviewLists {
    /// The project's id; the empty string is every project.
    pub project: String,
    /// RFC 3339, when the forge was read.
    pub fetched_at: String,
    pub lists: String,
}

impl Store {
    /// Keep the lists of a project, replacing the ones before.
    pub fn keep_review_lists(&mut self, kept: &KeptReviewLists) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO review_lists_kept (project, fetched_at, lists) VALUES (?1, ?2, ?3)",
                rusqlite::params![kept.project, kept.fetched_at, kept.lists],
            )
            .map(|_| ())
            .map_err(fault("keeping the review lists"))
            .map_err(WriteError::Database)
    }

    /// The lists last kept for a project, if any.
    pub fn kept_review_lists(&self, project: &str) -> Result<Option<KeptReviewLists>, WriteError> {
        self.conn
            .query_row(
                "SELECT project, fetched_at, lists FROM review_lists_kept WHERE project = ?1",
                [project],
                |row| Ok(KeptReviewLists { project: row.get(0)?, fetched_at: row.get(1)?, lists: row.get(2)? }),
            )
            .optional()
            .map_err(fault("reading the kept review lists"))
            .map_err(WriteError::Database)
    }
}
