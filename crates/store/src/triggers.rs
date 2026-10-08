//! The Triggers frozen onto a Job at approval, and every firing of one.
//! `docs/concepts/trigger.md`.
//!
//! **Two tables.** The frozen set is written once, whole, and read at every
//! moment; a firing is appended when it opens and updated once when it ends.
//! A step restarted fires again, so a firing is a row of its own and not a
//! column of the frozen one.

use core_model::{
    FixChoice, FrozenTrigger, JobId, OnTriggerFailure, RepairRecord, StepId, Timestamp,
    TriggerFiring, TriggerResolution, TriggerSkipped, TriggerSource, TriggerState, TriggerWhen,
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
                TriggerResolution::Drone { brief } => ("drone", brief, false),
                TriggerResolution::Skipped(TriggerSkipped::NotInThisRepo { command }) => {
                    ("not_in_this_repo", command, false)
                }
                TriggerResolution::Skipped(TriggerSkipped::SkillNotRun { skill }) => {
                    ("skill", skill, false)
                }
                // Never a resolution: only the owner's skip of a held firing
                // writes it, and that is a firing's.
                TriggerResolution::Skipped(TriggerSkipped::ByOwner) => {
                    ("skill", &String::new(), false)
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
                "drone" => TriggerResolution::Drone { brief: runs },
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
            Some(TriggerSkipped::ByOwner) => (Some("by_owner"), None),
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

    /// Write what a firing's repair has come to: its state, the repair's own
    /// record, and when it ended where it has.
    pub fn settle_repair(
        &mut self,
        id: i64,
        state: TriggerState,
        repair: &RepairRecord,
        ended_at: Option<&Timestamp>,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE job_triggers SET state = ?2, repair_tries = ?3, repair_branch = ?4,
                     fix_choice = ?5, fix_pr = ?6, ended_at = ?7, repair_settled_at = ?8,
                     fix_files = ?9
                 WHERE firing_id = ?1",
                rusqlite::params![
                    id,
                    state.as_wire(),
                    repair.tries,
                    repair.branch,
                    repair.choice.map(FixChoice::as_wire),
                    repair.pull_request,
                    ended_at.map(Timestamp::as_str),
                    repair.settled_at.as_ref().map(Timestamp::as_str),
                    repair.files.join("\n"),
                ],
            )
            .map_err(fault("recording a trigger's repair"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Every firing for this Job, in the order they opened.
    pub fn trigger_firings(&self, job_id: &JobId) -> Result<Vec<TriggerFiring>, LoadJobError> {
        Ok(self
            .firings_with_ids(job_id)?
            .into_iter()
            .map(|(_, firing)| firing)
            .collect())
    }

    /// [`trigger_firings`](Store::trigger_firings) with each firing's id, which
    /// is what [`settle_repair`](Store::settle_repair) is asked by.
    pub fn firings_with_ids(
        &self,
        job_id: &JobId,
    ) -> Result<Vec<(i64, TriggerFiring)>, LoadJobError> {
        Ok(self
            .firings_where("job_id = ?1", &[&job_id.as_str()])?
            .into_iter()
            .map(|(_, id, firing)| (id, firing))
            .collect())
    }

    /// Let go of, or take up again, a firing that held its Job: its state, the
    /// way it ended, and whether the next entry to its moment must pass it by.
    /// **`released` is set for `step_starts` only**: that is the one moment a
    /// Job enters again after the hold, and re-firing it would undo the skip.
    pub fn settle_hold(
        &mut self,
        id: i64,
        after: &TriggerFiring,
        released: bool,
    ) -> Result<(), WriteError> {
        let why = match &after.skipped {
            Some(TriggerSkipped::ByOwner) => Some("by_owner"),
            _ => None,
        };
        self.conn
            .execute(
                "UPDATE job_triggers SET state = ?2, exit_code = ?3, ended_at = ?4,
                     skipped_why = COALESCE(?5, skipped_why), released = ?6
                 WHERE firing_id = ?1",
                rusqlite::params![
                    id,
                    after.state.as_wire(),
                    after.exit_code,
                    after.ended_at.as_ref().map(Timestamp::as_str),
                    why,
                    released,
                ],
            )
            .map_err(fault("recording how a held trigger was settled"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Whether a hold at `(when, step)` was let go since the Job last entered
    /// it, and **forget that**: the entry that asks is the one that passes it.
    /// Triggers and added steps both.
    pub fn take_released_hold(
        &mut self,
        job_id: &JobId,
        when: TriggerWhen,
        step: &StepId,
    ) -> Result<bool, WriteError> {
        let writing = fault("passing a released hold");
        let args = (job_id.as_str(), when.as_wire(), step.as_str());
        let triggers = self
            .conn
            .execute(
                "UPDATE job_triggers SET released = 0
                 WHERE job_id = ?1 AND moment = ?2 AND step_id = ?3 AND released = 1",
                args,
            )
            .map_err(writing)
            .map_err(WriteError::Database)?;
        let additions = self
            .conn
            .execute(
                "UPDATE job_additions SET released = 0
                 WHERE job_id = ?1 AND moment = ?2 AND step_id = ?3 AND released = 1",
                args,
            )
            .map_err(fault("passing a released hold"))
            .map_err(WriteError::Database)?;
        Ok(triggers + additions > 0)
    }

    /// The firings that hold this Job: the latest of each Trigger at its
    /// moment, blocking, with its failure not settled.
    pub fn holding_firings(
        &self,
        job_id: &JobId,
    ) -> Result<Vec<(i64, TriggerFiring)>, LoadJobError> {
        Ok(self
            .firings_where(
                "job_id = ?1 AND block_on_fail = 1
                 AND (state IN ('held', 'repairing', 'rerunning', 'fix_ready')
                      OR (state = 'running' AND repair_tries > 0))
                 AND firing_id IN (SELECT MAX(firing_id) FROM job_triggers
                                   GROUP BY job_id, name, moment, step_id)",
                &[&job_id.as_str()],
            )?
            .into_iter()
            .map(|(_, id, firing)| (id, firing))
            .collect())
    }

    /// Firings whose repair has not finished: `repairing`, or `rerunning` on
    /// either branch, and a Skill's run with a Drone on it, which reads
    /// `running` with a try spent. What a restarted Fleet takes up again.
    pub fn unfinished_repairs(&self) -> Result<Vec<(JobId, i64, TriggerFiring)>, LoadJobError> {
        self.firings_where(
            "state IN ('repairing', 'rerunning') OR (state = 'running' AND repair_tries > 0)",
            &[],
        )
    }

    /// Firings whose repair ended and left its branch behind: placed on the
    /// Job's, or failed. **Never a `new_pr` one**, whose branch is the pull
    /// request's head.
    pub fn repair_branches_left(&self) -> Result<Vec<(JobId, i64, TriggerFiring)>, LoadJobError> {
        self.firings_where(
            "repair_branch IS NOT NULL AND state IN ('passed', 'failed', 'held')
             AND (fix_choice IS NULL OR fix_choice = 'this_branch')",
            &[],
        )
    }

    /// Fixes the owner chose a place for that have not been placed yet.
    pub fn chosen_fixes(&self) -> Result<Vec<(JobId, i64, TriggerFiring)>, LoadJobError> {
        self.firings_where("state = 'fix_ready' AND fix_choice IS NOT NULL", &[])
    }

    /// What is waiting on a person: a fix with no choice, a Trigger that
    /// failed after a repair was tried, and one that holds its Job. **Only the latest firing of a Trigger
    /// counts**, so a later pass clears it, and a Job whose disk was given back
    /// has none.
    pub fn repairs_waiting_on_a_person(
        &self,
    ) -> Result<Vec<(JobId, i64, TriggerFiring)>, LoadJobError> {
        self.firings_where(
            "firing_id IN (SELECT MAX(firing_id) FROM job_triggers
                            GROUP BY job_id, name, moment, step_id)
             AND job_id IN (SELECT job_id FROM jobs WHERE reclaimed_at IS NULL)
             AND ((state = 'failed' AND repair_tries > 0)
                  OR (state = 'fix_ready' AND fix_choice IS NULL)
                  OR state = 'held')",
            &[],
        )
    }

    /// What on this Job waits on a person, by the rule
    /// [`repairs_waiting_on_a_person`](Store::repairs_waiting_on_a_person) has
    /// for the whole board: a hold, a fix with no choice, a failure after a
    /// repair was tried.
    pub fn alerting_firings(
        &self,
        job_id: &JobId,
    ) -> Result<Vec<(i64, TriggerFiring)>, LoadJobError> {
        Ok(self
            .firings_where(
                "job_id = ?1
                 AND firing_id IN (SELECT MAX(firing_id) FROM job_triggers
                                   GROUP BY job_id, name, moment, step_id)
                 AND ((state = 'failed' AND repair_tries > 0)
                      OR (state = 'fix_ready' AND fix_choice IS NULL)
                      OR state = 'held')",
                &[&job_id.as_str()],
            )?
            .into_iter()
            .map(|(_, id, firing)| (id, firing))
            .collect())
    }

    /// Set the `released` mark on a firing a repair let go, for the next
    /// entry to a `step_starts` moment to pass it by.
    pub fn mark_hold_released(&mut self, id: i64) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE job_triggers SET released = 1 WHERE firing_id = ?1 AND moment = 'step_starts'",
                (id,),
            )
            .map_err(fault("recording that a repair let a hold go"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    fn firings_where(
        &self,
        clause: &str,
        params: &[&dyn rusqlite::ToSql],
    ) -> Result<Vec<(JobId, i64, TriggerFiring)>, LoadJobError> {
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT name, moment, step_id, source, state, skipped_why, skipped_name,
                     exit_code, block_on_fail, repair_on_fail, started_at, ended_at,
                     firing_id, repair_tries, repair_branch, fix_choice, fix_pr,
                     repair_settled_at, job_id, fix_files
                 FROM job_triggers WHERE {clause} ORDER BY firing_id"
            ))
            .map_err(fault("reading a job's trigger firings"))
            .map_err(LoadJobError::Database)?;
        let rows = statement
            .query_map(params, |row| {
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
                    row.get::<_, i64>(12)?,
                    row.get::<_, u32>(13)?,
                    row.get::<_, Option<String>>(14)?,
                    row.get::<_, Option<String>>(15)?,
                    row.get::<_, Option<String>>(16)?,
                    row.get::<_, Option<String>>(17)?,
                    row.get::<_, String>(18)?,
                    row.get::<_, String>(19)?,
                ))
            })
            .map_err(fault("reading a job's trigger firings"))
            .map_err(LoadJobError::Database)?;
        let table = "job_triggers";
        let mut out = Vec::new();
        for row in rows {
            let (
                name,
                moment,
                step,
                source,
                state,
                why,
                named,
                code,
                block,
                repair,
                began,
                ended,
                id,
                tries,
                branch,
                choice,
                pull_request,
                settled,
                job,
                files,
            ) = row
                .map_err(fault("reading a job's trigger firings"))
                .map_err(LoadJobError::Database)?;
            let skipped = match (why.as_deref(), named) {
                (None, _) => None,
                (Some("not_in_this_repo"), Some(command)) => {
                    Some(TriggerSkipped::NotInThisRepo { command })
                }
                (Some("skill_not_run"), Some(skill)) => Some(TriggerSkipped::SkillNotRun { skill }),
                (Some("by_owner"), _) => Some(TriggerSkipped::ByOwner),
                (Some(other), _) => return Err(unknown(table, "skipped_why", other.to_string())),
            };
            let choice = match choice {
                None => None,
                Some(text) => Some(
                    FixChoice::from_wire(&text)
                        .ok_or_else(|| unknown(table, "fix_choice", text))?,
                ),
            };
            out.push((
                JobId::carried(core_model::Ulid::carried(job)),
                id,
                TriggerFiring {
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
                    repair: RepairRecord {
                        tries,
                        branch,
                        choice,
                        pull_request,
                        settled_at: settled.map(Timestamp::from_rfc3339),
                        files: files.lines().map(str::to_string).collect(),
                    },
                },
            ));
        }
        Ok(out)
    }
}
