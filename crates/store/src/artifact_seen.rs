//! The newest revision he opened of each artifact a Job made. `docs/concepts/artifact.md`, *Seen and anchored notes*.
//!
//! **Only forward**: a mark of an older revision leaves the row as it is.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One artifact's newest opened revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptArtifactSeen {
    /// The artifact's id: `sketch:<title>`, a page's path or `draft:<title>`.
    pub artifact: String,
    pub revision: u32,
}

impl Store {
    /// Every artifact of `session` he opened, with the newest revision of each.
    pub fn artifact_seen(&self, session: &str) -> Result<Vec<KeptArtifactSeen>, WriteError> {
        let doing = "reading the revisions he opened of a Job's artifacts";
        let mut statement = self
            .conn
            .prepare("SELECT artifact, revision FROM artifact_seen WHERE session_id = ?1")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([session], |row| Ok(KeptArtifactSeen { artifact: row.get(0)?, revision: row.get(1)? }))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// Keep that he opened `revision`, where it is newer than what is kept. Answers with the newest kept and whether
    /// this mark moved it.
    pub fn keep_artifact_seen(&mut self, session: &str, artifact: &str, revision: u32) -> Result<(u32, bool), WriteError> {
        let doing = "keeping the revision he opened of a Job's artifact";
        let kept = (|| {
            let transaction = self.conn.transaction()?;
            let moved = transaction.execute(
                "INSERT INTO artifact_seen (session_id, artifact, revision) VALUES (?1, ?2, ?3)
                 ON CONFLICT (session_id, artifact) DO UPDATE SET revision = excluded.revision WHERE excluded.revision > artifact_seen.revision",
                params![session, artifact, revision],
            )? > 0;
            let seen = transaction.query_row(
                "SELECT revision FROM artifact_seen WHERE session_id = ?1 AND artifact = ?2",
                params![session, artifact],
                |row| row.get(0),
            )?;
            transaction.commit()?;
            Ok((seen, moved))
        })();
        kept.map_err(fault(doing)).map_err(WriteError::Database)
    }
}
