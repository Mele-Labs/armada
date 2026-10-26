//! The beads issue each Job worked, kept past the Job's end. `docs/concepts/tracker.md`, *Beads*.
//!
//! **Not the ledger's `issue` row**, which is given back when the Job ends or loses its claim.
//! This is what a merge reads to close the issue of a Job that is gone.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One Job and an issue it worked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptJobIssue {
    pub session_id: String,
    pub manifest_id: String,
    pub issue: String,
    /// The pull request, by address, that merged for it. Empty until one does.
    pub merged_in: String,
    /// The pull request the issue was closed for. Empty until it is.
    pub closed_in: String,
}

fn failing(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

impl Store {
    /// `session_id` worked `issue`. Already kept is left as it was.
    pub fn keep_job_issue(&mut self, session_id: &str, manifest_id: &str, issue: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO job_issues (session_id, manifest_id, issue) VALUES (?1, ?2, ?3)",
                params![session_id, manifest_id, issue],
            )
            .map(|_| ())
            .map_err(failing("keeping the issue a job worked"))
    }

    /// The Job lost its claim: it did not work `issue`.
    pub fn forget_job_issue(&mut self, session_id: &str, manifest_id: &str, issue: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "DELETE FROM job_issues WHERE session_id = ?1 AND manifest_id = ?2 AND issue = ?3",
                params![session_id, manifest_id, issue],
            )
            .map(|_| ())
            .map_err(failing("forgetting the issue a job worked"))
    }

    /// Every issue a Job worked in `manifest_id`.
    pub fn job_issues(&self, manifest_id: &str) -> Result<Vec<KeptJobIssue>, WriteError> {
        let doing = "reading the issues jobs worked";
        let mut statement = self
            .conn
            .prepare("SELECT session_id, issue, merged_in, closed_in FROM job_issues WHERE manifest_id = ?1 ORDER BY rowid")
            .map_err(failing(doing))?;
        let rows = statement
            .query_map(params![manifest_id], |row| {
                Ok(KeptJobIssue {
                    session_id: row.get(0)?,
                    manifest_id: manifest_id.to_string(),
                    issue: row.get(1)?,
                    merged_in: row.get(2)?,
                    closed_in: row.get(3)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(failing(doing))?;
        Ok(rows)
    }

    /// The pull request at `url` merged for this Job's `issue`.
    pub fn job_issue_merged(&mut self, session_id: &str, manifest_id: &str, issue: &str, url: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE job_issues SET merged_in = ?4 WHERE session_id = ?1 AND manifest_id = ?2 AND issue = ?3 AND merged_in <> ?4",
                params![session_id, manifest_id, issue, url],
            )
            .map(|_| ())
            .map_err(failing("keeping the merge of a job's issue"))
    }

    /// The issue was closed for the pull request at `url`.
    pub fn job_issue_closed(&mut self, session_id: &str, manifest_id: &str, issue: &str, url: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE job_issues SET closed_in = ?4 WHERE session_id = ?1 AND manifest_id = ?2 AND issue = ?3",
                params![session_id, manifest_id, issue, url],
            )
            .map(|_| ())
            .map_err(failing("keeping the close of a job's issue"))
    }
}
