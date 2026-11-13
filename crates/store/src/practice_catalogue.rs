//! The practices read from a repository's docs for one kind of change in one area.
//! `docs/concepts/pull-review.md`, *Change sets*.
//!
//! **`items` and `sources` are opaque text here**, the JSON `fleet` validated, as a review run's
//! result is: their shape is the wire's.

use rusqlite::OptionalExtension;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One row of `practice_catalogue`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptPractices {
    pub manifest_id: String,
    /// The directory a change set's hunks share, `""` for the root.
    pub area: String,
    pub kind: String,
    pub items: String,
    pub sources: String,
    pub read_at: String,
}

impl Store {
    /// Keep what was read for an area and kind, replacing an earlier read.
    pub fn keep_practices(&mut self, kept: &KeptPractices) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO practice_catalogue (manifest_id, area, kind, items, sources, read_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![kept.manifest_id, kept.area, kept.kind, kept.items, kept.sources, kept.read_at],
            )
            .map(|_| ())
            .map_err(fault("keeping practices"))
            .map_err(WriteError::Database)
    }

    /// What was read for an area and kind, if anything was.
    pub fn practices(&self, manifest_id: &str, area: &str, kind: &str) -> Result<Option<KeptPractices>, WriteError> {
        self.conn
            .query_row(
                "SELECT manifest_id, area, kind, items, sources, read_at FROM practice_catalogue \
                 WHERE manifest_id = ?1 AND area = ?2 AND kind = ?3",
                rusqlite::params![manifest_id, area, kind],
                |row| {
                    Ok(KeptPractices {
                        manifest_id: row.get(0)?,
                        area: row.get(1)?,
                        kind: row.get(2)?,
                        items: row.get(3)?,
                        sources: row.get(4)?,
                        read_at: row.get(5)?,
                    })
                },
            )
            .optional()
            .map_err(fault("reading practices"))
            .map_err(WriteError::Database)
    }
}
