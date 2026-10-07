//! Steps added to one Job. `docs/concepts/trigger.md`, *Steps added to one Job*.
//!
//! **Beside the frozen workflow, never in it**: `job_steps` and the Job's
//! workflow are not touched here, and nothing in this file moves the Job's
//! status. A row is written when a step is added and rewritten when its moment
//! fires; **one that has not fired is the only one that can be removed**, and
//! removing keeps the row.

use core_model::{
    AddedKind, AddedStep, Fired, JobId, Kept, NotRun, OnTriggerFailure, Placed, StepId, Timestamp,
    TriggerState, TriggerWhen,
};

use crate::error::{fault, LoadJobError, RowError, WriteError};
use crate::open::Store;

/// What an addition is before it has an id, a place in the order, or a firing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAddition {
    pub kind: AddedKind,
    pub when: TriggerWhen,
    pub step: StepId,
    pub on_failure: OnTriggerFailure,
}

/// What removing an addition came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Removal {
    Removed,
    /// The Job holds no such addition, or it was removed already.
    NoSuch,
    /// Its moment has come, so it is part of what the Job did.
    Fired,
}

fn unknown(column: &'static str, value: String) -> LoadJobError {
    LoadJobError::Unreadable(RowError::UnknownEnumValue {
        table: "job_additions",
        column,
        value,
    })
}

fn insert(
    tx: &rusqlite::Transaction<'_>,
    job_id: &JobId,
    new: &NewAddition,
    placed: Placed,
    at: &Timestamp,
) -> Result<AddedStep, WriteError> {
    let writing = fault("adding a step to a job");
    let held: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(ordinal), 0) FROM job_additions WHERE job_id = ?1",
            (job_id.as_str(),),
            |row| row.get(0),
        )
        .map_err(writing)
        .map_err(WriteError::Database)?;
    // Over removed ones too, so a number that was in use is not handed out again.
    let ordinal = held + 1;
    let id = format!("a{ordinal}");
    tx.execute(
        "INSERT INTO job_additions (job_id, addition_id, ordinal, kind, runs_text, moment,
             step_id, block_on_fail, repair_on_fail, placed, added_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        rusqlite::params![
            job_id.as_str(),
            id,
            ordinal,
            new.kind.as_wire(),
            new.kind.text(),
            new.when.as_wire(),
            new.step.as_str(),
            new.on_failure.block,
            new.on_failure.repair,
            placed.as_wire(),
            at.as_str(),
        ],
    )
    .map_err(fault("adding a step to a job"))
    .map_err(WriteError::Database)?;
    Ok(AddedStep {
        id,
        kind: new.kind.clone(),
        when: new.when,
        step: new.step.clone(),
        on_failure: new.on_failure,
        placed,
        added_at: at.clone(),
        fired: None,
        kept: None,
    })
}

/// One row as SQLite hands it back, before its words become types.
struct Row {
    id: String,
    kind: String,
    text: String,
    moment: String,
    step: String,
    block: bool,
    repair: bool,
    placed: String,
    added: String,
    state: Option<String>,
    why: Option<String>,
    named: Option<String>,
    code: Option<i32>,
    began: Option<String>,
    ended: Option<String>,
    kept: Option<String>,
}

impl Row {
    fn of(row: &rusqlite::Row<'_>) -> rusqlite::Result<Row> {
        Ok(Row {
            id: row.get(0)?,
            kind: row.get(1)?,
            text: row.get(2)?,
            moment: row.get(3)?,
            step: row.get(4)?,
            block: row.get(5)?,
            repair: row.get(6)?,
            placed: row.get(7)?,
            added: row.get(8)?,
            state: row.get(9)?,
            why: row.get(10)?,
            named: row.get(11)?,
            code: row.get(12)?,
            began: row.get(13)?,
            ended: row.get(14)?,
            kept: row.get(15)?,
        })
    }

