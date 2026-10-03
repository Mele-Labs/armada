//! A Job's Drones beside the one it keeps, and the files each task's Drone
//! edited. Spike 022, slice 5.
//!
//! **A Drone beside the kept one is not on the Job's record of moves.** The
//! step's `assigned_drone` is one pointer and the fold refuses a second spawn
//! onto a step that holds one, so a Drone running a task at the same time as
//! the kept Drone is written here instead: its process, so a Fleet that
//! restarts can end what it left running, and how it left, so
//! `list_job_drones` can say. Its task binding is the `job_task_drones` row
//! every task's Drone already has.
//!
//! **The edit calls are kept once, at a Drone's first hand-in**, with the
//! instant they were read. Which tasks ran at the same time is read off that
//! instant and the spawn, and a group sent round by its Checks is one Drone
//! working on, whose later edits are a round's and not a task's.

use std::num::NonZeroU32;

use core_model::{Apart, DroneId, GroupId, GroupMove, JobId, TaskId, Timestamp};

use crate::error::{fault, LoadJobError, RowError, WriteError};
use crate::open::Store;
use crate::row::{maybe_number, string};

/// Version 99 — Drones beside the kept one, each task's edit calls, the pairs
/// run apart, and the tasks a planner said may run together.
///
/// One version, because each is half of running tasks at once. Nothing is
/// backfilled: no Job ran a Drone beside another before it.
pub(crate) const V99: &str = r#"
ALTER TABLE job_work_plan_tasks ADD COLUMN beside TEXT NOT NULL DEFAULT '';

ALTER TABLE job_task_drones ADD COLUMN extra INTEGER NOT NULL DEFAULT 0
    CHECK (extra IN (0, 1));
ALTER TABLE job_task_drones ADD COLUMN pid INTEGER CHECK (pid IS NULL OR pid > 0);
ALTER TABLE job_task_drones ADD COLUMN process_started_at TEXT;
ALTER TABLE job_task_drones ADD COLUMN edits_at TEXT;
ALTER TABLE job_task_drones ADD COLUMN left_at TEXT;
ALTER TABLE job_task_drones ADD COLUMN ended TEXT
    CHECK (ended IS NULL OR ended IN ('done', 'killed', 'failed'));

CREATE TABLE job_task_edits (
    job_id   TEXT NOT NULL REFERENCES jobs(job_id),
    drone_id TEXT NOT NULL,
    path     TEXT NOT NULL CHECK (trim(path) <> ''),
    PRIMARY KEY (job_id, drone_id, path)
) STRICT;

CREATE TABLE job_group_apart (
    job_id TEXT NOT NULL REFERENCES jobs(job_id),
    grp    INTEGER NOT NULL CHECK (grp > 0),
    first  INTEGER NOT NULL CHECK (first > 0),
    second INTEGER NOT NULL CHECK (second > first),
    paths  TEXT NOT NULL,
    at     TEXT NOT NULL,
    PRIMARY KEY (job_id, grp, first, second)
) STRICT;
"#;

/// How a Drone beside the kept one left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtraEnded {
    /// It handed its task in.
    Done,
    /// A person stopped it.
    Killed,
    /// It left without handing in, or Fleet ended it with nothing handed in.
    Failed,
}

impl ExtraEnded {
    fn as_column(self) -> &'static str {
        match self {
            ExtraEnded::Done => "done",
            ExtraEnded::Killed => "killed",
            ExtraEnded::Failed => "failed",
        }
    }

    pub(crate) fn from_column(text: &str) -> Option<ExtraEnded> {
        match text {
            "done" => Some(ExtraEnded::Done),
            "killed" => Some(ExtraEnded::Killed),
            "failed" => Some(ExtraEnded::Failed),
            _ => None,
        }
    }
}

/// One file a task's Drone named in an edit call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskEdit {
    pub drone_id: DroneId,
    /// Repository-relative.
    pub path: String,
}

