//! Which Drone was put on which task, and what it handed in. Spike 022,
//! slice 1b.
//!
//! **One row per Drone**, written at the spawn and filled at the hand-in. The
//! spawn is what binds a Drone to its task, so `submit_evidence` names none;
//! `list_job_drones` reads the binding back as `JobDrone.task`, and the step's
//! gate reads every hand-in as the one submission it weighs.
//!
//! **The claim is kept here, not on the plan.** The plan keeps `shown`, which
//! a person reads beside `expects`; what a Drone claimed and left unclaimed is
//! the gate's to read, and it has to survive a Fleet restarting mid-step.

use core_model::{DroneId, JobId, StepId, TaskId, Timestamp, Ulid};

use crate::error::{fault, LoadJobError, RowError, WriteError};
use crate::open::Store;
use crate::row::column;

/// Version 92 — a task's Drone and its hand-in.
///
/// Beside the table it creates, like [`V76`](crate::pending_evidence::V76).
/// Nothing is backfilled: no Drone was put on a task before it.
pub(crate) const V92: &str = r#"
CREATE TABLE job_task_drones (
    job_id       TEXT NOT NULL REFERENCES jobs(job_id),
    drone_id     TEXT NOT NULL,
    step_id      TEXT NOT NULL,
    task_id      INTEGER NOT NULL CHECK (task_id > 0),
    spawned_at   TEXT NOT NULL,
    handed_in_at TEXT,
    claimed      TEXT,
    shown_by     TEXT,
    not_claimed  TEXT,
    PRIMARY KEY (job_id, drone_id),
    CHECK ((handed_in_at IS NULL) = (claimed IS NULL)),
    CHECK ((handed_in_at IS NULL) = (shown_by IS NULL)),
    CHECK ((handed_in_at IS NULL) = (not_claimed IS NULL))
) STRICT;
"#;

/// What a task's Drone handed in: the three fields of a work submission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskHandIn {
    pub claimed: String,
    pub shown_by: String,
    pub not_claimed: String,
    pub at: Timestamp,
}

/// One Drone put on one task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskDrone {
    pub drone_id: DroneId,
    pub step_id: StepId,
    pub task: TaskId,
    pub spawned_at: Timestamp,
    /// `None` until it hands in, and on a Drone that never did.
    pub handed_in: Option<TaskHandIn>,
    /// Whether it ran beside the Job's kept Drone rather than as it. Slice 5.
    pub extra: bool,
    /// The process a Drone beside the kept one ran as, and when it started,
    /// so a Fleet that restarts can end it. `None` on the kept Drone's rows,
    /// whose process is `job_drone_process`.
    pub process: Option<(u32, String)>,
    /// When its edit calls were kept, at its first hand-in. `None` before.
    pub edits_at: Option<Timestamp>,
    /// When a Drone beside the kept one left, and how. `None` on the kept
    /// Drone's rows, whose leaving is on the Job's record.
    pub left: Option<(Timestamp, crate::ExtraEnded)>,
}

impl Store {
    /// Bind a Drone to the task it was spawned for.
    pub fn record_task_drone(
        &mut self,
        job_id: &JobId,
        drone_id: &DroneId,
        step_id: &StepId,
        task: TaskId,
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO job_task_drones (job_id, drone_id, step_id, task_id, spawned_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                (
                    job_id.as_str(),
                    drone_id.as_str(),
                    step_id.as_str(),
                    task.number(),
                    at.as_str(),
                ),
            )
            .map_err(|why| match why {
                rusqlite::Error::SqliteFailure(err, _)
                    if err.extended_code == FOREIGN_KEY_VIOLATION =>
                {
                    WriteError::NoSuchJob {
                        job_id: job_id.clone(),
                    }
                }
                other => WriteError::Database(fault("binding a Drone to its task")(other)),
            })?;
        Ok(())
    }

