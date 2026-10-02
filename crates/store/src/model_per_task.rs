//! A model per task: the Job's tier map, the model each Drone was spawned as,
//! and a person's edit to a task. Spike 022, slice 3.
//!
//! **The map is a Job's setting, kept as rows**, one per tier it names, so a
//! tier left out is a row that is not there and never a value meaning "Auto"
//! (answer 8). **The model a Drone ran is written at its spawn**, beside
//! [`crate::task_drones`]'s binding and for its reason: it has to survive a
//! Fleet restarting, and nothing else records it.

use core_model::{DroneId, JobId, ModelName, TaskTier, TierModels};

use crate::error::{fault, LoadJobError, WriteError};
use crate::open::Store;

/// Version 97 — a task's tier and a person's edit on the plan, a Job's tier
/// map, and the model each Drone ran.
///
/// The plan's changes are rebuilt the way [`V95`](crate::groups::V95) rebuilt
/// them, to admit `edited` and carry its `model`; an edit's other fields reuse
/// the columns an add writes, `NULL` being a field the person left alone.
/// Nothing is backfilled: a task recorded before this has no tier, which is
/// Armada picking, and a Drone spawned before it names no model.
pub(crate) const V97: &str = r#"
CREATE TABLE job_work_plan_changes_wide (
    job_id     TEXT NOT NULL REFERENCES jobs(job_id),
    seq        INTEGER NOT NULL CHECK (seq > 0),
    change     TEXT NOT NULL CHECK (change IN
                   ('recorded', 'added', 'updated', 'moved_task', 'moved_group', 'edited')),
    by_step    TEXT,
    by_attempt INTEGER CHECK (by_attempt IS NULL OR by_attempt > 0),
    at         TEXT NOT NULL,
    approach   TEXT,
    task_id    INTEGER CHECK (task_id IS NULL OR task_id > 0),
    title      TEXT,
    detail     TEXT,
    after_task INTEGER CHECK (after_task IS NULL OR after_task > 0),
    state      TEXT CHECK (state IS NULL OR state IN
                   ('open', 'working', 'handed_in', 'done', 'failed', 'dropped')),
    reason     TEXT,
    scope      TEXT,
    expects    TEXT,
    shown      TEXT,
    grp        INTEGER CHECK (grp IS NULL OR grp > 0),
    after_grp  INTEGER CHECK (after_grp IS NULL OR after_grp > 0),
    model      TEXT CHECK (model IS NULL OR trim(model) <> ''),
    PRIMARY KEY (job_id, seq),
    CHECK ((by_step IS NULL) = (by_attempt IS NULL)),
    CHECK (state IS NULL OR
           ((state IN ('dropped', 'failed')) = (reason IS NOT NULL AND trim(reason) <> ''))),
    CHECK (change = 'edited' OR model IS NULL)
) STRICT;

INSERT INTO job_work_plan_changes_wide (
    job_id, seq, change, by_step, by_attempt, at, approach, task_id, title, detail,
    after_task, state, reason, scope, expects, shown, grp, after_grp
) SELECT job_id, seq, change, by_step, by_attempt, at, approach, task_id, title, detail,
         after_task, state, reason, scope, expects, shown, grp, after_grp
  FROM job_work_plan_changes;

DROP TABLE job_work_plan_changes;
ALTER TABLE job_work_plan_changes_wide RENAME TO job_work_plan_changes;

CREATE TRIGGER job_work_plan_changes_are_never_edited
BEFORE UPDATE ON job_work_plan_changes
BEGIN
    SELECT RAISE(ABORT, 'a plan change is never edited');
END;

CREATE TRIGGER job_work_plan_changes_are_never_removed_from_a_job_that_exists
BEFORE DELETE ON job_work_plan_changes
WHEN EXISTS (SELECT 1 FROM jobs WHERE jobs.job_id = OLD.job_id)
BEGIN
    SELECT RAISE(ABORT, 'a plan change is never removed from a Job that exists');
END;

ALTER TABLE job_work_plan_tasks ADD COLUMN tier TEXT
    CHECK (tier IS NULL OR tier IN ('difficult', 'medium', 'easy'));

