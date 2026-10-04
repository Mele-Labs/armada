//! What a person said while walking a Job's served mock, kept on the Job.
//!
//! **Beside the Job, never in its worktree.** A Prototype's worktree is
//! throwaway and these are what the next Drone is told, so the row outlives the
//! checkout the way the Job's own record does. The frame is a path under the
//! machine directory, for `crate::showing`'s reason: the row is the cheap fact
//! and the bytes are read once by whoever opens them.
//!
//! **Written once, and one column moves after.** A note is never edited; it is
//! deleted while unsent, and `sent_at` is set once, when a `request_changes`
//! carries it to a Drone. A trigger refuses every other update, so the
//! promise is the file's and not this module's.

use core_model::{JobId, Timestamp};
use rusqlite::{OptionalExtension, Row};

use crate::error::{fault, LoadJobError, WriteError};
use crate::open::Store;

/// Version 103 — a Job's walk notes.
///
/// **`REFERENCES jobs(job_id)`**, so `forget_job`'s sweep takes them with the
/// Job and nothing registers them anywhere else. **Nothing to backfill**:
/// nothing could keep a walk note before this.
pub(crate) const V103: &str = r#"
CREATE TABLE job_walk_notes (
    note_id        TEXT PRIMARY KEY,
    job_id         TEXT NOT NULL REFERENCES jobs(job_id),
    said           TEXT NOT NULL CHECK (trim(said) <> ''),
    at             TEXT NOT NULL,
    element        TEXT NOT NULL,
    selector       TEXT NOT NULL,
    location       TEXT NOT NULL,
    served_run     TEXT,
    served_name    TEXT,
    served_address TEXT,
    frame          TEXT,
    sent_at        TEXT,
    CHECK ((served_run IS NULL) = (served_name IS NULL)
       AND (served_run IS NULL) = (served_address IS NULL))
) STRICT;

CREATE INDEX job_walk_notes_by_job ON job_walk_notes (job_id, at, note_id);

CREATE TRIGGER job_walk_notes_are_never_edited
BEFORE UPDATE ON job_walk_notes
WHEN OLD.sent_at IS NOT NULL
  OR NEW.sent_at IS NULL
  OR NEW.note_id IS NOT OLD.note_id
  OR NEW.job_id IS NOT OLD.job_id
  OR NEW.said IS NOT OLD.said
  OR NEW.at IS NOT OLD.at
  OR NEW.element IS NOT OLD.element
  OR NEW.selector IS NOT OLD.selector
  OR NEW.location IS NOT OLD.location
  OR NEW.served_run IS NOT OLD.served_run
  OR NEW.served_name IS NOT OLD.served_name
  OR NEW.served_address IS NOT OLD.served_address
  OR NEW.frame IS NOT OLD.frame
BEGIN
    SELECT RAISE(ABORT, 'a walk note is never edited, and is marked sent once');
END;
"#;

/// The server a walk note was captured on: its run, its name, its origin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalkServed {
    pub run: String,
    pub name: String,
    pub address: String,
}

/// One walk note, as the store keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptWalkNote {
    pub note_id: String,
    pub said: String,
    pub at: Timestamp,
    /// The one line naming what was pointed at, composed by the caller.
    pub element: String,
    pub selector: String,
    pub location: String,
    pub served: Option<WalkServed>,
    /// The kept frame's absolute path, where one was kept.
    pub frame: Option<String>,
    pub sent: bool,
}

/// What a removal came to. **Two refusals the caller words**, because only it
/// knows which code each is on the wire.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WalkNoteRemoved {
    /// Gone, and this was its frame, for the caller to delete.
    Removed { frame: Option<String> },
    /// No note by that id on this Job.
    NoSuchNote,
    /// A Drone was already handed it, so it is part of what the work was
    /// done against and stays.
    AlreadySent,
}

