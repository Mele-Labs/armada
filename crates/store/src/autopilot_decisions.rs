//! What a running Autopilot answered for the person, kept for them to review. A repository's, kept
//! until reviewed.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One answer. `kind` is the word of the box that allowed it, `asked` what was put to the helper
/// and `because` the helper's own reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptDecision {
    pub id: String,
    pub manifest_id: String,
    /// The Job it answered for, or started to. A correction is sent to it.
    pub session_id: Option<String>,
    pub who: String,
    pub kind: String,
    pub asked: String,
    pub chose: String,
    pub because: String,
    pub at: String,
    /// The person kept or changed it: it leaves the review.
    pub reviewed: bool,
    pub corrected: Option<String>,
}

const COLUMNS: &str =
    "id, manifest_id, session_id, who, kind, asked, chose, because, at, reviewed, corrected";

fn decision_of(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptDecision> {
    Ok(KeptDecision {
        id: row.get(0)?,
        manifest_id: row.get(1)?,
        session_id: row.get(2)?,
        who: row.get(3)?,
        kind: row.get(4)?,
        asked: row.get(5)?,
        chose: row.get(6)?,
        because: row.get(7)?,
        at: row.get(8)?,
        reviewed: row.get::<_, i64>(9)? != 0,
        corrected: row.get(10)?,
    })
}

fn failed(doing: &'static str) -> impl FnOnce(rusqlite::Error) -> WriteError {
    move |cause| WriteError::Database(fault(doing)(cause))
}

impl Store {
    /// Every decision not yet reviewed, in the order they were made, any repository's.
    pub fn autopilot_decisions_to_review(&self) -> Result<Vec<KeptDecision>, WriteError> {
        let doing = "reading what Autopilot decided";
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM autopilot_decisions WHERE reviewed = 0 ORDER BY position"
            ))
            .map_err(failed(doing))?;
        let rows = statement.query_map([], decision_of).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// One repository's latest decisions, newest first, reviewed or not: **what its helper reads
    /// of how this repository's person has been answered**, corrections included.
    pub fn autopilot_decisions_of(&self, manifest_id: &str, limit: u32) -> Result<Vec<KeptDecision>, WriteError> {
        let doing = "reading a repository's earlier decisions";
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM autopilot_decisions WHERE manifest_id = ?1
                 ORDER BY position DESC LIMIT ?2"
            ))
            .map_err(failed(doing))?;
        let rows = statement.query_map(params![manifest_id, limit], decision_of).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    pub fn autopilot_decision(&self, id: &str) -> Result<Option<KeptDecision>, WriteError> {
        self.conn
            .query_row(&format!("SELECT {COLUMNS} FROM autopilot_decisions WHERE id = ?1"), [id], decision_of)
            .optional()
            .map_err(failed("reading one Autopilot decision"))
    }

    /// Keep a decision. **False where its id is already there**, so an item is answered once.
    pub fn add_autopilot_decision(&mut self, one: &KeptDecision) -> Result<bool, WriteError> {
        self.conn
            .execute(
                &format!(
                    "INSERT OR IGNORE INTO autopilot_decisions ({COLUMNS})
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
                ),
                params![
                    one.id, one.manifest_id, one.session_id, one.who, one.kind, one.asked, one.chose,
                    one.because, one.at, one.reviewed, one.corrected
                ],
            )
            .map(|changed| changed > 0)
            .map_err(failed("keeping an Autopilot decision"))
    }

    /// Mark a decision corrected with the person's words, and reviewed. False where no row has the id.
    pub fn correct_autopilot_decision(&mut self, id: &str, text: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("UPDATE autopilot_decisions SET corrected = ?2, reviewed = 1 WHERE id = ?1", params![id, text])
            .map(|changed| changed > 0)
            .map_err(failed("correcting an Autopilot decision"))
    }

    /// Mark a decision reviewed: the person kept it. False where no row has the id.
    pub fn review_autopilot_decision(&mut self, id: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("UPDATE autopilot_decisions SET reviewed = 1 WHERE id = ?1", [id])
            .map(|changed| changed > 0)
            .map_err(failed("reviewing an Autopilot decision"))
    }
}
