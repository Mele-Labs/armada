//! The revisions of a Job's artifacts the person wrote with his Edit. `docs/concepts/artifact.md`, *His Edit*.
//!
//! **Apart from the artifact's attachment**, which a new revision rewrites whole. A revision not named here is
//! the agent's.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One revision he wrote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptOwnerRevision {
    /// The artifact's id: a page's path or `draft:<title>`.
    pub artifact: String,
    /// From 1, as the artifact's revisions are numbered.
    pub revision: u32,
    pub at: String,
}

impl Store {
    /// Every revision of `session`'s artifacts he wrote.
    pub fn owner_revisions(&self, session: &str) -> Result<Vec<KeptOwnerRevision>, WriteError> {
        let doing = "reading the revisions he wrote";
        let mut statement = self
            .conn
            .prepare("SELECT artifact, revision, at FROM artifact_owner_revisions WHERE session_id = ?1")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([session], |row| Ok(KeptOwnerRevision { artifact: row.get(0)?, revision: row.get(1)?, at: row.get(2)? }))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// Keep that he wrote one revision.
    pub fn keep_owner_revision(&mut self, session: &str, kept: &KeptOwnerRevision) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO artifact_owner_revisions (session_id, artifact, revision, at) VALUES (?1, ?2, ?3, ?4)",
                params![session, kept.artifact, kept.revision, kept.at],
            )
            .map(drop)
            .map_err(fault("keeping a revision he wrote"))
            .map_err(WriteError::Database)
    }

    /// Job `new` took the place of `old`: the revisions he wrote of `artifact` are `new`'s too.
    pub fn carry_owner_revisions(&mut self, old: &str, new: &str, artifact: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO artifact_owner_revisions (session_id, artifact, revision, at)
                 SELECT ?2, artifact, revision, at FROM artifact_owner_revisions WHERE session_id = ?1 AND artifact = ?3",
                params![old, new, artifact],
            )
            .map(drop)
            .map_err(fault("carrying the revisions he wrote"))
            .map_err(WriteError::Database)
    }
}