    fn typed(self) -> Result<AddedStep, LoadJobError> {
        let fired = match self.state {
            None => None,
            Some(state) => {
                let not_run = match (self.why.as_deref(), self.named) {
                    (None, _) => None,
                    (Some("not_in_this_repo"), Some(command)) => {
                        Some(NotRun::NotInThisRepo { command })
                    }
                    (Some("skill_not_run"), Some(skill)) => Some(NotRun::SkillNotRun { skill }),
                    (Some("drone_step_not_run"), _) => Some(NotRun::DroneStepNotRun),
                    (Some("by_owner"), _) => Some(NotRun::ByOwner),
                    (Some(other), _) => return Err(unknown("not_run_why", other.to_string())),
                };
                Some(Fired {
                    state: TriggerState::from_wire(&state)
                        .ok_or_else(|| unknown("state", state))?,
                    not_run,
                    exit_code: self.code,
                    started_at: Timestamp::from_rfc3339(self.began.unwrap_or_default()),
                    ended_at: self.ended.map(Timestamp::from_rfc3339),
                })
            }
        };
        let kept = match self.kept {
            None => None,
            Some(kept) => Some(Kept::from_wire(&kept).ok_or_else(|| unknown("kept", kept))?),
        };
        Ok(AddedStep {
            id: self.id,
            kind: AddedKind::from_wire(&self.kind, self.text)
                .ok_or_else(|| unknown("kind", self.kind))?,
            when: TriggerWhen::from_wire(&self.moment)
                .ok_or_else(|| unknown("moment", self.moment))?,
            step: StepId::new(self.step),
            on_failure: OnTriggerFailure {
                block: self.block,
                repair: self.repair,
            },
            placed: Placed::from_wire(&self.placed)
                .ok_or_else(|| unknown("placed", self.placed))?,
            added_at: Timestamp::from_rfc3339(self.added),
            fired,
            kept,
        })
    }
}

impl Store {
    /// Add one step to a Job that is underway, answering it with its id.
    pub fn add_job_step(
        &mut self,
        job_id: &JobId,
        new: &NewAddition,
        placed: Placed,
        at: &Timestamp,
    ) -> Result<AddedStep, WriteError> {
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting to add a step to a job"))
            .map_err(WriteError::Database)?;
        let added = insert(&tx, job_id, new, placed, at)?;
        tx.commit()
            .map_err(fault("committing a step added to a job"))
            .map_err(WriteError::Database)?;
        Ok(added)
    }

    /// Replace what was placed at approval with `set`. **Written at the press
    /// and before the Job runs**, so what is replaced is a draft and nothing
    /// that fired. Additions added while running are not touched.
    pub fn place_additions_at_approval(
        &mut self,
        job_id: &JobId,
        set: &[NewAddition],
        at: &Timestamp,
    ) -> Result<Vec<AddedStep>, WriteError> {
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting to place a job's added steps"))
            .map_err(WriteError::Database)?;
        tx.execute(
            "DELETE FROM job_additions WHERE job_id = ?1 AND placed = 'approval' AND state IS NULL",
            (job_id.as_str(),),
        )
        .map_err(fault("replacing a job's added steps"))
        .map_err(WriteError::Database)?;
        let mut out = Vec::new();
        for new in set {
            out.push(insert(&tx, job_id, new, Placed::AtApproval, at)?);
        }
        tx.commit()
            .map_err(fault("committing a job's added steps"))
            .map_err(WriteError::Database)?;
        Ok(out)
    }

    /// Every addition this Job holds, in the order they were added. A removed
    /// one is not here.
    pub fn job_additions(&self, job_id: &JobId) -> Result<Vec<AddedStep>, LoadJobError> {
        let mut statement = self
            .conn
            .prepare(
                "SELECT addition_id, kind, runs_text, moment, step_id, block_on_fail,
                     repair_on_fail, placed, added_at, state, not_run_why, not_run_name,
                     exit_code, started_at, ended_at, kept
                 FROM job_additions WHERE job_id = ?1 AND removed_at IS NULL ORDER BY ordinal",
            )
            .map_err(fault("reading a job's added steps"))
            .map_err(LoadJobError::Database)?;
        let rows = statement
            .query_map((job_id.as_str(),), Row::of)
            .map_err(fault("reading a job's added steps"))
            .map_err(LoadJobError::Database)?;
        let mut out = Vec::new();
        for row in rows {
            let row = row
                .map_err(fault("reading a job's added steps"))
                .map_err(LoadJobError::Database)?;
            out.push(row.typed()?);
        }
        Ok(out)
    }

