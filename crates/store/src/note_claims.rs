//! A note on a screen sent to every live Job on its features, and the Job that took it.
//! `docs/concepts/feature-catalog.md`, *A note on a screen*.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One note sent to several Jobs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptNoteClaim {
    pub note_id: String,
    pub manifest_id: String,
    /// The files and folders that draw the screen it was left on.
    pub paths: Vec<String>,
    /// The Jobs it was sent to, in that order.
    pub offered: Vec<String>,
    pub sent_at: String,
    /// The Job that holds it, empty until one claims it or Fleet gives it one.
    pub claimed_by: String,
    pub claimed_at: String,
    pub given: bool,
}

const COLUMNS: &str = "note_id, manifest_id, paths, offered, sent_at, claimed_by, claimed_at, given";

fn row(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptNoteClaim> {
    let list = |raw: String| serde_json::from_str::<Vec<String>>(&raw).unwrap_or_default();
    Ok(KeptNoteClaim {
        note_id: row.get(0)?,
        manifest_id: row.get(1)?,
        paths: list(row.get(2)?),
        offered: list(row.get(3)?),
        sent_at: row.get(4)?,
        claimed_by: row.get(5)?,
        claimed_at: row.get(6)?,
        given: row.get::<_, i64>(7)? != 0,
    })
}

impl Store {
    /// Keep a note as sent to its Jobs, unclaimed. A note sent again starts over.
    pub fn keep_note_offered(&mut self, note: &KeptNoteClaim) -> Result<(), WriteError> {
        let json = |list: &[String]| serde_json::to_string(list).unwrap_or_else(|_| String::from("[]"));
        self.conn
            .execute(
                &format!("INSERT OR REPLACE INTO note_claims ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"),
                params![note.note_id, note.manifest_id, json(&note.paths), json(&note.offered), note.sent_at, note.claimed_by, note.claimed_at, i64::from(note.given)],
            )
            .map(drop)
            .map_err(fault("keeping a note sent to a feature's Jobs"))
            .map_err(WriteError::Database)
    }

    /// The note `note_id`, where it was sent to several Jobs.
    pub fn note_claim(&self, note_id: &str) -> Result<Option<KeptNoteClaim>, WriteError> {
        self.conn
            .query_row(&format!("SELECT {COLUMNS} FROM note_claims WHERE note_id = ?1"), [note_id], row)
            .optional()
            .map_err(fault("reading a note sent to a feature's Jobs"))
            .map_err(WriteError::Database)
    }

    /// The notes no Job holds, oldest first.
    pub fn notes_unclaimed(&self) -> Result<Vec<KeptNoteClaim>, WriteError> {
        let doing = "reading the notes no Job claimed";
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM note_claims WHERE claimed_by = '' ORDER BY sent_at, note_id"))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement.query_map([], row).map_err(fault(doing)).map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// `job` takes the note, if no Job holds it yet. Whether it took it.
    pub fn take_note(&mut self, note_id: &str, job: &str, at: &str, given: bool) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE note_claims SET claimed_by = ?2, claimed_at = ?3, given = ?4 WHERE note_id = ?1 AND claimed_by = ''",
                params![note_id, job, at, i64::from(given)],
            )
            .map(|changed| changed == 1)
            .map_err(fault("taking a note for a Job"))
            .map_err(WriteError::Database)
    }

    /// Forget the note: none of the Jobs it was sent to is live to be given it.
    pub fn drop_note_claim(&mut self, note_id: &str) -> Result<(), WriteError> {
        self.conn
            .execute("DELETE FROM note_claims WHERE note_id = ?1", [note_id])
            .map(drop)
            .map_err(fault("forgetting a note no Job is live to take"))
            .map_err(WriteError::Database)
    }
}
