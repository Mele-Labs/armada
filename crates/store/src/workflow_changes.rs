//! The changes to a workflow an agent drafted for the owner to approve, kept so the Workflows page can
//! show the card and the drafting state. `docs/concepts/workflow.md`.
//!
//! **A change is open until it is settled**: drafting, ready or failed. Approving or dismissing one keeps
//! the row and takes it off the page. `source` is where the definition came from, since the
//! repository's copy and Kit's of one id are two workflows.

use rusqlite::{params, OptionalExtension, Row};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// Where a change stands, as the store spells it.
pub const DRAFTING: &str = "drafting";
pub const READY: &str = "ready";
pub const APPROVED: &str = "approved";
pub const DISMISSED: &str = "dismissed";
pub const FAILED: &str = "failed";

/// One change, as the store keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptWorkflowChange {
    pub id: i64,
    pub manifest_id: String,
    pub workflow_id: String,
    pub source: String,
    /// The one step a suggestion may change. Absent on a change the owner asked for.
    pub step_id: Option<String>,
    /// The owner's words. Absent on a suggestion.
    pub asked: Option<String>,
    /// The nudges a suggestion was drawn from.
    pub nudges: Vec<i64>,
    pub before_text: String,
    pub after_text: Option<String>,
    pub state: String,
    pub failed_because: Option<String>,
    /// What the owner said when he sent it back, the latest.
    pub notes: Option<String>,
    pub created_at: String,
    pub changed_at: String,
}

/// What a change is begun with.
pub struct NewWorkflowChange<'a> {
    pub manifest_id: &'a str,
    pub workflow_id: &'a str,
    pub source: &'a str,
    pub step_id: Option<&'a str>,
    pub asked: Option<&'a str>,
    pub nudges: &'a [i64],
    pub before_text: &'a str,
}

const COLUMNS: &str = "id, manifest_id, workflow_id, source, step_id, asked, nudges, before_text, after_text, state, \
                       failed_because, notes, created_at, changed_at";

fn read(row: &Row<'_>) -> rusqlite::Result<KeptWorkflowChange> {
    let nudges: String = row.get(6)?;
    Ok(KeptWorkflowChange {
        id: row.get(0)?,
        manifest_id: row.get(1)?,
        workflow_id: row.get(2)?,
        source: row.get(3)?,
        step_id: row.get(4)?,
        asked: row.get(5)?,
        nudges: nudges.split(',').filter_map(|one| one.parse().ok()).collect(),
        before_text: row.get(7)?,
        after_text: row.get(8)?,
        state: row.get(9)?,
        failed_because: row.get(10)?,
        notes: row.get(11)?,
        created_at: row.get(12)?,
        changed_at: row.get(13)?,
    })
}

impl Store {
    /// Begin a change, drafting, at `at`. Its id.
    pub fn begin_workflow_change(&mut self, new: &NewWorkflowChange<'_>, at: &str) -> Result<i64, WriteError> {
        let nudges = new.nudges.iter().map(i64::to_string).collect::<Vec<_>>().join(",");
        self.conn
            .execute(
                "INSERT INTO workflow_changes (manifest_id, workflow_id, source, step_id, asked, nudges, before_text, state, created_at, changed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
                params![new.manifest_id, new.workflow_id, new.source, new.step_id, new.asked, nudges, new.before_text, DRAFTING, at],
            )
            .map_err(fault("beginning a workflow change"))
            .map_err(WriteError::Database)?;
        Ok(self.conn.last_insert_rowid())
    }

    /// One change by id, settled ones included.
    pub fn workflow_change(&self, id: i64) -> Result<Option<KeptWorkflowChange>, WriteError> {
        self.conn
            .query_row(&format!("SELECT {COLUMNS} FROM workflow_changes WHERE id = ?1"), [id], read)
            .optional()
            .map_err(fault("reading a workflow change"))
            .map_err(WriteError::Database)
    }

    /// The open changes to a workflow in a place, oldest first. **A repository's copy is one repository's**,
    /// so `manifest_id` narrows only that place; Kit's is every repository's.
    pub fn open_workflow_changes(
        &self,
        workflow_id: &str,
        source: &str,
        manifest_id: &str,
    ) -> Result<Vec<KeptWorkflowChange>, WriteError> {
        let doing = "reading a workflow's open changes";
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT {COLUMNS} FROM workflow_changes
                 WHERE workflow_id = ?1 AND source = ?2 AND state IN (?3, ?4, ?5) AND (source <> 'repository' OR manifest_id = ?6)
                 ORDER BY id"
            ))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map(params![workflow_id, source, DRAFTING, READY, FAILED, manifest_id], read)
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        Ok(rows)
    }

    /// Move a change from one of the states `from` to `to` at `at`, with what the move carries. Whether
    /// it moved: a change that stands elsewhere is left alone.
    fn move_workflow_change(
        &mut self,
        id: i64,
        from: &[&str],
        to: &str,
        after: Option<&str>,
        because: Option<&str>,
        at: &str,
    ) -> Result<bool, WriteError> {
        let held = self.workflow_change(id)?;
        if !held.is_some_and(|one| from.contains(&one.state.as_str())) {
            return Ok(false);
        }
        self.conn
            .execute(
                "UPDATE workflow_changes SET state = ?2, after_text = ?3, failed_because = ?4, changed_at = ?5 WHERE id = ?1",
                params![id, to, after, because, at],
            )
            .map_err(fault("moving a workflow change"))
            .map_err(WriteError::Database)?;
        Ok(true)
    }

    /// A draft came back: the change is ready with `after`.
    pub fn workflow_change_drafted(&mut self, id: i64, after: &str, at: &str) -> Result<bool, WriteError> {
        self.move_workflow_change(id, &[DRAFTING], READY, Some(after), None, at)
    }

    /// No draft fit: the change failed, and says why.
    pub fn workflow_change_failed(&mut self, id: i64, because: &str, at: &str) -> Result<bool, WriteError> {
        self.move_workflow_change(id, &[DRAFTING], FAILED, None, Some(because), at)
    }

    /// The owner sent a ready or failed change back with `notes`: it is drafting again, keeping the draft
    /// it is sent back from until the new one comes.
    pub fn workflow_change_sent_back(&mut self, id: i64, notes: &str, at: &str) -> Result<bool, WriteError> {
        let held = self.workflow_change(id)?;
        let Some(held) = held.filter(|one| one.state == READY || one.state == FAILED) else {
            return Ok(false);
        };
        self.conn
            .execute(
                "UPDATE workflow_changes SET state = ?2, notes = ?3, failed_because = NULL, changed_at = ?4,
                 after_text = ?5 WHERE id = ?1",
                params![id, DRAFTING, notes, at, held.after_text],
            )
            .map_err(fault("sending a workflow change back"))
            .map_err(WriteError::Database)?;
        Ok(true)
    }

    /// The owner approved a ready change.
    pub fn workflow_change_approved(&mut self, id: i64, at: &str) -> Result<bool, WriteError> {
        let after = self.workflow_change(id)?.and_then(|one| one.after_text);
        self.move_workflow_change(id, &[READY], APPROVED, after.as_deref(), None, at)
    }

    /// The owner dismissed an open change, whatever it stood at.
    pub fn workflow_change_dismissed(&mut self, id: i64, at: &str) -> Result<bool, WriteError> {
        let held = self.workflow_change(id)?;
        let (after, because) = held.map(|one| (one.after_text, one.failed_because)).unwrap_or_default();
        self.move_workflow_change(id, &[DRAFTING, READY, FAILED], DISMISSED, after.as_deref(), because.as_deref(), at)
    }
}
