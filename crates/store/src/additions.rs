//! Steps added to one Job. `docs/concepts/trigger.md`, *Steps added to one Job*.
//!
//! **Beside the frozen workflow, never in it**: `job_steps` and the Job's
//! workflow are not touched here, and nothing in this file moves the Job's
//! status. A row is written when a step is added and rewritten when its moment
//! fires; **one that has not fired is the only one that can be removed**, and
//! removing keeps the row.

use core_model::{
    AddedKind, AddedStep, Fired, FixChoice, JobId, Kept, NotRun, OnTriggerFailure, Placed,
    RepairRecord, StepId, Timestamp, TriggerState, TriggerWhen,
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

/// What changing an addition's switches came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edited {
    Changed,
    /// The Job holds no such addition, or it was removed.
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
        repair: RepairRecord::default(),
    })
}

/// One row as SQLite hands it back, before its words become types.
struct Row {
    job: String,
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
    repair_state: Option<String>,
    tries: u32,
    branch: Option<String>,
    choice: Option<String>,
    pull_request: Option<String>,
    settled: Option<String>,
    files: String,
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
            repair_state: row.get(16)?,
            tries: row.get(17)?,
            branch: row.get(18)?,
            choice: row.get(19)?,
            pull_request: row.get(20)?,
            settled: row.get(21)?,
            files: row.get(22)?,
            job: row.get(23)?,
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
                // A repair in flight reads over `running`, which is all the
                // state column can hold of it.
                let state = self.repair_state.unwrap_or(state);
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
        let choice = match self.choice {
            None => None,
            Some(choice) => {
                Some(FixChoice::from_wire(&choice).ok_or_else(|| unknown("fix_choice", choice))?)
            }
        };
        let repair = RepairRecord {
            tries: self.tries,
            branch: self.branch,
            choice,
            pull_request: self.pull_request,
            settled_at: self.settled.map(Timestamp::from_rfc3339),
            files: self.files.lines().map(str::to_string).collect(),
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
            repair,
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
        Ok(self
            .additions_where("job_id = ?1 AND removed_at IS NULL", &[&job_id.as_str()])?
            .into_iter()
            .map(|(_, added)| added)
            .collect())
    }