impl Store {
    /// Mark a task's Drone as one running beside the kept Drone, as this
    /// process. **After the binding**: the row is the one
    /// [`record_task_drone`](Store::record_task_drone) wrote.
    pub fn record_extra_drone(
        &mut self,
        job_id: &JobId,
        drone_id: &DroneId,
        pid: Option<(u32, String)>,
    ) -> Result<(), WriteError> {
        let (pid, started) = match pid {
            Some((pid, started)) => (Some(pid), Some(started)),
            None => (None, None),
        };
        self.conn
            .execute(
                "UPDATE job_task_drones SET extra = 1, pid = ?3, process_started_at = ?4 \
                 WHERE job_id = ?1 AND drone_id = ?2",
                rusqlite::params![job_id.as_str(), drone_id.as_str(), pid, started],
            )
            .map_err(fault("marking a Drone beside the kept one"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// A Drone beside the kept one has gone, and how.
    pub fn record_extra_left(
        &mut self,
        job_id: &JobId,
        drone_id: &DroneId,
        ended: ExtraEnded,
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE job_task_drones SET left_at = ?3, ended = ?4 \
                 WHERE job_id = ?1 AND drone_id = ?2 AND left_at IS NULL",
                (
                    job_id.as_str(),
                    drone_id.as_str(),
                    at.as_str(),
                    ended.as_column(),
                ),
            )
            .map_err(fault("recording a Drone beside the kept one leaving"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Keep the files a task's Drone named in its edit calls, **once**: a
    /// second hand-in from the same Drone keeps nothing, for the module's
    /// reason. Answers whether this call kept them.
    pub fn keep_task_edits(
        &mut self,
        job_id: &JobId,
        drone_id: &DroneId,
        paths: &[String],
        at: &Timestamp,
    ) -> Result<bool, WriteError> {
        let kept = self
            .conn
            .execute(
                "UPDATE job_task_drones SET edits_at = ?3 \
                 WHERE job_id = ?1 AND drone_id = ?2 AND edits_at IS NULL",
                (job_id.as_str(), drone_id.as_str(), at.as_str()),
            )
            .map_err(fault("stamping a task's edit calls"))
            .map_err(WriteError::Database)?;
        if kept == 0 {
            return Ok(false);
        }
        for path in paths {
            self.conn
                .execute(
                    "INSERT OR IGNORE INTO job_task_edits (job_id, drone_id, path) \
                     VALUES (?1, ?2, ?3)",
                    (job_id.as_str(), drone_id.as_str(), path.as_str()),
                )
                .map_err(fault("keeping a task's edit call"))
                .map_err(WriteError::Database)?;
        }
        Ok(true)
    }

    /// Every file any of this Job's task Drones named in an edit call.
    pub fn task_edits(&self, job_id: &JobId) -> Result<Vec<TaskEdit>, LoadJobError> {
        self.collect(
            "SELECT drone_id, path FROM job_task_edits WHERE job_id = ?1 \
             ORDER BY drone_id, path",
            job_id,
            "reading a Job's task edit calls",
            |row| {
                Ok(TaskEdit {
                    drone_id: DroneId::carried(core_model::Ulid::carried(string(row, "drone_id")?)),
                    path: string(row, "path")?,
                })
            },
        )
        .map_err(LoadJobError::Unreadable)
    }

    /// Keep a pair run apart. A pair already apart is kept once.
    pub(crate) fn record_apart(&mut self, job_id: &JobId, apart: &Apart) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO job_group_apart (job_id, grp, first, second, paths, at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    job_id.as_str(),
                    apart.group.number(),
                    apart.tasks.0.number(),
                    apart.tasks.1.number(),
                    apart.paths.join("\n"),
                    apart.at.as_str(),
                ],
            )
            .map_err(fault("keeping a pair run apart"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Every pair this Job's plan ran apart, oldest first.
    pub(crate) fn apart_moves(&self, job_id: &JobId) -> Result<Vec<GroupMove>, RowError> {
        self.collect(
            "SELECT grp, first, second, paths, at FROM job_group_apart WHERE job_id = ?1 \
             ORDER BY at, grp, first, second",
            job_id,
            "reading the pairs a plan ran apart",
            |row| {
                let number = |name: &'static str| -> Result<NonZeroU32, RowError> {
                    maybe_number(row, name)?
                        .and_then(NonZeroU32::new)
                        .ok_or_else(|| RowError::MalformedColumn {
                            table: "job_group_apart",
                            column: name,
                            detail: "one-based".to_string(),
                        })
                };
                Ok(GroupMove::Apart(Apart {
                    group: GroupId::numbered(number("grp")?),
                    tasks: (
                        TaskId::numbered(number("first")?),
                        TaskId::numbered(number("second")?),
                    ),
                    paths: string(row, "paths")?.lines().map(str::to_string).collect(),
                    at: Timestamp::from_rfc3339(string(row, "at")?),
                }))
            },
        )
    }
}
