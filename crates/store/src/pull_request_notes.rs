//! The owner's notes on a pull request's lines, kept so a Job the owner sent one to can resolve it.
//!
//! **Plain text here**: `side` and `status` are the wire's words and validation is Fleet's.

use rusqlite::{Row, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One row of `pull_request_notes`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptPullRequestNote {
    pub id: String,
    /// As the forge names it: `owner/name`.
    pub repository: String,
    pub number: u32,
    pub path: String,
    pub line: u32,
    /// `left` or `right`.
    pub side: String,
    pub body: String,
    /// `open` or `done`.
    pub status: String,
    pub sent_to: Option<String>,
    pub sent_title: Option<String>,
    pub sent_at: Option<String>,
    pub created_at: String,
}

const NOTE: &str = "id, repository, number, path, line, side, body, status, sent_to, sent_title, sent_at, created_at";

fn note(row: &Row<'_>) -> rusqlite::Result<KeptPullRequestNote> {
    Ok(KeptPullRequestNote {
        id: row.get(0)?,
        repository: row.get(1)?,
        number: row.get::<_, i64>(2)?.try_into().unwrap_or_default(),
        path: row.get(3)?,
        line: row.get::<_, i64>(4)?.try_into().unwrap_or_default(),
        side: row.get(5)?,
        body: row.get(6)?,
        status: row.get(7)?,
        sent_to: row.get(8)?,
        sent_title: row.get(9)?,
        sent_at: row.get(10)?,
        created_at: row.get(11)?,
    })
}

impl Store {
    /// Keep a new note. Answers the row as kept.
    pub fn keep_pull_request_note(&mut self, kept: &KeptPullRequestNote) -> Result<KeptPullRequestNote, WriteError> {
        let wrote = |why: rusqlite::Error| WriteError::Database(fault("keeping a pull request note")(why));
        self.conn
            .execute(
                &format!("INSERT INTO pull_request_notes ({NOTE}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"),
                rusqlite::params![
                    kept.id,
                    kept.repository,
                    i64::from(kept.number),
                    kept.path,
                    i64::from(kept.line),
                    kept.side,
                    kept.body,
                    kept.status,
                    kept.sent_to,
                    kept.sent_title,
                    kept.sent_at,
                    kept.created_at
                ],
            )
            .map_err(wrote)?;
        Ok(kept.clone())
    }

    /// One note, if there is one.
    pub fn pull_request_note(&self, id: &str) -> Result<Option<KeptPullRequestNote>, WriteError> {
        self.conn
            .query_row(&format!("SELECT {NOTE} FROM pull_request_notes WHERE id = ?1"), rusqlite::params![id], note)
            .optional()
            .map_err(fault("reading a pull request note"))
            .map_err(WriteError::Database)
    }

    /// Every note on one pull request, oldest first.
    pub fn pull_request_notes_of(&self, repository: &str, number: u32) -> Result<Vec<KeptPullRequestNote>, WriteError> {
        let unreadable = |why: rusqlite::Error| WriteError::Database(fault("reading pull request notes")(why));
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {NOTE} FROM pull_request_notes WHERE repository = ?1 AND number = ?2 ORDER BY created_at, id"
            ))
            .map_err(unreadable)?;
        let rows = statement
            .query_map(rusqlite::params![repository, i64::from(number)], note)
            .map_err(unreadable)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(unreadable)
    }

    /// Change a note's body. Answers the note, or nothing where there is none.
    pub fn edit_pull_request_note_body(&mut self, id: &str, body: &str) -> Result<Option<KeptPullRequestNote>, WriteError> {
        self.conn
            .execute("UPDATE pull_request_notes SET body = ?2 WHERE id = ?1", rusqlite::params![id, body])
            .map_err(fault("editing a pull request note"))
            .map_err(WriteError::Database)?;
        self.pull_request_note(id)
    }

    /// Record that the notes named were sent to a session. An id with no note is passed over.
    pub fn send_pull_request_notes(
        &mut self,
        ids: &[String],
        session_id: &str,
        title: &str,
        at: &str,
    ) -> Result<(), WriteError> {
        let wrote = |why: rusqlite::Error| WriteError::Database(fault("recording sent pull request notes")(why));
        let transaction = self.conn.transaction().map_err(wrote)?;
        for id in ids {
            transaction
                .execute(
                    "UPDATE pull_request_notes SET sent_to = ?2, sent_title = ?3, sent_at = ?4 WHERE id = ?1",
                    rusqlite::params![id, session_id, title, at],
                )
                .map_err(wrote)?;
        }
        transaction.commit().map_err(wrote)
    }

    /// Mark a note done. Answers the note, or nothing where there is none.
    pub fn finish_pull_request_note(&mut self, id: &str) -> Result<Option<KeptPullRequestNote>, WriteError> {
        self.conn
            .execute("UPDATE pull_request_notes SET status = 'done' WHERE id = ?1", rusqlite::params![id])
            .map_err(fault("finishing a pull request note"))
            .map_err(WriteError::Database)?;
        self.pull_request_note(id)
    }

    /// Forget a note. One already gone is not an error.
    pub fn forget_pull_request_note(&mut self, id: &str) -> Result<(), WriteError> {
        self.conn
            .execute("DELETE FROM pull_request_notes WHERE id = ?1", rusqlite::params![id])
            .map(|_| ())
            .map_err(fault("forgetting a pull request note"))
            .map_err(WriteError::Database)
    }
}
