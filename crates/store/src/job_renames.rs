//! The Jobs a person renamed in Bridge, which the proposer reads to name later Jobs his way.
//! `docs/concepts/job-proposer.md`, *Names he gave*.
//!
//! **One row a Job, and `was` is Armada's.** A second rename of one Job replaces his title and
//! leaves the name Armada gave it, so what the proposer reads is always its name against his.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One Job's rename, as last made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptRename {
    pub session_id: String,
    /// The Job's Manifest id.
    pub repository: String,
    /// The Job's first message from the person.
    pub request: String,
    /// The title before his first rename.
    pub was: String,
    /// His latest title.
    pub title: String,
    pub renamed_at: String,
}

impl Store {
    /// Keep `rename`. A Job renamed before keeps its first `was`, `request` and `repository`.
    pub fn keep_rename(&mut self, rename: &KeptRename) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO job_renames (session_id, repository, request, was, title, renamed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT (session_id) DO UPDATE SET title = excluded.title, renamed_at = excluded.renamed_at",
                params![rename.session_id, rename.repository, rename.request, rename.was, rename.title, rename.renamed_at],
            )
            .map(|_| ())
            .map_err(fault("keeping a Job's rename"))
            .map_err(WriteError::Database)
    }

    /// Up to `most` renames, `repository`'s first, each part newest first.
    pub fn renames_for(&self, repository: &str, most: usize) -> Result<Vec<KeptRename>, WriteError> {
        let doing = "reading the Jobs a person renamed";
        let mut statement = self
            .conn
            .prepare(
                "SELECT session_id, repository, request, was, title, renamed_at FROM job_renames
                 ORDER BY repository = ?1 DESC, renamed_at DESC LIMIT ?2",
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map(params![repository, most as i64], |row| {
                Ok(KeptRename {
                    session_id: row.get(0)?,
                    repository: row.get(1)?,
                    request: row.get(2)?,
                    was: row.get(3)?,
                    title: row.get(4)?,
                    renamed_at: row.get(5)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        Ok(rows)
    }
}
