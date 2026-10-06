//! The Checks a Drone asked for mid-step, one row per ask. `docs/concepts/manifest.md`,
//! *A Drone's own run*.
//!
//! **Never a Check row.** `job_step_checks` is what a gate ruled on, and a dry
//! result read as one is a pass nobody measured at the gate. An asked run is
//! its own table, so nothing that reads the gate's rows can count it.
//!
//! **Written when the run starts and updated when it ends**, so a run that
//! never ends is still on record. `fleet` is the Fleet process that began the
//! row: a `running` row begun by any other Fleet is read as `lost`, with
//! nothing swept and nothing to remember to run at start.

use core_model::{DroneId, JobId, StepId, TaskId, Timestamp, Ulid};

use crate::error::{fault, LoadJobError, RowError, WriteError};
use crate::open::Store;
use crate::row::column;

const TABLE: &str = "asked_runs";
const FOREIGN_KEY_VIOLATION: i32 = 787;

/// Version 113 — one row per asked run. Nothing is backfilled: no ask was
/// kept before it.
pub(crate) const V113: &str = r#"
CREATE TABLE asked_runs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    drone       TEXT NOT NULL,
    task        INTEGER CHECK (task IS NULL OR task > 0),
    step_id     TEXT NOT NULL,
    attempt     INTEGER NOT NULL CHECK (attempt > 0),
    fleet       TEXT NOT NULL,
    started_at  TEXT NOT NULL,
    finished_at TEXT,
    state       TEXT NOT NULL CHECK (state IN ('running', 'passed', 'failed', 'stopped', 'lost')),
    checks      TEXT NOT NULL,
    narrowed    INTEGER NOT NULL CHECK (narrowed IN (0, 1)),
    only_check  TEXT,
    logs        TEXT NOT NULL DEFAULT '[]'
) STRICT;
CREATE INDEX asked_runs_by_job ON asked_runs (job_id, id);
"#;

/// Where an asked run stands. `Lost` is never written by a run that ends: it
/// is what the supervisor writes for a task that died, and what a read says of
/// a `running` row whose Fleet is gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AskedState {
    Running,
    Passed,
    Failed,
    Stopped,
    Lost,
}

impl AskedState {
    pub fn as_str(self) -> &'static str {
        match self {
            AskedState::Running => "running",
            AskedState::Passed => "passed",
            AskedState::Failed => "failed",
            AskedState::Stopped => "stopped",
            AskedState::Lost => "lost",
        }
    }

    fn from_column(text: &str) -> Option<AskedState> {
        Some(match text {
            "running" => AskedState::Running,
            "passed" => AskedState::Passed,
            "failed" => AskedState::Failed,
            "stopped" => AskedState::Stopped,
            "lost" => AskedState::Lost,
            _ => return None,
        })
    }
}

/// What is known when the run starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AskedRunBegun {
    /// The Drone that asked.
    pub drone: DroneId,
    /// The task it was put on, where it works one.
    pub task: Option<TaskId>,
    pub step: StepId,
    pub attempt: u32,
    /// The Fleet process starting it.
    pub fleet: Ulid,
    pub at: Timestamp,
    /// The Checks the run is about, in the step's order.
    pub checks: Vec<String>,
    pub narrowed: bool,
    /// The one Check the ask named, where it named one.
    pub only_check: Option<String>,
}

/// One asked run, as read back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AskedRun {
    pub id: i64,
    /// The Drone that asked.
    pub drone: DroneId,
    pub task: Option<TaskId>,
    pub step: StepId,
    pub attempt: u32,
    pub started_at: Timestamp,
    /// Absent while it runs, and on a run nobody saw end.
    pub finished_at: Option<Timestamp>,
    pub state: AskedState,
    pub checks: Vec<String>,
    pub narrowed: bool,
    pub only_check: Option<String>,
    pub logs: Vec<String>,
}