impl Store {
    /// Keep one walk note on `job_id`.
    pub fn keep_walk_note(
        &mut self,
        job_id: &JobId,
        note: &KeptWalkNote,
    ) -> Result<(), WriteError> {
        let served = note.served.as_ref();
        self.conn
            .execute(
                "INSERT INTO job_walk_notes (note_id, job_id, said, at, element, selector, \
                 location, served_run, served_name, served_address, frame, sent_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL)",
                rusqlite::params![
                    note.note_id,
                    job_id.as_str(),
                    note.said,
                    note.at.as_str(),
                    note.element,
                    note.selector,
                    note.location,
                    served.map(|s| s.run.as_str()),
                    served.map(|s| s.name.as_str()),
                    served.map(|s| s.address.as_str()),
                    note.frame,
                ],
            )
            .map(|_| ())
            .map_err(fault("keeping a walk note"))
            .map_err(WriteError::Database)
    }

    /// Every walk note on a Job, oldest first, sent ones included.
    pub fn walk_notes(&self, job_id: &JobId) -> Result<Vec<KeptWalkNote>, LoadJobError> {
        let unreadable =
            |why: rusqlite::Error| LoadJobError::Database(fault("reading walk notes")(why));
        let mut statement = self
            .conn
            .prepare(
                "SELECT note_id, said, at, element, selector, location, served_run, \
                 served_name, served_address, frame, sent_at
                 FROM job_walk_notes WHERE job_id = ?1 ORDER BY at, note_id",
            )
            .map_err(unreadable)?;
        let rows = statement
            .query_map((job_id.as_str(),), walk_note)
            .map_err(unreadable)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(unreadable)
    }

    /// Delete one unsent walk note, answering with its frame so the caller can
    /// delete the file too. A sent note and an unknown id are answered, not
    /// faulted.
    pub fn remove_walk_note(
        &mut self,
        job_id: &JobId,
        note_id: &str,
    ) -> Result<WalkNoteRemoved, WriteError> {
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting a walk note's removal"))
            .map_err(WriteError::Database)?;
        let found: Option<(Option<String>, Option<String>)> = tx
            .query_row(
                "SELECT frame, sent_at FROM job_walk_notes WHERE job_id = ?1 AND note_id = ?2",
                (job_id.as_str(), note_id),
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(fault("reading the walk note to remove"))
            .map_err(WriteError::Database)?;
        let removed = match found {
            None => WalkNoteRemoved::NoSuchNote,
            Some((_, Some(_))) => WalkNoteRemoved::AlreadySent,
            Some((frame, None)) => {
                tx.execute(
                    "DELETE FROM job_walk_notes WHERE job_id = ?1 AND note_id = ?2",
                    (job_id.as_str(), note_id),
                )
                .map_err(fault("removing a walk note"))
                .map_err(WriteError::Database)?;
                WalkNoteRemoved::Removed { frame }
            }
        };
        tx.commit()
            .map_err(fault("committing a walk note's removal"))
            .map_err(WriteError::Database)?;
        Ok(removed)
    }

    /// Mark the named notes sent, in one transaction. **A note already sent is
    /// left as it was**, so this never moves `sent_at` twice.
    pub fn mark_walk_notes_sent(
        &mut self,
        job_id: &JobId,
        note_ids: &[String],
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting to mark walk notes sent"))
            .map_err(WriteError::Database)?;
        for note_id in note_ids {
            tx.execute(
                "UPDATE job_walk_notes SET sent_at = ?3
                 WHERE job_id = ?1 AND note_id = ?2 AND sent_at IS NULL",
                (job_id.as_str(), note_id.as_str(), at.as_str()),
            )
            .map_err(fault("marking a walk note sent"))
            .map_err(WriteError::Database)?;
        }
        tx.commit()
            .map_err(fault("committing walk notes marked sent"))
            .map_err(WriteError::Database)
    }
}

fn walk_note(row: &Row<'_>) -> rusqlite::Result<KeptWalkNote> {
    let run: Option<String> = row.get(6)?;
    let name: Option<String> = row.get(7)?;
    let address: Option<String> = row.get(8)?;
    let sent_at: Option<String> = row.get(10)?;
    Ok(KeptWalkNote {
        note_id: row.get(0)?,
        said: row.get(1)?,
        at: Timestamp::from_rfc3339(row.get::<_, String>(2)?),
        element: row.get(3)?,
        selector: row.get(4)?,
        location: row.get(5)?,
        served: match (run, name, address) {
            (Some(run), Some(name), Some(address)) => Some(WalkServed { run, name, address }),
            _ => None,
        },
        frame: row.get(9)?,
        sent: sent_at.is_some(),
    })
}
