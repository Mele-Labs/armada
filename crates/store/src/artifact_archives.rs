//! A Job's artifacts the agent or the person archived as stale. `docs/concepts/artifact.md`, *Archive*.
//!
//! **Apart from the artifact's attachment**, which a new revision rewrites whole, so only a restore lifts
//! the archive. **The dismissal goes with it in one write**: an archive that took an item off his list
//! keeps the item's id, and its restore lifts that dismissal alone.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One archived artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptArchive {
    /// The artifact's id: `sketch:<title>`, a page's path or `draft:<title>`.
    pub artifact: String,
    /// `agent` or `owner`.
    pub by: String,
    pub at: String,
    /// The Needs you id the archive dismissed, where it waited on him.
    pub item: Option<String>,
}

impl Store {
    /// Every artifact of `session` archived now.
    pub fn artifact_archives(&self, session: &str) -> Result<Vec<KeptArchive>, WriteError> {
        let doing = "reading a Job's archived artifacts";
        let mut statement = self
            .conn
            .prepare("SELECT artifact, archived_by, archived_at, item FROM artifact_archives WHERE session_id = ?1")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([session], |row| Ok(KeptArchive { artifact: row.get(0)?, by: row.get(1)?, at: row.get(2)?, item: row.get(3)? }))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// Archive one artifact, dismissing `archive.item` with it. `false` where it was archived already, and
    /// nothing is written.
    pub fn archive_artifact(&mut self, session: &str, archive: &KeptArchive) -> Result<bool, WriteError> {
        let doing = "archiving a Job's artifact";
        let kept = (|| {
            let transaction = self.conn.transaction()?;
            let added = transaction.execute(
                "INSERT INTO artifact_archives (session_id, artifact, archived_by, archived_at, item) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT DO NOTHING",
                params![session, archive.artifact, archive.by, archive.at, archive.item],
            )?;
            if added == 0 {
                return Ok(false);
            }
            if let Some(item) = &archive.item {
                transaction.execute(
                    "INSERT OR IGNORE INTO session_waiting_dismissed (session_id, item_id) VALUES (?1, ?2)",
                    params![session, item],
                )?;
                transaction.execute("DELETE FROM session_waiting WHERE session_id = ?1 AND item_id = ?2", params![session, item])?;
            }
            transaction.commit()?;
            Ok(true)
        })();
        kept.map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// Lift one artifact's archive, and the dismissal it wrote. `None` where it was not archived.
    pub fn restore_artifact(&mut self, session: &str, artifact: &str) -> Result<Option<KeptArchive>, WriteError> {
        let doing = "restoring a Job's archived artifact";
        let kept = (|| {
            let transaction = self.conn.transaction()?;
            let lifted = transaction
                .query_row(
                    "DELETE FROM artifact_archives WHERE session_id = ?1 AND artifact = ?2 RETURNING artifact, archived_by, archived_at, item",
                    params![session, artifact],
                    |row| Ok(KeptArchive { artifact: row.get(0)?, by: row.get(1)?, at: row.get(2)?, item: row.get(3)? }),
                )
                .optional()?;
            if let Some(item) = lifted.as_ref().and_then(|one| one.item.as_ref()) {
                transaction.execute(
                    "DELETE FROM session_waiting_dismissed WHERE session_id = ?1 AND item_id = ?2",
                    params![session, item],
                )?;
            }
            transaction.commit()?;
            Ok(lifted)
        })();
        kept.map_err(fault(doing)).map_err(WriteError::Database)
    }
}
