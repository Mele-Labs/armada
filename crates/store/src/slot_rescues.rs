//! The Finding a rescue Scout brought back from a stranded slot, kept against
//! the slot so it outlives a Bridge reload and a Fleet restart.
//! `docs/concepts/scout.md`, *Starting from a stranded slot*.
//!
//! **One row per slot, replaced as the Scout reads.** The slot is the key, and
//! no Job or Studio owns it: a stranded slot's holder is gone. The row goes
//! when a person scraps or stashes the slot, or when the slot is no longer
//! stranded.

use crate::error::{fault, WriteError};
use crate::open::Store;

/// Version 106 — a stranded slot's rescue Finding.
///
/// **`read` and `searched` are JSON arrays of text**, written whole with each
/// look: they are shown and never queried. **Nothing to backfill.**
pub(crate) const V106: &str = r#"
CREATE TABLE slot_rescues (
    manifest_id TEXT NOT NULL,
    slot        INTEGER NOT NULL,
    state       TEXT NOT NULL CHECK (state IN ('reading', 'answered', 'stopped', 'failed')),
    work_commit TEXT NOT NULL,
    uncommitted INTEGER NOT NULL CHECK (uncommitted IN (0, 1)),
    cut         INTEGER NOT NULL,
    read        TEXT NOT NULL,
    searched    TEXT NOT NULL,
    summary     TEXT,
    why         TEXT,
    cost_micros INTEGER,
    PRIMARY KEY (manifest_id, slot)
) STRICT;
"#;

/// Version 107 — what the Scout concluded: a verdict and its items.
///
/// **`items` is a JSON array of text**, as `read` is. A row kept before this
/// has neither and reads as a Finding with only its summary.
pub(crate) const V107: &str = r#"
ALTER TABLE slot_rescues ADD COLUMN verdict TEXT CHECK (verdict IN ('unfinished', 'scraps'));
ALTER TABLE slot_rescues ADD COLUMN items TEXT NOT NULL DEFAULT '[]';
"#;

/// What a rescue Scout concluded of the work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RescueVerdict {
    Unfinished,
    Scraps,
}

impl RescueVerdict {
    fn as_text(self) -> &'static str {
        match self {
            RescueVerdict::Unfinished => "unfinished",
            RescueVerdict::Scraps => "scraps",
        }
    }

    fn of(text: &str) -> Option<RescueVerdict> {
        Some(match text {
            "unfinished" => RescueVerdict::Unfinished,
            "scraps" => RescueVerdict::Scraps,
            _ => return None,
        })
    }
}

/// Where a rescue Scout is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RescueState {
    Reading,
    Answered,
    Stopped,
    Failed,
}

impl RescueState {
    fn as_text(self) -> &'static str {
        match self {
            RescueState::Reading => "reading",
            RescueState::Answered => "answered",
            RescueState::Stopped => "stopped",
            RescueState::Failed => "failed",
        }
    }

    fn of(text: &str) -> Option<RescueState> {
        Some(match text {
            "reading" => RescueState::Reading,
            "answered" => RescueState::Answered,
            "stopped" => RescueState::Stopped,
            "failed" => RescueState::Failed,
            _ => return None,
        })
    }
}

/// One stranded slot's Finding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptRescue {
    pub manifest_id: String,
    pub slot: u32,
    pub state: RescueState,
    /// The commit the slot was at when the Scout read it.
    pub commit: String,
    /// Whether uncommitted changes were on top of it.
    pub uncommitted: bool,
    /// Characters of the change dropped from the end before the Scout was
    /// handed it. `0` where it got all of it.
    pub cut: u64,
    pub read: Vec<String>,
    pub searched: Vec<String>,
    /// Whether the work has a part left to do, or is leftovers. `None` until
    /// the Scout answers in the shape asked for.
    pub verdict: Option<RescueVerdict>,
    /// What is left to do, or what the leftovers are.
    pub items: Vec<String>,
    /// What the Scout said last, where it was not the shape asked for.
    pub summary: Option<String>,
    /// Why it failed, where it did.
    pub why: Option<String>,
    pub cost_micros: Option<u64>,
}

impl Store {
    /// Keep `rescue`, replacing the slot's last.
    pub fn keep_rescue(&mut self, rescue: &KeptRescue) -> Result<(), WriteError> {
        let list = |items: &[String]| serde_json::to_string(items).unwrap_or_else(|_| "[]".into());
        self.conn
            .execute(
                "INSERT OR REPLACE INTO slot_rescues (manifest_id, slot, state, work_commit, \
                 uncommitted, cut, read, searched, summary, why, cost_micros, verdict, items)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                rusqlite::params![
                    rescue.manifest_id,
                    i64::from(rescue.slot),
                    rescue.state.as_text(),
                    rescue.commit,
                    rescue.uncommitted,
                    rescue.cut as i64,
                    list(&rescue.read),
                    list(&rescue.searched),
                    rescue.summary,
                    rescue.why,
                    rescue.cost_micros.map(|micros| micros as i64),
                    rescue.verdict.map(RescueVerdict::as_text),
                    list(&rescue.items),
                ],
            )
            .map(|_| ())
            .map_err(fault("keeping a rescue finding"))
            .map_err(WriteError::Database)
    }

    /// Every rescue Finding kept, across every repository.
    pub fn rescues(&self) -> Result<Vec<KeptRescue>, WriteError> {
        let doing = "reading the rescue findings";
        let mut statement = self
            .conn
            .prepare(
                "SELECT manifest_id, slot, state, work_commit, uncommitted, cut, read, searched, \
                 summary, why, cost_micros, verdict, items FROM slot_rescues \
                 ORDER BY manifest_id, slot",
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([], |row| {
                let list = |text: String| serde_json::from_str(&text).unwrap_or_default();
                let state: String = row.get(2)?;
                Ok(KeptRescue {
                    manifest_id: row.get(0)?,
                    slot: row.get::<_, i64>(1)? as u32,
                    state: RescueState::of(&state).unwrap_or(RescueState::Failed),
                    commit: row.get(3)?,
                    uncommitted: row.get(4)?,
                    cut: row.get::<_, i64>(5)? as u64,
                    read: list(row.get(6)?),
                    searched: list(row.get(7)?),
                    verdict: row
                        .get::<_, Option<String>>(11)?
                        .and_then(|text| RescueVerdict::of(&text)),
                    items: list(row.get(12)?),
                    summary: row.get(8)?,
                    why: row.get(9)?,
                    cost_micros: row.get::<_, Option<i64>>(10)?.map(|micros| micros as u64),
                })
            })
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>()
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// The slot's Finding gone. Nothing, where it had none.
    pub fn forget_rescue(&mut self, manifest_id: &str, slot: u32) -> Result<(), WriteError> {
        self.conn
            .execute(
                "DELETE FROM slot_rescues WHERE manifest_id = ?1 AND slot = ?2",
                rusqlite::params![manifest_id, i64::from(slot)],
            )
            .map(|_| ())
            .map_err(fault("forgetting a rescue finding"))
            .map_err(WriteError::Database)
    }

    /// Every Finding left reading, ended as failed: no Scout outlives the
    /// Fleet that was reading for it. Run once, at start.
    pub fn rescues_left_reading(&mut self, why: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE slot_rescues SET state = 'failed', why = ?1 WHERE state = 'reading'",
                [why],
            )
            .map(|_| ())
            .map_err(fault("ending the rescue findings left reading"))
            .map_err(WriteError::Database)
    }
}
