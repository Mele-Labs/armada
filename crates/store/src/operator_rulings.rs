//! The Operator's Log: what it ruled on each arrival, and each arrival it passed on untouched.
//! Written by `fleet::operator`, whose words the text columns hold.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One row of the Log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptRuling {
    pub arrival: String,
    pub arrived: String,
    pub source: String,
    /// The Job a start came from, where another Job started it.
    pub source_session: Option<String>,
    pub session_id: String,
    pub manifest_id: String,
    pub subject: String,
    pub outcome: String,
    pub other_session: Option<String>,
    pub shared: Vec<String>,
    pub because: String,
    /// The call that ruled it, or `None` where none was made.
    pub call: Option<String>,
    /// The signer's wire word.
    pub by: String,
    pub at: String,
}

const COLUMNS: &str = "arrival, arrived, source, source_session, session_id, manifest_id, subject, outcome, \
                       other_session, shared, because, call, by, at";

fn ruling_of(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptRuling> {
    let shared: String = row.get(9)?;
    Ok(KeptRuling {
        arrival: row.get(0)?,
        arrived: row.get(1)?,
        source: row.get(2)?,
        source_session: row.get(3)?,
        session_id: row.get(4)?,
        manifest_id: row.get(5)?,
        subject: row.get(6)?,
        outcome: row.get(7)?,
        other_session: row.get(8)?,
        shared: shared.lines().filter(|one| !one.is_empty()).map(str::to_string).collect(),
        because: row.get(10)?,
        call: row.get(11)?,
        by: row.get(12)?,
        at: row.get(13)?,
    })
}

fn failed(doing: &'static str) -> impl FnOnce(rusqlite::Error) -> WriteError {
    move |cause| WriteError::Database(fault(doing)(cause))
}

impl Store {
    /// Keep one row of the Log.
    pub fn add_operator_ruling(&mut self, one: &KeptRuling) -> Result<(), WriteError> {
        self.conn
            .execute(
                &format!(
                    "INSERT INTO operator_rulings ({COLUMNS})
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"
                ),
                params![
                    one.arrival,
                    one.arrived,
                    one.source,
                    one.source_session,
                    one.session_id,
                    one.manifest_id,
                    one.subject,
                    one.outcome,
                    one.other_session,
                    one.shared.join("\n"),
                    one.because,
                    one.call,
                    one.by,
                    one.at
                ],
            )
            .map(|_| ())
            .map_err(failed("keeping an Operator ruling"))
    }

    /// The latest rows of the Log, newest first.
    pub fn operator_rulings(&self, limit: u32) -> Result<Vec<KeptRuling>, WriteError> {
        let doing = "reading the Operator's Log";
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM operator_rulings ORDER BY position DESC LIMIT ?1"))
            .map_err(failed(doing))?;
        let rows = statement.query_map([limit], ruling_of).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// How many calls the Log names at or after `since`, an RFC3339 instant.
    pub fn operator_calls_since(&self, since: &str) -> Result<u32, WriteError> {
        self.conn
            .query_row(
                "SELECT COUNT(DISTINCT call) FROM operator_rulings WHERE call IS NOT NULL AND at >= ?1",
                [since],
                |row| row.get::<_, u32>(0),
            )
            .map_err(failed("counting the Operator's calls"))
    }
}