impl Store {
    /// Keep the start of a run, state `running`. Answers the row's id.
    pub fn begin_asked_run(
        &mut self,
        job: &JobId,
        begun: &AskedRunBegun,
    ) -> Result<i64, WriteError> {
        let checks = serde_json::to_string(&begun.checks).unwrap_or_else(|_| "[]".to_string());
        self.conn
            .execute(
                "INSERT INTO asked_runs (job_id, drone, task, step_id, attempt, fleet,
                     started_at, state, checks, narrowed, only_check)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'running', ?8, ?9, ?10)",
                rusqlite::params![
                    job.as_str(),
                    begun.drone.as_str(),
                    begun.task.map(TaskId::number),
                    begun.step.as_str(),
                    begun.attempt,
                    begun.fleet.as_str(),
                    begun.at.as_str(),
                    checks,
                    begun.narrowed,
                    begun.only_check,
                ],
            )
            .map_err(|why| match why {
                rusqlite::Error::SqliteFailure(err, _)
                    if err.extended_code == FOREIGN_KEY_VIOLATION =>
                {
                    WriteError::NoSuchJob {
                        job_id: job.clone(),
                    }
                }
                other => WriteError::Database(fault("keeping the start of an asked run")(other)),
            })?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Keep how a run ended. **Only a row still `running` moves**, so a second
    /// ending, from a supervisor that heard late, cannot overwrite the first.
    pub fn end_asked_run(
        &mut self,
        id: i64,
        state: AskedState,
        at: &Timestamp,
        logs: &[String],
    ) -> Result<(), WriteError> {
        let logs = serde_json::to_string(logs).unwrap_or_else(|_| "[]".to_string());
        self.conn
            .execute(
                "UPDATE asked_runs SET state = ?2, finished_at = ?3, logs = ?4
                 WHERE id = ?1 AND state = 'running'",
                rusqlite::params![id, state.as_str(), at.as_str(), logs],
            )
            .map_err(fault("keeping the end of an asked run"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Every asked run of this Job, oldest first. **`this_fleet` is the Fleet
    /// reading**: a `running` row begun by another reads as `lost`.
    pub fn asked_runs(
        &self,
        job: &JobId,
        this_fleet: &Ulid,
    ) -> Result<Vec<AskedRun>, LoadJobError> {
        let reading = "reading a Job's asked runs";
        let mut asked = self
            .conn
            .prepare(
                "SELECT id, drone, task, step_id, attempt, fleet, started_at, finished_at,
                        state, checks, narrowed, only_check, logs
                 FROM asked_runs WHERE job_id = ?1 ORDER BY id",
            )
            .map_err(fault(reading))
            .map_err(LoadJobError::Database)?;
        let rows = asked
            .query_map((job.as_str(),), |row| Ok(read(row, this_fleet)))
            .map_err(fault(reading))
            .map_err(LoadJobError::Database)?;
        let mut runs = Vec::new();
        for row in rows {
            let row = row
                .map_err(fault(reading))
                .map_err(LoadJobError::Database)?;
            runs.push(row.map_err(LoadJobError::Unreadable)?);
        }
        Ok(runs)
    }
}

fn read(row: &rusqlite::Row<'_>, this_fleet: &Ulid) -> Result<AskedRun, RowError> {
    let text = |name: &'static str| -> Result<String, RowError> {
        row.get(name).map_err(column(TABLE, name))
    };
    let malformed = |name: &'static str, detail: String| RowError::MalformedColumn {
        table: TABLE,
        column: name,
        detail,
    };
    let list = |name: &'static str| -> Result<Vec<String>, RowError> {
        serde_json::from_str(&text(name)?).map_err(|why| malformed(name, why.to_string()))
    };
    let stored = text("state")?;
    let mut state = AskedState::from_column(&stored)
        .ok_or_else(|| malformed("state", format!("`{stored}` is not a state")))?;
    if state == AskedState::Running && text("fleet")? != this_fleet.as_str() {
        state = AskedState::Lost;
    }
    let task: Option<u32> = row.get("task").map_err(column(TABLE, "task"))?;
    let finished: Option<String> = row
        .get("finished_at")
        .map_err(column(TABLE, "finished_at"))?;
    let narrowed: i64 = row.get("narrowed").map_err(column(TABLE, "narrowed"))?;
    Ok(AskedRun {
        id: row.get("id").map_err(column(TABLE, "id"))?,
        drone: DroneId::carried(Ulid::carried(text("drone")?)),
        task: task
            .and_then(std::num::NonZeroU32::new)
            .map(TaskId::numbered),
        step: StepId::new(text("step_id")?),
        attempt: row.get("attempt").map_err(column(TABLE, "attempt"))?,
        started_at: Timestamp::from_rfc3339(text("started_at")?),
        finished_at: finished.map(Timestamp::from_rfc3339),
        state,
        checks: list("checks")?,
        narrowed: narrowed != 0,
        only_check: row.get("only_check").map_err(column(TABLE, "only_check"))?,
        logs: list("logs")?,
    })
}
