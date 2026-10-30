//! What a Job changed in its own copy: its workflow once a step was changed, and the marks the lanes
//! draw for each change. The lifecycles and rules of the copy are `job_copies`'.
//!
//! **A Job that changed no step has no workflow row**, and reads the workflow as the repository holds
//! it. The text is the whole definition, as the loader reads it.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// The workflow a Job holds its own copy of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptWorkflowCopy {
    pub workflow_id: String,
    /// Where the definition was read from, for a sentence that names the file.
    pub path: String,
    pub text: String,
}

/// One change the lanes mark. `lane` is `workflow`, a lifecycle's id or `rule`; `item` is the step,
/// the state or the rule, and empty for a lifecycle as a whole. `by` is `job` or `you`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptCopyMark {
    pub lane: String,
    pub item: String,
    pub text: String,
    pub by: String,
    pub at: String,
}

fn failing(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

impl Store {
    /// The Job's own copy of its workflow. `None` for a Job that changed none of its steps.
    pub fn job_workflow_copy(&self, session_id: &str) -> Result<Option<KeptWorkflowCopy>, WriteError> {
        self.conn
            .query_row(
                "SELECT workflow_id, path, text FROM job_copy_workflows WHERE session_id = ?1",
                params![session_id],
                |row| Ok(KeptWorkflowCopy { workflow_id: row.get(0)?, path: row.get(1)?, text: row.get(2)? }),
            )
            .optional()
            .map_err(failing("reading the copy of its workflow a job holds"))
    }

    /// Keep the Job's copy of its workflow, replacing the one it held.
    pub fn keep_job_workflow_copy(&mut self, session_id: &str, copy: &KeptWorkflowCopy) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO job_copy_workflows (session_id, workflow_id, path, text) VALUES (?1, ?2, ?3, ?4)",
                params![session_id, copy.workflow_id, copy.path, copy.text],
            )
            .map(drop)
            .map_err(failing("keeping the copy of its workflow a job holds"))
    }

    /// Give the Job's copy of its workflow up, with the marks on its steps: it reads the repository's again.
    pub fn drop_job_workflow_copy(&mut self, session_id: &str) -> Result<(), WriteError> {
        let doing = "dropping the copy of its workflow a job holds";
        let tx = self.conn.transaction().map_err(failing(doing))?;
        tx.execute("DELETE FROM job_copy_workflows WHERE session_id = ?1", params![session_id]).map_err(failing(doing))?;
        tx.execute("DELETE FROM job_copy_marks WHERE session_id = ?1 AND lane = 'workflow'", params![session_id]).map_err(failing(doing))?;
        tx.commit().map_err(failing(doing))
    }

    /// What the lanes mark on the Job, oldest change first.
    pub fn job_copy_marks(&self, session_id: &str) -> Result<Vec<KeptCopyMark>, WriteError> {
        let doing = "reading what a job changed in its copy";
        self.conn
            .prepare("SELECT lane, item, text, by, at FROM job_copy_marks WHERE session_id = ?1 ORDER BY at, rowid")
            .map_err(failing(doing))?
            .query_map(params![session_id], |row| {
                Ok(KeptCopyMark { lane: row.get(0)?, item: row.get(1)?, text: row.get(2)?, by: row.get(3)?, at: row.get(4)? })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(failing(doing))
    }

    /// Mark a node or a rule as changed on this Job. The latest change to a node is the one kept.
    pub fn keep_job_copy_mark(&mut self, session_id: &str, mark: &KeptCopyMark) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO job_copy_marks (session_id, lane, item, text, by, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![session_id, mark.lane, mark.item, mark.text, mark.by, mark.at],
            )
            .map(drop)
            .map_err(failing("marking what a job changed in its copy"))
    }

    /// Take a mark away, where the change was undone or the node is gone.
    pub fn drop_job_copy_mark(&mut self, session_id: &str, lane: &str, item: &str) -> Result<(), WriteError> {
        self.conn
            .execute("DELETE FROM job_copy_marks WHERE session_id = ?1 AND lane = ?2 AND item = ?3", params![session_id, lane, item])
            .map(drop)
            .map_err(failing("taking a mark off what a job changed in its copy"))
    }
}
