//! What each offered branch told the owner to look at, one row a (branch, tip), for the build he runs. **Kept
//! with the offer and outliving it** (`preview_offered`): the build he loaded still holds the branch after its
//! Job ended or its tip landed. `docs/concepts/fleet.md`, *Offering a branch for the preview*.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;
use crate::preview_offered::{steps_read, KeptStep};

/// One note, as kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptBuildNote {
    pub branch: String,
    pub tip: String,
    /// The Job that last offered this branch at this tip.
    pub session_id: String,
    pub about: String,
    pub look: Option<String>,
    pub steps: Vec<KeptStep>,
    pub offered_at: String,
    /// When the owner marked it looked at. Absent until he does.
    pub looked_at: Option<String>,
}

const COLUMNS: &str = "branch, tip, session_id, about, look, steps, offered_at, looked_at";

fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptBuildNote> {
    Ok(KeptBuildNote {
        branch: row.get(0)?,
        tip: row.get(1)?,
        session_id: row.get(2)?,
        about: row.get(3)?,
        look: row.get(4)?,
        steps: steps_read(&row.get::<_, String>(5)?),
        offered_at: row.get(6)?,
        looked_at: row.get(7)?,
    })
}

impl Store {
    /// The note of `branch` at `tip`.
    pub fn build_note_of(&self, branch: &str, tip: &str) -> Result<Option<KeptBuildNote>, WriteError> {
        self.conn
            .query_row(&format!("SELECT {COLUMNS} FROM build_notes WHERE branch = ?1 AND tip = ?2"), params![branch, tip], read)
            .optional()
            .map_err(fault("reading a note of the preview build"))
            .map_err(WriteError::Database)
    }

    /// Mark the note of `branch` at `tip` looked at, **once**: a second mark keeps the first time. Whether there
    /// is such a note.
    pub fn keep_build_note_looked_at(&mut self, branch: &str, tip: &str, at: &str) -> Result<bool, WriteError> {
        let doing = "marking a note of the preview build looked at";
        self.conn
            .execute("UPDATE build_notes SET looked_at = COALESCE(looked_at, ?3) WHERE branch = ?1 AND tip = ?2", params![branch, tip, at])
            .map(|touched| touched > 0)
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }
}
