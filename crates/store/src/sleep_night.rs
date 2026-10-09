//! Sleep mode's night: whether it is on, since when, and the rows the night left. `docs/concepts/session.md`.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One thing the night left. `kind` is `decided`, `blocked`, `landed` or `walk`; `text` is what was
/// asked, what holds, the pull request's title or the walk's title.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SleepRow {
    pub id: String,
    pub kind: String,
    pub who: String,
    pub text: String,
    pub session_id: Option<String>,
    pub chose: Option<String>,
    pub corrected: Option<String>,
    pub pr: Option<String>,
}

/// Whether sleep is on, and since when (RFC3339).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SleepSwitch {
    pub on: bool,
    pub since: Option<String>,
}

const COLUMNS: &str = "id, kind, who, text, session_id, chose, corrected, pr";

fn row_of(row: &rusqlite::Row<'_>) -> rusqlite::Result<SleepRow> {
    Ok(SleepRow {
        id: row.get(0)?,
        kind: row.get(1)?,
        who: row.get(2)?,
        text: row.get(3)?,
        session_id: row.get(4)?,
        chose: row.get(5)?,
        corrected: row.get(6)?,
        pr: row.get(7)?,
    })
}

impl Store {
    pub fn sleep_switch(&self) -> Result<SleepSwitch, WriteError> {
        self.conn
            .query_row("SELECT on_now, since FROM sleep_state", [], |row| {
                Ok(SleepSwitch { on: row.get::<_, i64>(0)? != 0, since: row.get(1)? })
            })
            .map_err(fault("reading whether sleep is on"))
            .map_err(WriteError::Database)
    }

    /// The night's rows in the order they were left.
    pub fn sleep_rows(&self) -> Result<Vec<SleepRow>, WriteError> {
        let doing = "reading the night's rows";
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM sleep_rows ORDER BY position"))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement.query_map([], row_of).map_err(fault(doing)).map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }

    pub fn sleep_row(&self, id: &str) -> Result<Option<SleepRow>, WriteError> {
        self.conn
            .query_row(&format!("SELECT {COLUMNS} FROM sleep_rows WHERE id = ?1"), [id], row_of)
            .optional()
            .map_err(fault("reading one row of the night"))
            .map_err(WriteError::Database)
    }

    /// Turn sleep on: a new night, so the last one's rows go.
    pub fn begin_sleep(&mut self, since: &str) -> Result<(), WriteError> {
        let kept = (|| {
            let transaction = self.conn.transaction()?;
            transaction.execute("DELETE FROM sleep_rows", [])?;
            transaction.execute("UPDATE sleep_state SET on_now = 1, since = ?1", [since])?;
            transaction.commit()
        })();
        kept.map_err(fault("starting a night")).map_err(WriteError::Database)
    }

    /// Turn sleep off. The night's rows stay for the review.
    pub fn end_sleep(&mut self) -> Result<(), WriteError> {
        self.conn
            .execute("UPDATE sleep_state SET on_now = 0", [])
            .map(|_| ())
            .map_err(fault("ending a night"))
            .map_err(WriteError::Database)
    }

    /// Keep a row. **False where its id is already there**, so a row is left once.
    pub fn add_sleep_row(&mut self, row: &SleepRow) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO sleep_rows
                 (id, kind, who, text, session_id, chose, corrected, pr)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![row.id, row.kind, row.who, row.text, row.session_id, row.chose, row.corrected, row.pr],
            )
            .map(|changed| changed > 0)
            .map_err(fault("leaving a row of the night"))
            .map_err(WriteError::Database)
    }

    /// Mark a decision corrected with the owner's words. False where no row has the id.
    pub fn correct_sleep_row(&mut self, id: &str, text: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("UPDATE sleep_rows SET corrected = ?2 WHERE id = ?1 AND kind = 'decided'", params![id, text])
            .map(|changed| changed > 0)
            .map_err(fault("correcting a decision of the night"))
            .map_err(WriteError::Database)
    }
}