    /// Additions whose repair has not finished: `repairing`, or `rerunning` on
    /// either branch. What a restarted Fleet takes up again.
    pub fn unfinished_addition_repairs(&self) -> Result<Vec<(JobId, AddedStep)>, LoadJobError> {
        self.additions_where(
            "removed_at IS NULL AND (repair_state IN ('repairing', 'rerunning')
                 OR (repair_state IS NULL AND state = 'running' AND repair_tries > 0))",
            &[],
        )
    }

    /// Fixes the owner chose a place for that have not been placed yet.
    pub fn chosen_addition_fixes(&self) -> Result<Vec<(JobId, AddedStep)>, LoadJobError> {
        self.additions_where(
            "removed_at IS NULL AND repair_state = 'fix_ready' AND fix_choice IS NOT NULL",
            &[],
        )
    }

    /// What waits on a person among added steps, by the rule
    /// [`repairs_waiting_on_a_person`](Store::repairs_waiting_on_a_person) has
    /// for Triggers: a fix with no choice, and a step that failed or holds its
    /// Job after a repair was tried. A Job whose disk was given back has none.
    pub fn additions_waiting_on_a_person(&self) -> Result<Vec<(JobId, AddedStep)>, LoadJobError> {
        self.additions_where(
            "removed_at IS NULL
             AND job_id IN (SELECT job_id FROM jobs WHERE reclaimed_at IS NULL)
             AND ((state IN ('failed', 'held') AND repair_tries > 0)
                  OR (repair_state = 'fix_ready' AND fix_choice IS NULL)
                  OR state = 'awaiting_owner')",
            &[],
        )
    }

    /// Additions whose repair ended and left its branch behind: placed on the
    /// Job's, or failed. **Never a `new_pr` one**, whose branch is the pull
    /// request's head.
    pub fn addition_repair_branches_left(&self) -> Result<Vec<(JobId, AddedStep)>, LoadJobError> {
        self.additions_where(
            "repair_branch IS NOT NULL AND repair_state IS NULL
             AND state IN ('passed', 'failed', 'held')
             AND (fix_choice IS NULL OR fix_choice = 'this_branch')",
            &[],
        )
    }

    fn additions_where(
        &self,
        clause: &str,
        params: &[&dyn rusqlite::ToSql],
    ) -> Result<Vec<(JobId, AddedStep)>, LoadJobError> {
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT addition_id, kind, runs_text, moment, step_id, block_on_fail,
                     repair_on_fail, placed, added_at, state, not_run_why, not_run_name,
                     exit_code, started_at, ended_at, kept, repair_state, repair_tries,
                     repair_branch, fix_choice, fix_pr, repair_settled_at, fix_files, job_id
                 FROM job_additions WHERE {clause} ORDER BY job_id, ordinal"
            ))
            .map_err(fault("reading a job's added steps"))
            .map_err(LoadJobError::Database)?;
        let rows = statement
            .query_map(params, Row::of)
            .map_err(fault("reading a job's added steps"))
            .map_err(LoadJobError::Database)?;
        let mut out = Vec::new();
        for row in rows {
            let row = row
                .map_err(fault("reading a job's added steps"))
                .map_err(LoadJobError::Database)?;
            let job = JobId::carried(core_model::Ulid::carried(row.job.clone()));
            out.push((job, row.typed()?));
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
        // A repair in flight is `repair_state` over a `running` row, which is
        // all the CHECK on `state` allows; anything else ends it.
        let (state, in_flight) = match fired.state {
            TriggerState::Repairing | TriggerState::Rerunning | TriggerState::FixReady => {
                (TriggerState::Running, Some(fired.state.as_wire()))
            }
            other => (other, None),
        };
        self.conn
            .execute(
                "UPDATE job_additions
                 SET state = ?3, not_run_why = ?4, not_run_name = ?5, exit_code = ?6,
                     started_at = ?7, ended_at = ?8, repair_state = ?9
                 WHERE job_id = ?1 AND addition_id = ?2",
                rusqlite::params![
                    job_id.as_str(),
                    addition_id,
                    state.as_wire(),
                    why,
                    named,
                    fired.exit_code,
                    fired.started_at.as_str(),
                    fired.ended_at.as_ref().map(Timestamp::as_str),
                    in_flight,
                ],
            )
            .map_err(fault("recording how an added step went"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Write what an addition's repair has come to: its state, the repair's own
    /// record, and when it ended where it has.
    pub fn settle_addition_repair(
        &mut self,
        job_id: &JobId,
        addition_id: &str,
        fired: &Fired,
        repair: &RepairRecord,
    ) -> Result<(), WriteError> {
        self.set_addition_fired(job_id, addition_id, fired)?;
        self.conn
            .execute(
                "UPDATE job_additions SET repair_tries = ?3, repair_branch = ?4, fix_choice = ?5,
                     fix_pr = ?6, repair_settled_at = ?7, fix_files = ?8
                 WHERE job_id = ?1 AND addition_id = ?2",
                rusqlite::params![
                    job_id.as_str(),
                    addition_id,
                    repair.tries,
                    repair.branch,
                    repair.choice.map(FixChoice::as_wire),
                    repair.pull_request,
                    repair.settled_at.as_ref().map(Timestamp::as_str),
                    repair.files.join("\n"),
                ],
            )
            .map_err(fault("recording an added step's repair"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Set the `released` mark on an addition a repair let go, for the next
    /// entry to a `step_starts` moment to pass it by.
    pub fn mark_addition_hold_released(
        &mut self,
        job_id: &JobId,
        addition_id: &str,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE job_additions SET released = 1
                 WHERE job_id = ?1 AND addition_id = ?2 AND moment = 'step_starts'",
                (job_id.as_str(), addition_id),
            )
            .map_err(fault("recording that a repair let a hold go"))
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

    /// Change the switches of an addition **that has not fired**; a switch left
    /// `None` is left as it is.
    pub fn edit_job_step(
        &mut self,
        job_id: &JobId,
        addition_id: &str,
        block: Option<bool>,
        repair: Option<bool>,
    ) -> Result<Edited, WriteError> {
        let changed = self
            .conn
            .execute(
                "UPDATE job_additions
                 SET block_on_fail = COALESCE(?3, block_on_fail),
                     repair_on_fail = COALESCE(?4, repair_on_fail)
                 WHERE job_id = ?1 AND addition_id = ?2 AND removed_at IS NULL AND state IS NULL",
                (job_id.as_str(), addition_id, block, repair),
            )
            .map_err(fault("changing the switches of a step added to a job"))
            .map_err(WriteError::Database)?;
        if changed > 0 {
            return Ok(Edited::Changed);
        }
        let held: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM job_additions
                 WHERE job_id = ?1 AND addition_id = ?2 AND removed_at IS NULL",
                (job_id.as_str(), addition_id),
                |row| row.get(0),
            )
            .map_err(fault("changing the switches of a step added to a job"))
            .map_err(WriteError::Database)?;
        Ok(match held {
            0 => Edited::NoSuch,
            _ => Edited::Fired,
        })
    }
}
