//! The nudges Fleet sent about a workflow's steps, kept against the step so its page can list the
//! Jobs that drifted there. `docs/concepts/workflow.md`.
//!
//! **A nudge is a Job's, once for each step and fact**; marking one not a drift keeps the row and
//! stops it counting. `source` is where the definition came from, since the repository's copy and
//! Armada's of one id are two workflows.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One nudge, as the store keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptWorkflowNudge {
    pub id: i64,
    pub workflow_id: String,
    pub source: String,
    pub step_id: String,
    pub session_id: String,
    /// What happened, as a short fact.
    pub fact: String,
    pub at: String,
    /// When it was marked not a drift. Absent while it counts.
    pub dismissed_at: Option<String>,
}

impl KeptWorkflowNudge {
    /// Whether it counts as a drift: it has not been marked otherwise.
    pub fn counts(&self) -> bool {
        self.dismissed_at.is_none()
    }
}

const COLUMNS: &str = "id, workflow_id, source, step_id, session_id, fact, at, dismissed_at";

impl Store {
    /// Keep that `session_id` was nudged about `step_id` of the workflow, at `at`. Whether it was new:
    /// a Job drifts from a step in one way once.
    pub fn keep_workflow_nudge(
        &mut self,
        workflow_id: &str,
        source: &str,
        step_id: &str,
        session_id: &str,
        fact: &str,
        at: &str,
    ) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO workflow_nudges (workflow_id, source, step_id, session_id, fact, at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![workflow_id, source, step_id, session_id, fact, at],
            )
            .map(|kept| kept > 0)
            .map_err(fault("keeping a workflow nudge"))
            .map_err(WriteError::Database)
    }

    /// The newest `limit` nudges of a step, newest first, marked ones included.
    pub fn workflow_nudges(
        &self,
        workflow_id: &str,
        source: &str,
        step_id: &str,
        limit: u32,
    ) -> Result<Vec<KeptWorkflowNudge>, WriteError> {
        let doing = "reading a workflow step's nudges";
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM workflow_nudges WHERE workflow_id = ?1 AND source = ?2 AND step_id = ?3
                 ORDER BY at DESC, id DESC LIMIT ?4"
            ))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map(params![workflow_id, source, step_id, limit], |row| {
                Ok(KeptWorkflowNudge {
                    id: row.get(0)?,
                    workflow_id: row.get(1)?,
                    source: row.get(2)?,
                    step_id: row.get(3)?,
                    session_id: row.get(4)?,
                    fact: row.get(5)?,
                    at: row.get(6)?,
                    dismissed_at: row.get(7)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        Ok(rows)
    }

    /// Mark a nudge not a drift, at `at`, so it stops counting. Whether there was one to mark: a mark
    /// already made keeps its first time.
    pub fn dismiss_workflow_nudge(&mut self, id: i64, at: &str) -> Result<bool, WriteError> {
        let doing = "marking a nudge not a drift";
        let held: Option<Option<String>> = self
            .conn
            .query_row("SELECT dismissed_at FROM workflow_nudges WHERE id = ?1", [id], |row| row.get(0))
            .optional()
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        if held.is_none() {
            return Ok(false);
        }
        self.conn
            .execute("UPDATE workflow_nudges SET dismissed_at = ?2 WHERE id = ?1 AND dismissed_at IS NULL", params![id, at])
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        Ok(true)
    }
}
