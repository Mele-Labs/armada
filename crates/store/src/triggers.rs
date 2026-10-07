//! The Triggers frozen onto a Job at approval, and every firing of one.
//! `docs/concepts/trigger.md`.
//!
//! **Two tables.** The frozen set is written once, whole, and read at every
//! moment; a firing is appended when it opens and updated once when it ends.
//! A step restarted fires again, so a firing is a row of its own and not a
//! column of the frozen one.

use core_model::{
    FrozenTrigger, JobId, OnTriggerFailure, StepId, Timestamp, TriggerFiring, TriggerResolution,
    TriggerSkipped, TriggerSource, TriggerState, TriggerWhen,
};

use crate::error::{fault, LoadJobError, RowError, WriteError};
use crate::open::Store;

fn source_of(text: &str) -> Option<TriggerSource> {
    [
        TriggerSource::Armada,
        TriggerSource::Repository,
        TriggerSource::Machine,
    ]
    .into_iter()
    .find(|source| source.as_wire() == text)
}

fn unknown(table: &'static str, column: &'static str, value: String) -> LoadJobError {
    LoadJobError::Unreadable(RowError::UnknownEnumValue {
        table,
        column,
        value,
    })
}

impl Store {
    /// Keep the Triggers this Job runs, replacing any kept. Written at the
    /// approval and never again, so what a person saved afterwards is not here.
    pub fn freeze_triggers(
        &mut self,
        job_id: &JobId,
        frozen: &[FrozenTrigger],
    ) -> Result<(), WriteError> {
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting to freeze a job's triggers"))
            .map_err(WriteError::Database)?;
        let writing = fault("freezing a job's triggers");
        tx.execute(
            "DELETE FROM job_frozen_triggers WHERE job_id = ?1",
            (job_id.as_str(),),
        )
        .map_err(writing)
        .map_err(WriteError::Database)?;
        for (ordinal, one) in frozen.iter().enumerate() {
            let (resolution, name, asks_first) = match &one.resolution {
                TriggerResolution::Command { name, asks_first } => ("command", name, *asks_first),
                TriggerResolution::Skill { name } => ("skill", name, false),
                TriggerResolution::Skipped(TriggerSkipped::NotInThisRepo { command }) => {
                    ("not_in_this_repo", command, false)
                }
                TriggerResolution::Skipped(TriggerSkipped::SkillNotRun { skill }) => {
                    ("skill", skill, false)
                }
            };
            tx.execute(
                "INSERT INTO job_frozen_triggers (job_id, ordinal, name, moment, step_id, source,
                     resolution, runs_name, asks_first, block_on_fail, repair_on_fail)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                rusqlite::params![
                    job_id.as_str(),
                    ordinal as i64,
                    one.name,
                    one.when.as_wire(),
                    one.step.as_str(),
                    one.source.as_wire(),
                    resolution,
                    name,
                    asks_first,
                    one.on_failure.block,
                    one.on_failure.repair,
                ],
            )
            .map_err(fault("freezing a job's triggers"))
            .map_err(WriteError::Database)?;
        }
        tx.commit()
            .map_err(fault("committing a job's frozen triggers"))
            .map_err(WriteError::Database)
    }

    /// The Triggers frozen onto this Job, in the order they were frozen. Empty
    /// for a Job approved before Triggers, which runs none.
    pub fn frozen_triggers(&self, job_id: &JobId) -> Result<Vec<FrozenTrigger>, LoadJobError> {
        let reading = fault("reading a job's frozen triggers");
        let mut statement = self
            .conn
            .prepare(
                "SELECT name, moment, step_id, source, resolution, runs_name, asks_first,
                     block_on_fail, repair_on_fail
                 FROM job_frozen_triggers WHERE job_id = ?1 ORDER BY ordinal",
            )
            .map_err(reading)
            .map_err(LoadJobError::Database)?;
        let rows = statement
            .query_map((job_id.as_str(),), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, bool>(6)?,
                    row.get::<_, bool>(7)?,
                    row.get::<_, bool>(8)?,
                ))
            })
            .map_err(fault("reading a job's frozen triggers"))
            .map_err(LoadJobError::Database)?;
        let table = "job_frozen_triggers";
        let mut out = Vec::new();
        for row in rows {
            let (name, moment, step, source, how, runs, asks_first, block, repair) = row
                .map_err(fault("reading a job's frozen triggers"))
                .map_err(LoadJobError::Database)?;
            let resolution = match how.as_str() {
                "command" => TriggerResolution::Command {
                    name: runs,
                    asks_first,
                },
                "skill" => TriggerResolution::Skill { name: runs },
                "not_in_this_repo" => {
                    TriggerResolution::Skipped(TriggerSkipped::NotInThisRepo { command: runs })
                }
                other => return Err(unknown(table, "resolution", other.to_string())),
            };
            out.push(FrozenTrigger {
                name,
                when: TriggerWhen::from_wire(&moment)
                    .ok_or_else(|| unknown(table, "moment", moment))?,
                step: StepId::new(step),
                source: source_of(&source).ok_or_else(|| unknown(table, "source", source))?,
                resolution,
                on_failure: OnTriggerFailure { block, repair },
            });
        }
        Ok(out)
    }

    /// Append a firing as it opens, and answer its id for [`settle_firing`].
    ///
    /// [`settle_firing`]: Store::settle_firing
    pub fn open_firing(
        &mut self,
        job_id: &JobId,
        firing: &TriggerFiring,
    ) -> Result<i64, WriteError> {
        let (why, named) = match &firing.skipped {
            Some(TriggerSkipped::NotInThisRepo { command }) => {
                (Some("not_in_this_repo"), Some(command.as_str()))
            }
            Some(TriggerSkipped::SkillNotRun { skill }) => {
                (Some("skill_not_run"), Some(skill.as_str()))
            }
            None => (None, None),
        };
        self.conn
            .execute(
                "INSERT INTO job_triggers (job_id, name, moment, step_id, source, state,
                     skipped_why, skipped_name, exit_code, block_on_fail, repair_on_fail,
                     started_at, ended_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                rusqlite::params![
                    job_id.as_str(),
                    firing.name,
                    firing.when.as_wire(),
                    firing.step.as_str(),
                    firing.source.as_wire(),
                    firing.state.as_wire(),
                    why,
                    named,
                    firing.exit_code,
                    firing.on_failure.block,
                    firing.on_failure.repair,
                    firing.started_at.as_str(),
                    firing.ended_at.as_ref().map(Timestamp::as_str),
                ],
            )
            .map_err(fault("recording a trigger's firing"))
            .map_err(WriteError::Database)?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Write how a firing ended.
    pub fn settle_firing(&mut self, id: i64, ended: &TriggerFiring) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE job_triggers SET state = ?2, exit_code = ?3, ended_at = ?4
                 WHERE firing_id = ?1",
                rusqlite::params![
                    id,
                    ended.state.as_wire(),
                    ended.exit_code,
                    ended.ended_at.as_ref().map(Timestamp::as_str),
                ],
            )
            .map_err(fault("recording how a trigger ended"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Every firing for this Job, in the order they opened.
    pub fn trigger_firings(&self, job_id: &JobId) -> Result<Vec<TriggerFiring>, LoadJobError> {
        let mut statement = self
            .conn
            .prepare(
                "SELECT name, moment, step_id, source, state, skipped_why, skipped_name,
                     exit_code, block_on_fail, repair_on_fail, started_at, ended_at
                 FROM job_triggers WHERE job_id = ?1 ORDER BY firing_id",
            )
            .map_err(fault("reading a job's trigger firings"))
            .map_err(LoadJobError::Database)?;
        let rows = statement
            .query_map((job_id.as_str(),), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<i32>>(7)?,
                    row.get::<_, bool>(8)?,
                    row.get::<_, bool>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, Option<String>>(11)?,
                ))
            })
            .map_err(fault("reading a job's trigger firings"))
            .map_err(LoadJobError::Database)?;
        let table = "job_triggers";
        let mut out = Vec::new();
        for row in rows {
            let (name, moment, step, source, state, why, named, code, block, repair, began, ended) =
                row.map_err(fault("reading a job's trigger firings"))
                    .map_err(LoadJobError::Database)?;
            let skipped = match (why.as_deref(), named) {
                (None, _) => None,
                (Some("not_in_this_repo"), Some(command)) => {
                    Some(TriggerSkipped::NotInThisRepo { command })
                }
                (Some("skill_not_run"), Some(skill)) => Some(TriggerSkipped::SkillNotRun { skill }),
                (Some(other), _) => return Err(unknown(table, "skipped_why", other.to_string())),
            };
            out.push(TriggerFiring {
                name,
                when: TriggerWhen::from_wire(&moment)
                    .ok_or_else(|| unknown(table, "moment", moment))?,
                step: StepId::new(step),
                source: source_of(&source).ok_or_else(|| unknown(table, "source", source))?,
                on_failure: OnTriggerFailure { block, repair },
                state: TriggerState::from_wire(&state)
                    .ok_or_else(|| unknown(table, "state", state))?,
                skipped,
                exit_code: code,
                started_at: Timestamp::from_rfc3339(began),
                ended_at: ended.map(Timestamp::from_rfc3339),
            });
        }
        Ok(out)
    }
}
