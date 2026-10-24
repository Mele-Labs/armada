//! Which bead each group and task of a Job's plan was mirrored as. `docs/concepts/session/the-plan.md`.
//!
//! **Strings, as `session_plan` keeps them**: this crate knows neither the plan's rules nor the
//! backlog's. A row is written the moment its bead exists, before anything syncs, so a sync that
//! fails and is tried again never files the bead twice.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One plan id and the bead it was filed as.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptBead {
    pub session_id: String,
    /// The beads issue the plan was mirrored under.
    pub issue: String,
    /// `g1` for a group, `t3` for a task.
    pub plan_id: String,
    pub bead: String,
    /// The title last written to the bead.
    pub title: String,
    /// `open`, `in_progress` or `closed`: what was last written.
    pub status: String,
    /// The plan id of the group this one was last ordered after. Empty for none.
    pub blocked_by: String,
}

fn failing(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

impl Store {
    /// Every bead `session_id`'s plan was mirrored as under `issue`, in the order they were filed.
    pub fn plan_beads(&self, session_id: &str, issue: &str) -> Result<Vec<KeptBead>, WriteError> {
        let doing = "reading the beads a plan was mirrored as";
        let mut statement = self
            .conn
            .prepare(
                "SELECT plan_id, bead, title, status, blocked_by FROM plan_beads
                 WHERE session_id = ?1 AND issue = ?2 ORDER BY rowid",
            )
            .map_err(failing(doing))?;
        let rows = statement
            .query_map(params![session_id, issue], |row| {
                Ok(KeptBead {
                    session_id: session_id.to_string(),
                    issue: issue.to_string(),
                    plan_id: row.get(0)?,
                    bead: row.get(1)?,
                    title: row.get(2)?,
                    status: row.get(3)?,
                    blocked_by: row.get(4)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(failing(doing))?;
        Ok(rows)
    }

    /// Keep `bead`: a new row, or what was written to an existing one.
    pub fn keep_plan_bead(&mut self, bead: &KeptBead) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO plan_beads (session_id, issue, plan_id, bead, title, status, blocked_by)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT (session_id, issue, plan_id) DO UPDATE
                 SET title = excluded.title, status = excluded.status, blocked_by = excluded.blocked_by",
                params![bead.session_id, bead.issue, bead.plan_id, bead.bead, bead.title, bead.status, bead.blocked_by],
            )
            .map(|_| ())
            .map_err(failing("keeping the bead a plan was mirrored as"))
    }

    /// Forget the row for `plan_id`, once its bead is closed for good.
    pub fn forget_plan_bead(&mut self, session_id: &str, issue: &str, plan_id: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "DELETE FROM plan_beads WHERE session_id = ?1 AND issue = ?2 AND plan_id = ?3",
                params![session_id, issue, plan_id],
            )
            .map(|_| ())
            .map_err(failing("forgetting the bead a plan was mirrored as"))
    }
}
