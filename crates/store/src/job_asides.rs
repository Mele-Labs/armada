//! What the owner asked a Job on the side, and what a read-only copy of its conversation answered. Kept
//! beside the Job's record and never in its thread: the Job's agent reads an aside only once the owner tells it.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// Where an aside stands, as the table names it.
pub mod aside_state {
    pub const ASKING: &str = "asking";
    pub const ANSWERED: &str = "answered";
    pub const FAILED: &str = "failed";
}

/// One aside, as kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptAside {
    pub aside_id: String,
    pub session_id: String,
    pub question: String,
    /// The text the owner selected in the thread when he asked.
    pub quote: Option<String>,
    /// What has arrived. Grows while the aside is asking.
    pub answer: String,
    pub state: String,
    /// Fleet's sentence for why a failed aside failed.
    pub failed: Option<String>,
    pub asked_at: String,
    pub answered_at: Option<String>,
    /// When the owner told it to the Job. Absent until he does.
    pub told_at: Option<String>,
    /// What the copy read while it answered, one `<Tool> <target>` each, in order. Kept as one line apiece, so an
    /// entry never carries a newline.
    pub reads: Vec<String>,
}

const COLUMNS: &str = "aside_id, session_id, question, quote, answer, state, failed, asked_at, answered_at, told_at, reads";

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

fn kept(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptAside> {
    Ok(KeptAside {
        aside_id: row.get(0)?,
        session_id: row.get(1)?,
        question: row.get(2)?,
        quote: row.get(3)?,
        answer: row.get(4)?,
        state: row.get(5)?,
        failed: row.get(6)?,
        asked_at: row.get(7)?,
        answered_at: row.get(8)?,
        told_at: row.get(9)?,
        reads: row.get::<_, String>(10)?.lines().filter(|line| !line.is_empty()).map(str::to_string).collect(),
    })
}

impl Store {
    /// Keep `aside`, replacing the one of its id. **`told_at` is not overwritten by a later write of the
    /// answer**: a row already told keeps the time it was.
    pub fn keep_aside(&mut self, aside: &KeptAside) -> Result<(), WriteError> {
        self.conn
            .execute(
                &format!(
                    "INSERT INTO job_asides ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11) \
                     ON CONFLICT (aside_id) DO UPDATE SET answer = ?5, state = ?6, failed = ?7, answered_at = ?9, reads = ?11, \
                     told_at = COALESCE(job_asides.told_at, ?10)"
                ),
                params![
                    aside.aside_id,
                    aside.session_id,
                    aside.question,
                    aside.quote,
                    aside.answer,
                    aside.state,
                    aside.failed,
                    aside.asked_at,
                    aside.answered_at,
                    aside.told_at,
                    aside.reads.join("\n"),
                ],
            )
            .map(|_| ())
            .map_err(failed("keeping an aside"))
    }

    /// The asides of `session`, oldest first.
    pub fn asides_of(&self, session: &str) -> Result<Vec<KeptAside>, WriteError> {
        let doing = "reading the asides of a Job";
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM job_asides WHERE session_id = ?1 ORDER BY asked_at, rowid"))
            .map_err(failed(doing))?;
        let rows = statement.query_map([session], kept).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// The aside `aside_id` of `session`.
    pub fn aside_of(&self, session: &str, aside_id: &str) -> Result<Option<KeptAside>, WriteError> {
        self.conn
            .query_row(&format!("SELECT {COLUMNS} FROM job_asides WHERE session_id = ?1 AND aside_id = ?2"), params![session, aside_id], kept)
            .optional()
            .map_err(failed("reading an aside"))
    }

    /// Mark the aside told at `at`, **once**: whether this call moved it.
    pub fn keep_aside_told(&mut self, session: &str, aside_id: &str, at: &str) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE job_asides SET told_at = ?3 WHERE session_id = ?1 AND aside_id = ?2 AND told_at IS NULL",
                params![session, aside_id, at],
            )
            .map(|moved| moved > 0)
            .map_err(failed("marking an aside told"))
    }
}
