//! A job's word that a Fleet notice was wrong, the review job that settles it, and the notices a
//! job's replies are read against.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// A dispute of one Fleet notice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptDispute {
    pub id: String,
    pub session_id: String,
    /// The Fleet message row's id in the job's thread.
    pub notice_row: String,
    pub notice: String,
    pub why: String,
    /// `called` or `read`.
    pub spotted: String,
    pub at: String,
    /// What a second dispute of the same notice is matched by.
    pub key: String,
    /// The job started or joined to settle it.
    pub review: Option<String>,
}

/// A notice Fleet sent a hosted job that asks it something.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptNotice {
    pub session_id: String,
    pub row_id: String,
    pub at: String,
    pub text: String,
}

const DISPUTE_COLUMNS: &str = "id, session_id, notice_row, notice, why, spotted, at, notice_key, review";

fn kept_dispute(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptDispute> {
    Ok(KeptDispute {
        id: row.get(0)?,
        session_id: row.get(1)?,
        notice_row: row.get(2)?,
        notice: row.get(3)?,
        why: row.get(4)?,
        spotted: row.get(5)?,
        at: row.get(6)?,
        key: row.get(7)?,
        review: row.get(8)?,
    })
}

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

impl Store {
    pub fn keep_dispute(&mut self, dispute: &KeptDispute) -> Result<(), WriteError> {
        self.conn
            .execute(
                &format!("INSERT INTO fleet_disputes ({DISPUTE_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"),
                params![
                    dispute.id,
                    dispute.session_id,
                    dispute.notice_row,
                    dispute.notice,
                    dispute.why,
                    dispute.spotted,
                    dispute.at,
                    dispute.key,
                    dispute.review
                ],
            )
            .map(|_| ())
            .map_err(failed("keeping a dispute of a Fleet notice"))
    }

    /// Name the job that settles dispute `id`.
    pub fn keep_dispute_review(&mut self, id: &str, review: &str) -> Result<(), WriteError> {
        self.conn
            .execute("UPDATE fleet_disputes SET review = ?2 WHERE id = ?1", params![id, review])
            .map(|_| ())
            .map_err(failed("keeping a dispute's review job"))
    }

    pub fn dispute(&self, id: &str) -> Result<Option<KeptDispute>, WriteError> {
        self.conn
            .query_row(&format!("SELECT {DISPUTE_COLUMNS} FROM fleet_disputes WHERE id = ?1"), [id], kept_dispute)
            .optional()
            .map_err(failed("reading a dispute of a Fleet notice"))
    }

    /// The review jobs disputes matched by `key` were settled by, newest first.
    pub fn dispute_reviews(&self, key: &str) -> Result<Vec<String>, WriteError> {
        let doing = "reading the reviews of a disputed notice";
        let mut statement = self
            .conn
            .prepare("SELECT review FROM fleet_disputes WHERE notice_key = ?1 AND review IS NOT NULL GROUP BY review ORDER BY MAX(at) DESC")
            .map_err(failed(doing))?;
        let rows = statement.query_map([key], |row| row.get::<_, String>(0)).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Whether the notice row `notice_row` of `session_id` is disputed already.
    pub fn notice_disputed(&self, session_id: &str, notice_row: &str) -> Result<bool, WriteError> {
        self.conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM fleet_disputes WHERE session_id = ?1 AND notice_row = ?2)",
                params![session_id, notice_row],
                |row| row.get::<_, bool>(0),
            )
            .map_err(failed("reading whether a notice is disputed"))
    }

    /// Keep a notice that asks a job something, unread.
    pub fn keep_fleet_notice(&mut self, notice: &KeptNotice) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO fleet_notices (session_id, row_id, at, text) VALUES (?1, ?2, ?3, ?4)",
                params![notice.session_id, notice.row_id, notice.at, notice.text],
            )
            .map(|_| ())
            .map_err(failed("keeping a Fleet notice"))
    }

    /// Every notice whose replies have not been read, oldest first.
    pub fn unread_fleet_notices(&self) -> Result<Vec<KeptNotice>, WriteError> {
        let doing = "reading the Fleet notices not yet read";
        let mut statement = self
            .conn
            .prepare("SELECT session_id, row_id, at, text FROM fleet_notices WHERE read_at IS NULL ORDER BY at")
            .map_err(failed(doing))?;
        let rows = statement
            .query_map([], |row| {
                Ok(KeptNotice { session_id: row.get(0)?, row_id: row.get(1)?, at: row.get(2)?, text: row.get(3)? })
            })
            .map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Mark a notice read, so it is never read again. False where it was already, or is not kept.
    pub fn fleet_notice_read(&mut self, session_id: &str, row_id: &str, at: &str) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE fleet_notices SET read_at = ?3 WHERE session_id = ?1 AND row_id = ?2 AND read_at IS NULL",
                params![session_id, row_id, at],
            )
            .map(|changed| changed > 0)
            .map_err(failed("marking a Fleet notice read"))
    }
}
