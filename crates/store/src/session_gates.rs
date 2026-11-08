//! The gates a Job's plan names, as Fleet saw each close. `docs/concepts/session/the-plan.md`, *Gates*.
//!
//! **Written once and never worked out again**: a row is the fact Fleet saw, and a fact that drops
//! out of a later reading leaves it standing. A row is the group's for as long as the group keeps its
//! id and its gate, so a plan recorded again keeps what closed; a `plan_approved` row holds for the
//! version approved alone.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// The gate kind that holds for one plan version only.
pub const PLAN_APPROVED: &str = "plan_approved";

/// One gate that closed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptGate {
    pub group_id: String,
    /// The gate's word, such as `merged`.
    pub gate: String,
    /// The plan version a `plan_approved` row closed for. 0 for every other gate.
    pub version: u32,
    pub closed_at: String,
    /// One line, as a person reads it: "You approved plan v2".
    pub fact: String,
    /// What closed it, such as `pr:3360`: a fact closes one gate of its kind and no other.
    pub source: String,
    /// The commit it closed at, where there is one.
    pub commit: Option<String>,
    /// Tests that failed here and at the base on main too, so they did not hold it.
    pub already_red: Vec<String>,
    /// The person skipped the group rather than the fact arriving.
    pub skipped: bool,
}

impl KeptGate {
    /// Whether this row closes `gate` on the group `group_id` of plan `version`.
    pub fn closes(&self, group_id: &str, gate: &str, version: u32) -> bool {
        self.group_id == group_id && self.gate == gate && (gate != PLAN_APPROVED || self.version == version)
    }
}

fn reading(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("reading a job's gates")(why))
}

impl Store {
    /// Every gate of the Job that closed, oldest first.
    pub fn session_gates(&self, session_id: &str) -> Result<Vec<KeptGate>, WriteError> {
        let mut rows = self
            .conn
            .prepare(
                "SELECT group_id, gate, version, closed_at, fact, source, commit_sha, already_red, skipped
                 FROM session_gates WHERE session_id = ?1 ORDER BY closed_at, rowid",
            )
            .map_err(reading)?;
        let gates = rows
            .query_map([session_id], |row| {
                let red: Option<String> = row.get(7)?;
                Ok(KeptGate {
                    group_id: row.get(0)?,
                    gate: row.get(1)?,
                    version: row.get::<_, i64>(2)?.max(0) as u32,
                    closed_at: row.get(3)?,
                    fact: row.get(4)?,
                    source: row.get(5)?,
                    commit: row.get(6)?,
                    already_red: red.map(|red| red.lines().map(str::to_string).collect()).unwrap_or_default(),
                    skipped: row.get::<_, i64>(8)? == 1,
                })
            })
            .map_err(reading)?
            .collect::<Result<_, _>>()
            .map_err(reading)?;
        Ok(gates)
    }

    /// Keep one gate's close. **False where that gate had closed already**, so a close is kept once and
    /// the first fact stands.
    pub fn close_session_gate(&mut self, session_id: &str, gate: &KeptGate) -> Result<bool, WriteError> {
        let version = if gate.gate == PLAN_APPROVED { gate.version } else { 0 };
        let red = (!gate.already_red.is_empty()).then(|| gate.already_red.join("\n"));
        let wrote = self
            .conn
            .execute(
                "INSERT OR IGNORE INTO session_gates
                     (session_id, group_id, gate, version, closed_at, fact, source, commit_sha, already_red, skipped)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    session_id,
                    gate.group_id,
                    gate.gate,
                    version,
                    gate.closed_at,
                    gate.fact,
                    gate.source,
                    gate.commit,
                    red,
                    gate.skipped
                ],
            )
            .map_err(fault("keeping a job's gate"))
            .map_err(WriteError::Database)?;
        Ok(wrote == 1)
    }
}
