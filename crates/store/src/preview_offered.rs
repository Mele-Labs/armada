//! The branches Jobs offer for the owner's preview: one row a (Job, branch), the check's verdict on it,
//! and the Refresh row the owner skipped. `docs/concepts/fleet.md`, *Offering a branch for the preview*.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One offer, as kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptPreviewOffered {
    pub session_id: String,
    pub branch: String,
    /// Where the branch stood when the Job offered it.
    pub tip: String,
    pub about: String,
    pub offered_at: String,
    /// `checking`, `ready` or `failed`.
    pub state: String,
    pub why: Option<String>,
    pub files: Vec<String>,
    pub checked_at: Option<String>,
    /// The `<tip> <state>` the Job was last told, so a change of state is told once.
    pub told: Option<String>,
    /// One line on what the owner should look at.
    pub look: Option<String>,
    /// How to get there, in order.
    pub steps: Vec<KeptStep>,
}

/// One step of a note: what the owner does, and the act that takes him there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptStep {
    pub say: String,
    pub act: Option<String>,
}

/// `steps` as the one JSON column keeps them. Written by hand: the store holds no serde derives.
pub(crate) fn steps_text(steps: &[KeptStep]) -> String {
    let rows: Vec<serde_json::Value> = steps
        .iter()
        .map(|step| match &step.act {
            Some(act) => serde_json::json!({ "say": step.say, "act": act }),
            None => serde_json::json!({ "say": step.say }),
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// The steps a column holds. One that does not read is left out.
pub(crate) fn steps_read(text: &str) -> Vec<KeptStep> {
    let Ok(serde_json::Value::Array(rows)) = serde_json::from_str(text) else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|row| {
            let say = row.get("say")?.as_str()?.to_string();
            Some(KeptStep { say, act: row.get("act").and_then(|act| act.as_str()).map(str::to_string) })
        })
        .collect()
}

const COLUMNS: &str = "session_id, branch, tip, about, offered_at, state, why, files, checked_at, told, look, steps";

fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptPreviewOffered> {
    let files: String = row.get(7)?;
    Ok(KeptPreviewOffered {
        session_id: row.get(0)?,
        branch: row.get(1)?,
        tip: row.get(2)?,
        about: row.get(3)?,
        offered_at: row.get(4)?,
        state: row.get(5)?,
        why: row.get(6)?,
        files: serde_json::from_str(&files).unwrap_or_default(),
        checked_at: row.get(8)?,
        told: row.get(9)?,
        look: row.get(10)?,
        steps: steps_read(&row.get::<_, String>(11)?),
    })
}

impl Store {
    /// Keep an offer, replacing the Job's offer of that branch: at the new tip, checking, and told nothing
    /// yet. **The Job's earlier `told` is kept**, so a re-offer that ends where it began is not told twice.
    /// **The note of this (branch, tip) is kept with it**, in one step: a re-offer at the same tip updates the
    /// note and leaves it looked at, a new tip is a new note. The note is not taken away with the offer.
    #[allow(clippy::too_many_arguments)]
    pub fn keep_preview_offered(
        &mut self,
        session_id: &str,
        branch: &str,
        tip: &str,
        about: &str,
        look: Option<&str>,
        steps: &[KeptStep],
        at: &str,
    ) -> Result<(), WriteError> {
        let doing = "keeping an offer for the preview";
        let steps = steps_text(steps);
        let transaction = self.conn.transaction().map_err(fault(doing)).map_err(WriteError::Database)?;
        transaction
            .execute(
                "INSERT INTO preview_offered (session_id, branch, tip, about, offered_at, state, look, steps) VALUES (?1, ?2, ?3, ?4, ?5, 'checking', ?6, ?7) \
                 ON CONFLICT (session_id, branch) DO UPDATE SET tip = ?3, about = ?4, offered_at = ?5, state = 'checking', why = NULL, files = '[]', checked_at = NULL, look = ?6, steps = ?7",
                params![session_id, branch, tip, about, at, look, steps],
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        transaction
            .execute(
                "INSERT INTO build_notes (branch, tip, session_id, about, look, steps, offered_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
                 ON CONFLICT (branch, tip) DO UPDATE SET session_id = ?3, about = ?4, look = ?5, steps = ?6, offered_at = ?7",
                params![branch, tip, session_id, about, look, steps, at],
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        // A new offer takes the branch back onto the preview, so what was withdrawn of it is forgotten.
        transaction.execute("DELETE FROM preview_withdrawn WHERE branch = ?1", params![branch]).map_err(fault(doing)).map_err(WriteError::Database)?;
        transaction.commit().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// Keep `branch` at `tip` withdrawn by its Job, so a refresh leaves that tip out though the running build holds it.
    pub fn keep_preview_withdrawn(&mut self, branch: &str, tip: &str, at: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO preview_withdrawn (branch, tip, withdrawn_at) VALUES (?1, ?2, ?3) ON CONFLICT (branch, tip) DO UPDATE SET withdrawn_at = ?3",
                params![branch, tip, at],
            )
            .map_err(fault("keeping a branch withdrawn from the preview"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Every (branch, tip) a Job withdrew and has not offered again.
    pub fn preview_withdrawn_all(&self) -> Result<Vec<(String, String)>, WriteError> {
        let doing = "reading the branches withdrawn from the preview";
        let mut statement = self.conn.prepare("SELECT branch, tip FROM preview_withdrawn ORDER BY branch, tip").map_err(fault(doing)).map_err(WriteError::Database)?;
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).map_err(fault(doing)).map_err(WriteError::Database)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// Every offer, oldest first.
    pub fn preview_offered_all(&self) -> Result<Vec<KeptPreviewOffered>, WriteError> {
        let doing = "reading the offers for the preview";
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM preview_offered ORDER BY offered_at, session_id, branch"))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        statement
            .query_map([], read)
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// The branches the Job has on the preview list.
    pub fn preview_offered_branches_of(&self, session_id: &str) -> Result<Vec<String>, WriteError> {
        let doing = "reading a job's offers for the preview";
        let mut statement = self
            .conn
            .prepare("SELECT branch FROM preview_offered WHERE session_id = ?1 ORDER BY offered_at, branch")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        statement
            .query_map(params![session_id], |row| row.get(0))
            .and_then(|rows| rows.collect::<Result<Vec<String>, _>>())
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// The Job's offer of `branch`.
    pub fn preview_offered_of(&self, session_id: &str, branch: &str) -> Result<Option<KeptPreviewOffered>, WriteError> {
        self.conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM preview_offered WHERE session_id = ?1 AND branch = ?2"),
                params![session_id, branch],
                read,
            )
            .optional()
            .map_err(fault("reading an offer for the preview"))
            .map_err(WriteError::Database)
    }

    /// Take the Job's offer of `branch` away. Whether there was one.
    pub fn remove_preview_offered(&mut self, session_id: &str, branch: &str) -> Result<bool, WriteError> {
        let removed = self
            .conn
            .execute("DELETE FROM preview_offered WHERE session_id = ?1 AND branch = ?2", params![session_id, branch])
            .map_err(fault("taking an offer off the preview list"))
            .map_err(WriteError::Database)?;
        Ok(removed > 0)
    }

    /// Record what the check found on the offer **at `tip`**. A re-offer at another tip while it ran is
    /// left as it is, so it is checked again. Whether the row was written.
    pub fn keep_preview_checked(
        &mut self,
        session_id: &str,
        branch: &str,
        tip: &str,
        state: &str,
        why: Option<&str>,
        files: &[String],
        at: &str,
    ) -> Result<bool, WriteError> {
        let files = serde_json::to_string(files).unwrap_or_else(|_| String::from("[]"));
        let written = self
            .conn
            .execute(
                "UPDATE preview_offered SET state = ?4, why = ?5, files = ?6, checked_at = ?7 WHERE session_id = ?1 AND branch = ?2 AND tip = ?3",
                params![session_id, branch, tip, state, why, files, at],
            )
            .map_err(fault("keeping what the preview check found"))
            .map_err(WriteError::Database)?;
        Ok(written > 0)
    }

    /// Record the `<tip> <state>` the Job was told for its offer of `branch`.
    pub fn keep_preview_told(&mut self, session_id: &str, branch: &str, told: &str) -> Result<(), WriteError> {
        self.conn
            .execute("UPDATE preview_offered SET told = ?3 WHERE session_id = ?1 AND branch = ?2", params![session_id, branch, told])
            .map_err(fault("keeping what a Job was told of its offer"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Keep `id` as the Refresh row skipped. **Only the newest is kept**: a different set of ready
    /// offers has a different id, and the one skipped before it no longer matters.
    pub fn keep_preview_refresh_skipped(&mut self, id: &str, at: &str) -> Result<(), WriteError> {
        let doing = "keeping a skipped refresh of the preview";
        let transaction = self.conn.transaction().map_err(fault(doing)).map_err(WriteError::Database)?;
        transaction.execute("DELETE FROM preview_refresh_skipped", []).map_err(fault(doing)).map_err(WriteError::Database)?;
        transaction
            .execute("INSERT INTO preview_refresh_skipped (id, skipped_at) VALUES (?1, ?2)", params![id, at])
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        transaction.commit().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// The Refresh row's id the owner skipped, if any.
    pub fn preview_refresh_skipped(&self) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row("SELECT id FROM preview_refresh_skipped", [], |row| row.get(0))
            .optional()
            .map_err(fault("reading the skipped refresh of the preview"))
            .map_err(WriteError::Database)
    }
}
