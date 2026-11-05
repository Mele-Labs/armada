//! What the Operator would have done where its role stands at Suggest, waiting on one press.
//! Written by `fleet::operator`, whose words the text columns hold.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One suggestion, as Fleet would have acted on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptSuggestion {
    /// The row's number, once kept. Ignored as it is kept.
    pub n: i64,
    pub arrival: String,
    pub arrived: String,
    pub way: String,
    pub session_id: String,
    pub manifest_id: String,
    pub subject: String,
    pub words: String,
    pub group: Option<String>,
    pub other_session: Option<String>,
    pub shared: Vec<String>,
    pub first_session: Option<String>,
    pub step: Option<String>,
    /// The item an answer would settle.
    pub item: Option<String>,
    /// The option an answer would pick, in its words.
    pub pick: Option<String>,
    pub because: String,
    /// What the row says.
    pub text: String,
    pub raised_at: String,
}

const COLUMNS: &str = "n, arrival, arrived, way, session_id, manifest_id, subject, words, group_id, other_session, shared, \
                       first_session, step, item, pick, because, text, raised_at";

fn suggestion_of(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptSuggestion> {
    let shared: String = row.get(10)?;
    Ok(KeptSuggestion {
        n: row.get(0)?,
        arrival: row.get(1)?,
        arrived: row.get(2)?,
        way: row.get(3)?,
        session_id: row.get(4)?,
        manifest_id: row.get(5)?,
        subject: row.get(6)?,
        words: row.get(7)?,
        group: row.get(8)?,
        other_session: row.get(9)?,
        shared: shared.lines().filter(|one| !one.is_empty()).map(str::to_string).collect(),
        first_session: row.get(11)?,
        step: row.get(12)?,
        item: row.get(13)?,
        pick: row.get(14)?,
        because: row.get(15)?,
        text: row.get(16)?,
        raised_at: row.get(17)?,
    })
}

fn failed(doing: &'static str) -> impl FnOnce(rusqlite::Error) -> WriteError {
    move |cause| WriteError::Database(fault(doing)(cause))
}

impl Store {
    /// Keep one suggestion. **The newest for an arrival stands alone**: one still standing for the
    /// same arrival goes, so an item ruled again shows one row.
    pub fn keep_operator_suggestion(&mut self, one: &KeptSuggestion) -> Result<(), WriteError> {
        let doing = "keeping an Operator suggestion";
        self.conn.execute("DELETE FROM operator_suggestions WHERE arrival = ?1 AND settled_at IS NULL", [&one.arrival]).map_err(failed(doing))?;
        self.conn
            .execute(
                "INSERT INTO operator_suggestions (arrival, arrived, way, session_id, manifest_id, subject, words, group_id, other_session,
                     shared, first_session, step, item, pick, because, text, raised_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                params![
                    one.arrival,
                    one.arrived,
                    one.way,
                    one.session_id,
                    one.manifest_id,
                    one.subject,
                    one.words,
                    one.group,
                    one.other_session,
                    one.shared.join("\n"),
                    one.first_session,
                    one.step,
                    one.item,
                    one.pick,
                    one.because,
                    one.text,
                    one.raised_at
                ],
            )
            .map(|_| ())
            .map_err(failed(doing))
    }

    /// The suggestions standing on `session`, oldest first. The empty id is the ones no Job holds.
    pub fn operator_suggestions_of(&self, session: &str) -> Result<Vec<KeptSuggestion>, WriteError> {
        let doing = "reading a Job's Operator suggestions";
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM operator_suggestions WHERE session_id = ?1 AND settled_at IS NULL ORDER BY n"))
            .map_err(failed(doing))?;
        let rows = statement.query_map([session], suggestion_of).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// One suggestion still standing, by its number.
    pub fn operator_suggestion(&self, n: i64) -> Result<Option<KeptSuggestion>, WriteError> {
        let doing = "reading an Operator suggestion";
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM operator_suggestions WHERE n = ?1 AND settled_at IS NULL"))
            .map_err(failed(doing))?;
        let mut rows = statement.query_map([n], suggestion_of).map_err(failed(doing))?;
        rows.next().transpose().map_err(failed(doing))
    }

    /// Whether suggestion `n` was ever kept: its Job (empty where it has none) and whether it is settled.
    pub fn operator_suggestion_seen(&self, n: i64) -> Result<Option<(String, bool)>, WriteError> {
        let doing = "reading an Operator suggestion";
        let mut statement = self.conn.prepare("SELECT session_id, settled_at IS NOT NULL FROM operator_suggestions WHERE n = ?1").map_err(failed(doing))?;
        let mut rows = statement.query_map([n], |row| Ok((row.get::<_, String>(0)?, row.get::<_, bool>(1)?))).map_err(failed(doing))?;
        rows.next().transpose().map_err(failed(doing))
    }

    /// Settle a suggestion, taken or skipped. **The first press wins**: false where it is settled already.
    pub fn settle_operator_suggestion(&mut self, n: i64, taken: bool, at: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("UPDATE operator_suggestions SET settled_at = ?2, taken = ?3 WHERE n = ?1 AND settled_at IS NULL", params![n, at, taken])
            .map(|moved| moved > 0)
            .map_err(failed("settling an Operator suggestion"))
    }
}
