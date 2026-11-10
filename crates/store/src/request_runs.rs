//! The runs of a saved request: one row for each send, by the owner or a Job's agent, kept after
//! the Job ends. `docs/practices/protocol.md`, *Requests*.
//!
//! **Keyed by the request, not the Job**: the repository and the collection's and request's own
//! ids. A request keeps its newest [`RUNS_KEPT`] and the write that goes past drops the oldest in
//! the same transaction, so the table never holds more for one request. Nothing here holds a
//! secret's value: Fleet hides each one before it hands a run over.

use rusqlite::types::Type;
use rusqlite::{params_from_iter, Row, ToSql};

use crate::error::{fault, DatabaseFault};
use crate::open::Store;

/// The most runs one request keeps. Chosen, not measured: a day's testing of one endpoint, with a
/// Job's loop on top, and few enough that a request's rows stay a few megabytes at their caps.
pub const RUNS_KEPT: usize = 200;

/// A run to keep.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewRun {
    pub repository: String,
    pub collection_id: String,
    pub request_id: String,
    pub sent_at: String,
    /// `owner` or `agent`.
    pub sent_by: String,
    pub session_id: Option<String>,
    /// The Job's title when it was sent, for a Job the ledger no longer names.
    pub session_title: Option<String>,
    pub environment: Option<String>,
    pub method: String,
    pub url: String,
    pub request_headers: Vec<(String, String)>,
    pub request_body: Option<String>,
    pub status: Option<u16>,
    pub ms: u64,
    pub response_headers: Vec<(String, String)>,
    pub response_body: String,
    /// What the answer weighed as it came, which `response_body` may be the start of.
    pub response_bytes: u64,
    pub response_cut: bool,
    pub error: Option<String>,
}

/// A run as kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptRequestRun {
    /// Rises with each run kept, and is never given again.
    pub id: i64,
    pub run: NewRun,
}

/// Which status a read wants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusWanted {
    /// 200 to 299 for `2`, and so on.
    Class(u16),
    /// Nothing answered.
    Failed,
}

/// What a list of one request's runs is asked for.
#[derive(Clone, Copy, Debug)]
pub struct RunsAsked<'a> {
    pub repository: &'a str,
    pub collection_id: &'a str,
    pub request_id: &'a str,
    pub sent_by: Option<&'a str>,
    pub status: Option<StatusWanted>,
    /// Only runs older than this one.
    pub before: Option<i64>,
    pub most: usize,
}

const COLUMNS: &str = "id, repository, collection_id, request_id, sent_at, sent_by, session_id, session_title, \
                       environment, method, url, request_headers, request_body, status, ms, response_headers, \
                       response_body, response_bytes, response_cut, error";

fn pairs(text: String, at: usize) -> Result<Vec<(String, String)>, rusqlite::Error> {
    serde_json::from_str(&text).map_err(|cause| rusqlite::Error::FromSqlConversionFailure(at, Type::Text, Box::new(cause)))
}

fn kept(row: &Row<'_>) -> Result<KeptRequestRun, rusqlite::Error> {
    Ok(KeptRequestRun {
        id: row.get(0)?,
        run: NewRun {
            repository: row.get(1)?,
            collection_id: row.get(2)?,
            request_id: row.get(3)?,
            sent_at: row.get(4)?,
            sent_by: row.get(5)?,
            session_id: row.get(6)?,
            session_title: row.get(7)?,
            environment: row.get(8)?,
            method: row.get(9)?,
            url: row.get(10)?,
            request_headers: pairs(row.get(11)?, 11)?,
            request_body: row.get(12)?,
            status: row.get::<_, Option<i64>>(13)?.and_then(|status| u16::try_from(status).ok()),
            ms: u64::try_from(row.get::<_, i64>(14)?).unwrap_or(0),
            response_headers: pairs(row.get(15)?, 15)?,
            response_body: row.get(16)?,
            response_bytes: u64::try_from(row.get::<_, i64>(17)?).unwrap_or(0),
            response_cut: row.get(18)?,
            error: row.get(19)?,
        },
    })
}

fn written(headers: &[(String, String)]) -> String {
    serde_json::to_string(headers).unwrap_or_else(|_| String::from("[]"))
}