    /// Keep what a task's Drone handed in. **A second hand-in replaces the
    /// first**: a Drone the step's Checks sent the work back to hands in again.
    pub fn record_task_hand_in(
        &mut self,
        job_id: &JobId,
        drone_id: &DroneId,
        hand_in: &TaskHandIn,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE job_task_drones SET handed_in_at = ?3, claimed = ?4, shown_by = ?5, \
                 not_claimed = ?6 WHERE job_id = ?1 AND drone_id = ?2",
                (
                    job_id.as_str(),
                    drone_id.as_str(),
                    hand_in.at.as_str(),
                    hand_in.claimed.as_str(),
                    hand_in.shown_by.as_str(),
                    hand_in.not_claimed.as_str(),
                ),
            )
            .map_err(fault("keeping a task's hand-in"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Every Drone this Job put on a task, in the order they were spawned.
    pub fn task_drones(&self, job_id: &JobId) -> Result<Vec<TaskDrone>, LoadJobError> {
        let reading = "reading a Job's task Drones";
        let mut asked = self
            .conn
            .prepare(
                "SELECT drone_id, step_id, task_id, spawned_at, handed_in_at, claimed, \
                 shown_by, not_claimed, extra, pid, process_started_at, edits_at, left_at, \
                 ended FROM job_task_drones WHERE job_id = ?1 ORDER BY spawned_at, rowid",
            )
            .map_err(fault(reading))
            .map_err(LoadJobError::Database)?;
        let rows = asked
            .query_map((job_id.as_str(),), |row| Ok(read_task_drone(row)))
            .map_err(fault(reading))
            .map_err(LoadJobError::Database)?;
        let mut drones = Vec::new();
        for row in rows {
            let row = row
                .map_err(fault(reading))
                .map_err(LoadJobError::Database)?;
            drones.push(row.map_err(LoadJobError::Unreadable)?);
        }
        Ok(drones)
    }
}

fn read_task_drone(row: &rusqlite::Row<'_>) -> Result<TaskDrone, RowError> {
    let text = |name: &'static str| -> Result<String, RowError> {
        row.get(name).map_err(column(TABLE, name))
    };
    let maybe = |name: &'static str| -> Result<Option<String>, RowError> {
        row.get(name).map_err(column(TABLE, name))
    };
    let number: u32 = row.get("task_id").map_err(column(TABLE, "task_id"))?;
    let task = std::num::NonZeroU32::new(number)
        .map(TaskId::numbered)
        .ok_or_else(|| RowError::MalformedColumn {
            table: TABLE,
            column: "task_id",
            detail: "a task id is one-based".to_string(),
        })?;
    let handed_in = match maybe("handed_in_at")? {
        None => None,
        Some(at) => Some(TaskHandIn {
            claimed: text("claimed")?,
            shown_by: text("shown_by")?,
            not_claimed: text("not_claimed")?,
            at: Timestamp::from_rfc3339(at),
        }),
    };
    let extra: i64 = row.get("extra").map_err(column(TABLE, "extra"))?;
    let pid: Option<u32> = row.get("pid").map_err(column(TABLE, "pid"))?;
    let process = match (pid, maybe("process_started_at")?) {
        (Some(pid), Some(started)) => Some((pid, started)),
        _ => None,
    };
    let left = match (maybe("left_at")?, maybe("ended")?) {
        (Some(at), Some(ended)) => Some((
            Timestamp::from_rfc3339(at),
            crate::crew::ExtraEnded::from_column(&ended).ok_or_else(|| {
                RowError::MalformedColumn {
                    table: TABLE,
                    column: "ended",
                    detail: format!("`{ended}` is not how a Drone left"),
                }
            })?,
        )),
        _ => None,
    };
    Ok(TaskDrone {
        drone_id: DroneId::carried(Ulid::carried(text("drone_id")?)),
        step_id: StepId::new(text("step_id")?),
        task,
        spawned_at: Timestamp::from_rfc3339(text("spawned_at")?),
        handed_in,
        extra: extra == 1,
        process,
        edits_at: maybe("edits_at")?.map(Timestamp::from_rfc3339),
        left,
    })
}

/// Named once, because every error above points at the same table.
const TABLE: &str = "job_task_drones";

/// `SQLITE_CONSTRAINT_FOREIGNKEY`, `crate::pending_evidence`'s own constant.
const FOREIGN_KEY_VIOLATION: i32 = 787;
