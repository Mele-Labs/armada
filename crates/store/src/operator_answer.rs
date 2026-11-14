//! The person's own answers to a Job's questions. Written by `fleet`, whose words the text columns hold.

use rusqlite::{params, params_from_iter};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One answer he gave to a Job's question.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptAnswer {
    /// The row's number, once kept. Ignored as it is kept.
    pub n: i64,
    pub session: String,
    pub manifest_id: String,
    /// The Job's title.
    pub job: Option<String>,
    pub item_id: String,
    /// The item's text.
    pub question: String,
    /// The labels the item offered, as a JSON array.
    pub options: String,
    /// The label he pressed.
    pub choice: Option<String>,
    pub words: Option<String>,
    pub answered_at: String,
    /// `job`, `operator_page` or `thread`.
    pub via: String,
}

const COLUMNS: &str = "n, session, manifest_id, job, item_id, question, options, choice, words, answered_at, via";

fn row_of(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptAnswer> {
    Ok(KeptAnswer {
        n: row.get(0)?,
        session: row.get(1)?,
        manifest_id: row.get(2)?,
        job: row.get(3)?,
        item_id: row.get(4)?,
        question: row.get(5)?,
        options: row.get(6)?,
        choice: row.get(7)?,
        words: row.get(8)?,
        answered_at: row.get(9)?,
        via: row.get(10)?,
    })
}

fn failed(doing: &'static str) -> impl FnOnce(rusqlite::Error) -> WriteError {
    move |cause| WriteError::Database(fault(doing)(cause))
}

impl Store {
    /// Keep one answer. Answers its row's number.
    pub fn add_operator_answer(&mut self, one: &KeptAnswer) -> Result<i64, WriteError> {
        self.conn
            .execute(
                "INSERT INTO operator_answer (session, manifest_id, job, item_id, question, options, choice, words, answered_at, via) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![one.session, one.manifest_id, one.job, one.item_id, one.question, one.options, one.choice, one.words, one.answered_at, one.via],
            )
            .map_err(failed("keeping an answer of the person's"))?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Whether no answer is kept yet.
    pub fn operator_answers_empty(&self) -> Result<bool, WriteError> {
        let any: i64 = self.conn.query_row("SELECT count(*) FROM (SELECT 1 FROM operator_answer LIMIT 1)", [], |row| row.get(0)).map_err(failed("reading the person's answers"))?;
        Ok(any == 0)
    }

    /// The newest `limit` answers in the repositories `manifests`, newest first.
    pub fn operator_answers_of(&self, manifests: &[String], limit: usize) -> Result<Vec<KeptAnswer>, WriteError> {
        if manifests.is_empty() {
            return Ok(Vec::new());
        }
        let doing = "reading the person's answers";
        let marks = vec!["?"; manifests.len()].join(", ");
        let sql = format!("SELECT {COLUMNS} FROM operator_answer WHERE manifest_id IN ({marks}) ORDER BY n DESC LIMIT {limit}");
        let mut statement = self.conn.prepare(&sql).map_err(failed(doing))?;
        let rows = statement.query_map(params_from_iter(manifests.iter()), row_of).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Answer `n`.
    pub fn operator_answer(&self, n: i64) -> Result<Option<KeptAnswer>, WriteError> {
        let doing = "reading the person's answers";
        let mut statement = self.conn.prepare(&format!("SELECT {COLUMNS} FROM operator_answer WHERE n = ?1")).map_err(failed(doing))?;
        let rows = statement.query_map(params![n], row_of).map_err(failed(doing))?;
        Ok(rows.collect::<Result<Vec<_>, _>>().map_err(failed(doing))?.into_iter().next())
    }
}