CREATE TABLE job_tier_models (
    job_id TEXT NOT NULL REFERENCES jobs(job_id),
    tier   TEXT NOT NULL CHECK (tier IN ('difficult', 'medium', 'easy')),
    model  TEXT NOT NULL CHECK (trim(model) <> ''),
    PRIMARY KEY (job_id, tier)
) STRICT;

CREATE TABLE job_drone_models (
    job_id   TEXT NOT NULL REFERENCES jobs(job_id),
    drone_id TEXT NOT NULL,
    model    TEXT NOT NULL CHECK (trim(model) <> ''),
    PRIMARY KEY (job_id, drone_id)
) STRICT;
"#;

impl Store {
    /// Which model each tier of this Job's tasks runs on. Empty is Armada
    /// picking for every tier, which is every Job nobody set a map for.
    pub fn tier_models(&self, job_id: &JobId) -> Result<TierModels, LoadJobError> {
        let reading = "reading a job's tier map";
        let mut asked = self
            .conn
            .prepare("SELECT tier, model FROM job_tier_models WHERE job_id = ?1")
            .map_err(fault(reading))
            .map_err(LoadJobError::Database)?;
        let rows = asked
            .query_map((job_id.as_str(),), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(fault(reading))
            .map_err(LoadJobError::Database)?;
        let mut tiers = TierModels::default();
        for row in rows {
            let (tier, model) = row
                .map_err(fault(reading))
                .map_err(LoadJobError::Database)?;
            // The table's checks admit nothing else, so a row that does not
            // read is skipped rather than failing every spawn of the Job.
            if let (Some(tier), Ok(model)) = (TaskTier::from_wire(&tier), ModelName::new(&model)) {
                tiers = tiers.with(tier, model);
            }
        }
        Ok(tiers)
    }

    /// Replace this Job's tier map whole. The next spawn reads it; a Drone
    /// already running keeps the model it was started as.
    pub fn set_tier_models(
        &mut self,
        job_id: &JobId,
        tiers: &TierModels,
    ) -> Result<(), WriteError> {
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting a tier map change"))
            .map_err(WriteError::Database)?;
        let known: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM jobs WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| row.get(0),
            )
            .map_err(fault("finding the job a tier map is for"))
            .map_err(WriteError::Database)?;
        if known == 0 {
            return Err(WriteError::NoSuchJob {
                job_id: job_id.clone(),
            });
        }
        tx.execute(
            "DELETE FROM job_tier_models WHERE job_id = ?1",
            (job_id.as_str(),),
        )
        .map_err(fault("clearing a job's tier map"))
        .map_err(WriteError::Database)?;
        for (tier, model) in tiers.named() {
            tx.execute(
                "INSERT INTO job_tier_models (job_id, tier, model) VALUES (?1, ?2, ?3)",
                (job_id.as_str(), tier.as_wire(), model.as_str()),
            )
            .map_err(fault("keeping a job's tier map"))
            .map_err(WriteError::Database)?;
        }
        tx.commit()
            .map_err(fault("committing a job's tier map"))
            .map_err(WriteError::Database)
    }

    /// Keep the model a Drone was spawned as.
    pub fn record_drone_model(
        &mut self,
        job_id: &JobId,
        drone_id: &DroneId,
        model: &ModelName,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO job_drone_models (job_id, drone_id, model) VALUES (?1, ?2, ?3)",
                (job_id.as_str(), drone_id.as_str(), model.as_str()),
            )
            .map_err(fault("keeping the model a drone ran"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// The model each of this Job's Drones was spawned as, where it was kept.
    pub fn drone_models(&self, job_id: &JobId) -> Result<Vec<(DroneId, String)>, LoadJobError> {
        let reading = "reading the models a job's drones ran";
        let mut asked = self
            .conn
            .prepare("SELECT drone_id, model FROM job_drone_models WHERE job_id = ?1")
            .map_err(fault(reading))
            .map_err(LoadJobError::Database)?;
        let rows = asked
            .query_map((job_id.as_str(),), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(fault(reading))
            .map_err(LoadJobError::Database)?;
        let mut models = Vec::new();
        for row in rows {
            let (drone, model) = row
                .map_err(fault(reading))
                .map_err(LoadJobError::Database)?;
            models.push((DroneId::carried(core_model::Ulid::carried(drone)), model));
        }
        Ok(models)
    }
}
