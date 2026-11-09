//! The gates a Job's plan names, as Fleet saw each close. `docs/concepts/session/the-plan.md`, *Gates*.
//!
//! **Written once and never worked out again**: a row is the fact Fleet saw, and a fact that drops
//! out of a later reading leaves it standing. A row is the group's for as long as the group keeps its
//! id and its gate, so a plan recorded again keeps what closed; a `plan_approved` row holds for the
//! version approved alone.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// The gate kind that holds for the plan version that asked, and later versions that did not ask again.
pub const PLAN_APPROVED: &str = "plan_approved";

/// The gate kind a walk closes. It can close again: each close is a row of its own, numbered.
pub const WALK_APPROVED: &str = "walk_approved";

/// The gate kind a Fleet-run Check closes.
pub const CHECKS_GREEN: &str = "checks_green";

/// One gate that closed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptGate {
    pub group_id: String,
    /// The gate's word, such as `merged`.
    pub gate: String,
    /// The plan version asked that a `plan_approved` row closed for, and for `walk_approved` which close of
    /// the group it is (0, then 1 when a new walk reopened it). 0 for every other gate.
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
    /// The `version` a row for `group_id`'s `gate` takes now: the version asked for a plan's approval,
    /// the number of closes the group's walk gate already has, and 0 for any other gate.
    pub fn next_version(kept: &[KeptGate], group_id: &str, gate: &str, asked: u32) -> u32 {
        match gate {
            PLAN_APPROVED => asked,
            WALK_APPROVED => kept.iter().filter(|one| one.group_id == group_id && one.gate == WALK_APPROVED).count() as u32,
            _ => 0,
        }
    }

    /// Whether this row closes `gate` on the group `group_id` of plan `version`.
    pub fn closes(&self, group_id: &str, gate: &str, version: u32) -> bool {
        self.group_id == group_id && self.gate == gate && (gate != PLAN_APPROVED || self.version == version)
    }
}

/// The latest gate-check run of a Job whose `checks_green` gate is open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptGateChecks {
    /// The commit Fleet ran at.
    pub commit: String,
    /// `running` or `red`.
    pub state: String,
    /// Tests that failed here and passed at the base.
    pub failed: Vec<String>,
    /// Tests that failed here and at the base on main too.
    pub already_red: Vec<String>,
    pub at: String,
}

/// The agent's `all_done`, held while a gate of its plan stands open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptGateHold {
    /// When the agent first said it. A second call leaves it.
    pub since: String,
    /// The agent's summary of the last call that was held.
    pub summary: Option<String>,
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
        let version = if gate.gate == PLAN_APPROVED || gate.gate == WALK_APPROVED { gate.version } else { 0 };
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

    /// Note that a walk was shown for the plan group at `shown_at`. The newest stands.
    pub fn keep_session_gate_walk(&mut self, session_id: &str, group_id: &str, shown_at: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO session_gate_walks (session_id, group_id, shown_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(session_id, group_id) DO UPDATE SET shown_at = MAX(shown_at, excluded.shown_at)",
                params![session_id, group_id, shown_at],
            )
            .map_err(fault("keeping when a group's walk was shown"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// When a walk was last shown for each plan group of the Job that has had one.
    pub fn session_gate_walks(&self, session_id: &str) -> Result<std::collections::HashMap<String, String>, WriteError> {
        let mut rows = self.conn.prepare("SELECT group_id, shown_at FROM session_gate_walks WHERE session_id = ?1").map_err(reading)?;
        let walks = rows
            .query_map([session_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .map_err(reading)?
            .collect::<Result<_, _>>()
            .map_err(reading)?;
        Ok(walks)
    }

    /// The latest gate-check run of the Job, where it has one.
    pub fn session_gate_checks(&self, session_id: &str) -> Result<Option<KeptGateChecks>, WriteError> {
        let lines = |text: Option<String>| text.map(|text| text.lines().map(str::to_string).collect()).unwrap_or_default();
        self.conn
            .query_row(
                "SELECT commit_sha, state, failed, already_red, at FROM session_gate_checks WHERE session_id = ?1",
                [session_id],
                |row| {
                    Ok(KeptGateChecks {
                        commit: row.get(0)?,
                        state: row.get(1)?,
                        failed: lines(row.get(2)?),
                        already_red: lines(row.get(3)?),
                        at: row.get(4)?,
                    })
                },
            )
            .map(Some)
            .or_else(|why| match why {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(reading(other)),
            })
    }

    /// Keep the Job's latest gate-check run, replacing the one before it.
    pub fn keep_session_gate_checks(&mut self, session_id: &str, run: &KeptGateChecks) -> Result<(), WriteError> {
        let joined = |names: &[String]| (!names.is_empty()).then(|| names.join("\n"));
        self.conn
            .execute(
                "INSERT OR REPLACE INTO session_gate_checks (session_id, commit_sha, state, failed, already_red, at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![session_id, run.commit, run.state, joined(&run.failed), joined(&run.already_red), run.at],
            )
            .map_err(fault("keeping a job's gate check"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Forget the Job's gate-check run: its gate closed.
    pub fn clear_session_gate_checks(&mut self, session_id: &str) -> Result<(), WriteError> {
        self.conn
            .execute("DELETE FROM session_gate_checks WHERE session_id = ?1", [session_id])
            .map_err(fault("clearing a job's gate check"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// The Job's held `all_done`, where it has one.
    pub fn session_gate_hold(&self, session_id: &str) -> Result<Option<KeptGateHold>, WriteError> {
        self.conn
            .query_row("SELECT since, summary FROM session_gate_holds WHERE session_id = ?1", [session_id], |row| {
                Ok(KeptGateHold { since: row.get(0)?, summary: row.get(1)? })
            })
            .map(Some)
            .or_else(|why| match why {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(reading(other)),
            })
    }

    /// Hold the Job's `all_done`. **The first `since` stands** and the summary is replaced.
    pub fn keep_session_gate_hold(&mut self, session_id: &str, since: &str, summary: Option<&str>) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO session_gate_holds (session_id, since, summary) VALUES (?1, ?2, ?3)
                 ON CONFLICT(session_id) DO UPDATE SET summary = excluded.summary",
                params![session_id, since, summary],
            )
            .map_err(fault("keeping a job's held done"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Forget the Job's held `all_done`. True where there was one.
    pub fn clear_session_gate_hold(&mut self, session_id: &str) -> Result<bool, WriteError> {
        let gone = self
            .conn
            .execute("DELETE FROM session_gate_holds WHERE session_id = ?1", [session_id])
            .map_err(fault("clearing a job's held done"))
            .map_err(WriteError::Database)?;
        Ok(gone == 1)
    }
}
