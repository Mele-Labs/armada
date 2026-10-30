//! The copy of the lifecycles and rules a Job started with.
//!
//! **A file's text as it was read, and not what the file says now**, so a rule saved afterwards
//! reaches only the Jobs started after it. Text in, text out: reading it back into a lifecycle or a
//! rule is Fleet's, because the store is the one crate that knows no domain type.
//!
//! **Only a Job with a row in `job_copies` has one.** A Job started before the table existed has
//! none, and Fleet reads the lifecycles that apply and no rules for it.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// What a kept piece is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PieceKind {
    Lifecycle,
    Rule,
}

impl PieceKind {
    fn as_str(self) -> &'static str {
        match self {
            PieceKind::Lifecycle => "lifecycle",
            PieceKind::Rule => "rule",
        }
    }
}

/// One file of the copy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptPiece {
    pub kind: PieceKind,
    /// The id the piece goes by, so the copy holds one of each.
    pub piece_id: String,
    /// Where it was read, for a sentence that names the file.
    pub path: String,
    pub text: String,
}

/// Everything a Job froze: its files, and the autonomy defaults laid over each other for its
/// repository, `(act kind, autonomy)` as each is spelled on the wire.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptCopy {
    pub pieces: Vec<KeptPiece>,
    pub autonomy: Vec<(String, String)>,
}

fn failing(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

impl Store {
    /// Keep the copy `session_id` started with. **Once**: a Job that has one keeps it, and false
    /// says so, since a second freeze would be a Job reading files it did not start with.
    pub fn freeze_job_copy(
        &mut self,
        session_id: &str,
        since: &str,
        copy: &KeptCopy,
    ) -> Result<bool, WriteError> {
        let doing = "keeping the copy of the lifecycles and rules a job started with";
        let tx = self.conn.transaction().map_err(failing(doing))?;
        let new = tx
            .execute("INSERT OR IGNORE INTO job_copies (session_id, since) VALUES (?1, ?2)", params![session_id, since])
            .map_err(failing(doing))?
            > 0;
        if !new {
            return Ok(false);
        }
        for piece in &copy.pieces {
            tx.execute(
                "INSERT OR REPLACE INTO job_copy_pieces (session_id, kind, piece_id, path, text) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![session_id, piece.kind.as_str(), piece.piece_id, piece.path, piece.text],
            )
            .map_err(failing(doing))?;
        }
        for (kind, autonomy) in &copy.autonomy {
            tx.execute(
                "INSERT OR REPLACE INTO job_copy_autonomy (session_id, act_kind, autonomy) VALUES (?1, ?2, ?3)",
                params![session_id, kind, autonomy],
            )
            .map_err(failing(doing))?;
        }
        tx.commit().map_err(failing(doing))?;
        Ok(true)
    }

    /// Whether the Job froze a copy. A look at every Job asks, so it reads no file.
    pub fn has_job_copy(&self, session_id: &str) -> Result<bool, WriteError> {
        self.conn
            .query_row("SELECT 1 FROM job_copies WHERE session_id = ?1", params![session_id], |_| Ok(()))
            .optional()
            .map(|found| found.is_some())
            .map_err(failing("asking whether a job froze a copy of its lifecycles and rules"))
    }

    /// The live Jobs that froze a copy, for what a Fleet that stopped mid-way owes them.
    pub fn live_job_copies(&self) -> Result<Vec<String>, WriteError> {
        let doing = "reading the live jobs that froze a copy of their lifecycles and rules";
        self.conn
            .prepare(
                "SELECT c.session_id FROM job_copies c JOIN sessions s ON s.id = c.session_id
                 WHERE s.state = 'live' ORDER BY c.session_id",
            )
            .map_err(failing(doing))?
            .query_map([], |row| row.get(0))
            .and_then(|rows| rows.collect::<Result<Vec<String>, _>>())
            .map_err(failing(doing))
    }

    /// The copy the Job started with. `None` for a Job that froze none.
    pub fn job_copy_of(&self, session_id: &str) -> Result<Option<KeptCopy>, WriteError> {
        let doing = "reading the copy of the lifecycles and rules a job started with";
        let frozen = self
            .conn
            .query_row("SELECT 1 FROM job_copies WHERE session_id = ?1", params![session_id], |_| Ok(()))
            .optional()
            .map_err(failing(doing))?
            .is_some();
        if !frozen {
            return Ok(None);
        }
        let pieces = self
            .conn
            .prepare(
                "SELECT kind, piece_id, path, text FROM job_copy_pieces WHERE session_id = ?1 ORDER BY kind, piece_id",
            )
            .map_err(failing(doing))?
            .query_map(params![session_id], |row| {
                let kind: String = row.get(0)?;
                Ok(KeptPiece {
                    kind: if kind == "lifecycle" { PieceKind::Lifecycle } else { PieceKind::Rule },
                    piece_id: row.get(1)?,
                    path: row.get(2)?,
                    text: row.get(3)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(failing(doing))?;
        let autonomy = self
            .conn
            .prepare("SELECT act_kind, autonomy FROM job_copy_autonomy WHERE session_id = ?1 ORDER BY act_kind")
            .map_err(failing(doing))?
            .query_map(params![session_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(failing(doing))?;
        Ok(Some(KeptCopy { pieces, autonomy }))
    }
}
