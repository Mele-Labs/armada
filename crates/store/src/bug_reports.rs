//! The faults in Armada agents reported (`report_bug`), kept on this Mac for the owner to keep or drop.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One report, as it was made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptBugReport {
    pub id: String,
    pub session_id: String,
    /// The Job's title when it reported; empty where it had none.
    pub job_title: String,
    pub title: String,
    pub what: String,
    pub steps: Vec<String>,
    pub errors: Vec<String>,
    pub logs: Vec<String>,
    pub at: String,
    /// When the owner kept it.
    pub kept: Option<String>,
    /// The pull request that fixed it, once one of its Jobs' merged. Absent while it stands open.
    pub fixed: Option<KeptBugFix>,
}

/// The merge that fixed a report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptBugFix {
    /// When the pull request merged.
    pub at: String,
    pub pull_request: u64,
    pub title: String,
    pub url: String,
}

const COLUMNS: &str = "id, session_id, job_title, title, what, steps, errors, logs, at, kept, fixed_at, fixed_pull_request, fixed_title, fixed_url";

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

/// The lines a column holds. A column that will not read as lines is read as none, not as a fault.
fn lines(raw: String) -> Vec<String> {
    serde_json::from_str(&raw).unwrap_or_default()
}

/// The fix a row names, whole or not at all.
fn fixed(row: &rusqlite::Row<'_>) -> rusqlite::Result<Option<KeptBugFix>> {
    let at: Option<String> = row.get(10)?;
    let pull_request: Option<u64> = row.get(11)?;
    let (title, url): (Option<String>, Option<String>) = (row.get(12)?, row.get(13)?);
    Ok(match (at, pull_request) {
        (Some(at), Some(pull_request)) => Some(KeptBugFix { at, pull_request, title: title.unwrap_or_default(), url: url.unwrap_or_default() }),
        _ => None,
    })
}

fn kept_report(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptBugReport> {
    Ok(KeptBugReport {
        id: row.get(0)?,
        session_id: row.get(1)?,
        job_title: row.get(2)?,
        title: row.get(3)?,
        what: row.get(4)?,
        steps: lines(row.get(5)?),
        errors: lines(row.get(6)?),
        logs: lines(row.get(7)?),
        at: row.get(8)?,
        kept: row.get(9)?,
        fixed: fixed(row)?,
    })
}

impl Store {
    pub fn keep_bug_report(&mut self, report: &KeptBugReport) -> Result<(), WriteError> {
        let lines = |lines: &[String]| serde_json::to_string(lines).unwrap_or_else(|_| String::from("[]"));
        self.conn
            .execute(
                &format!("INSERT INTO bug_reports ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"),
                params![
                    report.id,
                    report.session_id,
                    report.job_title,
                    report.title,
                    report.what,
                    lines(&report.steps),
                    lines(&report.errors),
                    lines(&report.logs),
                    report.at,
                    report.kept,
                    report.fixed.as_ref().map(|fix| &fix.at),
                    report.fixed.as_ref().map(|fix| fix.pull_request),
                    report.fixed.as_ref().map(|fix| &fix.title),
                    report.fixed.as_ref().map(|fix| &fix.url)
                ],
            )
            .map(|_| ())
            .map_err(failed("keeping a bug report"))
    }

    /// Every report, newest first.
    pub fn bug_reports(&self) -> Result<Vec<KeptBugReport>, WriteError> {
        let doing = "reading the bug reports";
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM bug_reports ORDER BY at DESC, id DESC"))
            .map_err(failed(doing))?;
        let rows = statement.query_map([], kept_report).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Mark report `id` kept at `at`. A report already kept keeps its first time. None where `id` names none.
    pub fn mark_bug_report_kept(&mut self, id: &str, at: &str) -> Result<Option<KeptBugReport>, WriteError> {
        let doing = "keeping a bug report as the owner's";
        self.conn
            .execute("UPDATE bug_reports SET kept = ?2 WHERE id = ?1 AND kept IS NULL", params![id, at])
            .map_err(failed(doing))?;
        self.conn
            .query_row(&format!("SELECT {COLUMNS} FROM bug_reports WHERE id = ?1"), [id], kept_report)
            .optional()
            .map_err(failed(doing))
    }

    /// Report `id`, if it is kept.
    pub fn bug_report(&self, id: &str) -> Result<Option<KeptBugReport>, WriteError> {
        self.conn
            .query_row(&format!("SELECT {COLUMNS} FROM bug_reports WHERE id = ?1"), [id], kept_report)
            .optional()
            .map_err(failed("reading a bug report"))
    }

    /// Mark report `id` fixed by `fix`. **Set once**: a report already fixed keeps its first merge. True where this
    /// call fixed it, false where it was fixed already or `id` names none.
    pub fn mark_bug_report_fixed(&mut self, id: &str, fix: &KeptBugFix) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE bug_reports SET fixed_at = ?2, fixed_pull_request = ?3, fixed_title = ?4, fixed_url = ?5
                 WHERE id = ?1 AND fixed_at IS NULL",
                params![id, fix.at, fix.pull_request, fix.title, fix.url],
            )
            .map(|changed| changed > 0)
            .map_err(failed("marking a bug report fixed"))
    }

    /// Delete report `id`. False where it names none.
    pub fn drop_bug_report(&mut self, id: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("DELETE FROM bug_reports WHERE id = ?1", [id])
            .map(|changed| changed > 0)
            .map_err(failed("dropping a bug report"))
    }
}
