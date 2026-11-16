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
    /// What he made of it, `good` or `needs_work`, as the wire spells it. Absent where he has not said.
    pub verdict: Option<String>,
    /// What its Ready to try card shows beside the note.
    pub card: KeptCard,
    /// When a card he asked to be reminded of comes back.
    pub later_until: Option<String>,
    /// When a reminder last brought the card back.
    pub back_at: Option<String>,
    /// When the Job took the offer back.
    pub withdrawn_at: Option<String>,
}

/// What an offer gave its Ready to try card, beyond the note.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptCard {
    pub approved: Option<KeptApproved>,
    /// Paths, as Fleet resolved them.
    pub pictures: Vec<String>,
    pub words: Option<String>,
}

/// The artifact he approved, as it read when the Job offered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptApproved {
    pub artifact: String,
    pub title: String,
    /// `sketch`, `draft` or `page`, as the wire spells it.
    pub form: String,
    pub revision: u32,
    pub approved_at: Option<String>,
}

const COLUMNS: &str = "branch, tip, session_id, about, look, steps, offered_at, looked_at, verdict, \
    approved_artifact, approved_title, approved_form, approved_revision, approved_at, pictures, words, later_until, back_at, withdrawn_at";

fn pictures_read(text: &str) -> Vec<String> {
    serde_json::from_str(text).unwrap_or_default()
}

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
        verdict: row.get(8)?,
        card: KeptCard {
            approved: match (row.get::<_, Option<String>>(9)?, row.get::<_, Option<String>>(10)?, row.get::<_, Option<String>>(11)?, row.get::<_, Option<u32>>(12)?) {
                (Some(artifact), Some(title), Some(form), Some(revision)) => Some(KeptApproved { artifact, title, form, revision, approved_at: row.get(13)? }),
                _ => None,
            },
            pictures: pictures_read(&row.get::<_, String>(14)?),
            words: row.get(15)?,
        },
        later_until: row.get(16)?,
        back_at: row.get(17)?,
        withdrawn_at: row.get(18)?,
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

    /// Every note a Job last offered, oldest offer first: its Ready to try cards.
    pub fn build_notes_of_session(&self, session_id: &str) -> Result<Vec<KeptBuildNote>, WriteError> {
        let doing = "reading the notes a job offered";
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM build_notes WHERE session_id = ?1 ORDER BY offered_at, branch, tip"))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        statement
            .query_map(params![session_id], read)
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// The newest note the Job offered of `branch`, at any tip: what a re-offer that leaves a card's parts out keeps.
    pub fn build_note_newest(&self, session_id: &str, branch: &str) -> Result<Option<KeptBuildNote>, WriteError> {
        self.conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM build_notes WHERE session_id = ?1 AND branch = ?2 ORDER BY offered_at DESC, tip LIMIT 1"),
                params![session_id, branch],
                read,
            )
            .optional()
            .map_err(fault("reading the newest note of a branch"))
            .map_err(WriteError::Database)
    }

    /// Keep what the offer of `branch` at `tip` gave its card, **and lift a withdrawal**: an offer made again stands.
    /// Called after [`Store::keep_preview_offered`], which made the note. Whether there is such a note.
    pub fn keep_build_note_card(&mut self, branch: &str, tip: &str, card: &KeptCard) -> Result<bool, WriteError> {
        let approved = card.approved.as_ref();
        let pictures = serde_json::to_string(&card.pictures).unwrap_or_else(|_| String::from("[]"));
        self.conn
            .execute(
                "UPDATE build_notes SET approved_artifact = ?3, approved_title = ?4, approved_form = ?5, approved_revision = ?6, approved_at = ?7, \
                 pictures = ?8, words = ?9, withdrawn_at = NULL WHERE branch = ?1 AND tip = ?2",
                params![
                    branch,
                    tip,
                    approved.map(|one| &one.artifact),
                    approved.map(|one| &one.title),
                    approved.map(|one| &one.form),
                    approved.map(|one| one.revision),
                    approved.and_then(|one| one.approved_at.as_ref()),
                    pictures,
                    card.words,
                ],
            )
            .map(|touched| touched > 0)
            .map_err(fault("keeping what an offer gave its card"))
            .map_err(WriteError::Database)
    }

    /// Keep when the card of `branch` at `tip` comes back. Whether there is such a note.
    pub fn keep_build_note_later(&mut self, branch: &str, tip: &str, until: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("UPDATE build_notes SET later_until = ?3 WHERE branch = ?1 AND tip = ?2", params![branch, tip, until])
            .map(|touched| touched > 0)
            .map_err(fault("keeping when a card comes back"))
            .map_err(WriteError::Database)
    }

    /// Bring back every card whose time came by `now`: `back_at` is its time, and it is no longer later. The
    /// (session, branch, tip) of each, so its thread is told.
    pub fn build_notes_come_back(&mut self, now: &str) -> Result<Vec<(String, String, String)>, WriteError> {
        let doing = "bringing back the cards whose time came";
        let transaction = self.conn.transaction().map_err(fault(doing)).map_err(WriteError::Database)?;
        let due = {
            let mut statement = transaction
                .prepare("SELECT session_id, branch, tip FROM build_notes WHERE later_until IS NOT NULL AND later_until <= ?1")
                .map_err(fault(doing))
                .map_err(WriteError::Database)?;
            statement
                .query_map(params![now], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                .and_then(|rows| rows.collect::<Result<Vec<(String, String, String)>, _>>())
                .map_err(fault(doing))
                .map_err(WriteError::Database)?
        };
        if due.is_empty() {
            return Ok(due);
        }
        transaction
            .execute("UPDATE build_notes SET back_at = later_until, later_until = NULL WHERE later_until IS NOT NULL AND later_until <= ?1", params![now])
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        transaction.commit().map_err(fault(doing)).map_err(WriteError::Database)?;
        Ok(due)
    }

    /// Keep the note of `branch` at `tip` withdrawn: the Job took the offer back. Whether there is such a note.
    pub fn keep_build_note_withdrawn(&mut self, branch: &str, tip: &str, at: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("UPDATE build_notes SET withdrawn_at = ?3 WHERE branch = ?1 AND tip = ?2", params![branch, tip, at])
            .map(|touched| touched > 0)
            .map_err(fault("keeping a note withdrawn"))
            .map_err(WriteError::Database)
    }

    /// Mark the note of `branch` at `tip` looked at, **once**: a second mark keeps the first time. A `verdict`
    /// replaces the one kept, and none keeps it. Whether there is such a note.
    pub fn keep_build_note_looked_at(&mut self, branch: &str, tip: &str, at: &str, verdict: Option<&str>) -> Result<bool, WriteError> {
        let doing = "marking a note of the preview build looked at";
        self.conn
            .execute(
                "UPDATE build_notes SET looked_at = COALESCE(looked_at, ?3), verdict = COALESCE(?4, verdict) WHERE branch = ?1 AND tip = ?2",
                params![branch, tip, at, verdict],
            )
            .map(|touched| touched > 0)
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }
}