    /// Write what its moment came to: opened running, skipped or held, and
    /// again where it ended. The latest firing is the one kept.
    pub fn set_addition_fired(
        &mut self,
        job_id: &JobId,
        addition_id: &str,
        fired: &Fired,
    ) -> Result<(), WriteError> {
        let (why, named) = match &fired.not_run {
            Some(NotRun::NotInThisRepo { command }) => {
                (Some("not_in_this_repo"), Some(command.as_str()))
            }
            Some(NotRun::SkillNotRun { skill }) => (Some("skill_not_run"), Some(skill.as_str())),
            Some(NotRun::DroneStepNotRun) => (Some("drone_step_not_run"), None),
            Some(NotRun::ByOwner) => (Some("by_owner"), None),
            None => (None, None),
        };
        self.conn
            .execute(
                "UPDATE job_additions
                 SET state = ?3, not_run_why = ?4, not_run_name = ?5, exit_code = ?6,
                     started_at = ?7, ended_at = ?8
                 WHERE job_id = ?1 AND addition_id = ?2",
                rusqlite::params![
                    job_id.as_str(),
                    addition_id,
                    fired.state.as_wire(),
                    why,
                    named,
                    fired.exit_code,
                    fired.started_at.as_str(),
                    fired.ended_at.as_ref().map(Timestamp::as_str),
                ],
            )
            .map_err(fault("recording how an added step went"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Settle an addition that held its Job: the firing as it now stands, and
    /// whether the next entry to its moment must pass it by, which is a
    /// `step_starts` hold's alone.
    pub fn settle_addition_hold(
        &mut self,
        job_id: &JobId,
        addition_id: &str,
        fired: &Fired,
        released: bool,
    ) -> Result<(), WriteError> {
        self.set_addition_fired(job_id, addition_id, fired)?;
        self.conn
            .execute(
                "UPDATE job_additions SET released = ?3 WHERE job_id = ?1 AND addition_id = ?2",
                (job_id.as_str(), addition_id, released),
            )
            .map_err(fault("recording how a held added step was settled"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Say where an addition was kept for every Job.
    pub fn set_addition_kept(
        &mut self,
        job_id: &JobId,
        addition_id: &str,
        kept: Kept,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE job_additions SET kept = ?3 WHERE job_id = ?1 AND addition_id = ?2",
                (job_id.as_str(), addition_id, kept.as_wire()),
            )
            .map_err(fault("recording where an added step was kept"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Remove an addition **that has not fired**. The row stays, marked.
    pub fn remove_job_step(
        &mut self,
        job_id: &JobId,
        addition_id: &str,
        at: &Timestamp,
    ) -> Result<Removal, WriteError> {
        let changed = self
            .conn
            .execute(
                "UPDATE job_additions SET removed_at = ?3
                 WHERE job_id = ?1 AND addition_id = ?2 AND removed_at IS NULL AND state IS NULL",
                (job_id.as_str(), addition_id, at.as_str()),
            )
            .map_err(fault("removing a step added to a job"))
            .map_err(WriteError::Database)?;
        if changed > 0 {
            return Ok(Removal::Removed);
        }
        let held: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM job_additions
                 WHERE job_id = ?1 AND addition_id = ?2 AND removed_at IS NULL",
                (job_id.as_str(), addition_id),
                |row| row.get(0),
            )
            .map_err(fault("removing a step added to a job"))
            .map_err(WriteError::Database)?;
        Ok(match held {
            0 => Removal::NoSuch,
            _ => Removal::Fired,
        })
    }
}