impl Store {
    /// Keep `run`, and drop this request's oldest runs past [`RUNS_KEPT`]. Answers its id.
    pub fn keep_request_run(&mut self, run: &NewRun) -> Result<i64, DatabaseFault> {
        let doing = "keeping a run of a saved request";
        let tx = self.conn.transaction().map_err(fault(doing))?;
        tx.execute(
            "INSERT INTO request_runs (repository, collection_id, request_id, sent_at, sent_by, session_id, \
             session_title, environment, method, url, request_headers, request_body, status, ms, \
             response_headers, response_body, response_bytes, response_cut, error) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            rusqlite::params![
                run.repository,
                run.collection_id,
                run.request_id,
                run.sent_at,
                run.sent_by,
                run.session_id,
                run.session_title,
                run.environment,
                run.method,
                run.url,
                written(&run.request_headers),
                run.request_body,
                run.status.map(i64::from),
                i64::try_from(run.ms).unwrap_or(i64::MAX),
                written(&run.response_headers),
                run.response_body,
                i64::try_from(run.response_bytes).unwrap_or(i64::MAX),
                run.response_cut,
                run.error,
            ],
        )
        .map_err(fault(doing))?;
        let id = tx.last_insert_rowid();
        tx.execute(
            "DELETE FROM request_runs WHERE repository = ?1 AND collection_id = ?2 AND request_id = ?3 \
             AND id NOT IN (SELECT id FROM request_runs WHERE repository = ?1 AND collection_id = ?2 \
             AND request_id = ?3 ORDER BY id DESC LIMIT ?4)",
            rusqlite::params![run.repository, run.collection_id, run.request_id, RUNS_KEPT as i64],
        )
        .map_err(fault(doing))?;
        tx.commit().map_err(fault(doing))?;
        Ok(id)
    }

    /// One request's runs, newest first, up to `asked.most` and the ones `asked` leaves in.
    pub fn request_runs(&self, asked: &RunsAsked<'_>) -> Result<Vec<KeptRequestRun>, DatabaseFault> {
        let doing = "reading the runs of a saved request";
        let mut sql = format!(
            "SELECT {COLUMNS} FROM request_runs WHERE repository = ?1 AND collection_id = ?2 AND request_id = ?3"
        );
        let mut values: Vec<Box<dyn ToSql>> = vec![
            Box::new(asked.repository.to_string()),
            Box::new(asked.collection_id.to_string()),
            Box::new(asked.request_id.to_string()),
        ];
        if let Some(by) = asked.sent_by {
            values.push(Box::new(by.to_string()));
            sql.push_str(&format!(" AND sent_by = ?{}", values.len()));
        }
        match asked.status {
            Some(StatusWanted::Class(class)) => {
                values.push(Box::new(i64::from(class) * 100));
                values.push(Box::new(i64::from(class) * 100 + 99));
                sql.push_str(&format!(" AND status BETWEEN ?{} AND ?{}", values.len() - 1, values.len()));
            }
            Some(StatusWanted::Failed) => sql.push_str(" AND status IS NULL"),
            None => {}
        }
        if let Some(before) = asked.before {
            values.push(Box::new(before));
            sql.push_str(&format!(" AND id < ?{}", values.len()));
        }
        values.push(Box::new(i64::try_from(asked.most).unwrap_or(i64::MAX)));
        sql.push_str(&format!(" ORDER BY id DESC LIMIT ?{}", values.len()));
        let mut statement = self.conn.prepare(&sql).map_err(fault(doing))?;
        let rows = statement
            .query_map(params_from_iter(values.iter().map(|value| value.as_ref())), kept)
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))?;
        Ok(rows)
    }

    /// The run `id` of `repository`, if it is kept there. A run of another repository is not found.
    pub fn request_run(&self, repository: &str, id: i64) -> Result<Option<KeptRequestRun>, DatabaseFault> {
        use rusqlite::OptionalExtension;
        self.conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM request_runs WHERE id = ?1 AND repository = ?2"),
                rusqlite::params![id, repository],
                kept,
            )
            .optional()
            .map_err(fault("reading a run of a saved request"))
    }
}
