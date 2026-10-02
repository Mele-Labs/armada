//! Fleet's record of each group's runs, and the group a Check run or a Record
//! row was made at. Spike 022, slice 2.
//!
//! **Appended and never edited**, as a plan's changes are, and folded by
//! [`GroupRuns::fold`]: a run's start and its end are two rows, so a Fleet
//! that stops between them reads the run back open.

use std::num::NonZeroU32;

use core_model::{
    Attempt, EscalationTrigger, GroupId, GroupMove, GroupRuns, JobId, StepId, StepLevelTrigger,
    StepState, StepVerdict, Timestamp,
};

use crate::error::{fault, LoadJobError, RowError, WriteError};
use crate::open::Store;
use crate::row::{maybe, maybe_number, string};

const RUNS: &str = "job_group_runs";

/// Version 94 — a plan's groups, a task that failed, and each group's runs.
///
/// One version for four tables, because each is half of one fact: the plan's
/// changes gain two moves and a failed task's reason, a recorded task the
/// planner's group number, `job_step_checks` the group and its run in its key
/// (so two groups gated on one run of a step keep both), and `job_group_runs`
/// is new. Rebuilt the way [`V90`](crate::work_plan::V90) is. Nothing is
/// backfilled: a plan recorded before this reads as one group.
pub(crate) const V94: &str = r#"
CREATE TABLE job_work_plan_changes_wide (
    job_id     TEXT NOT NULL REFERENCES jobs(job_id),
    seq        INTEGER NOT NULL CHECK (seq > 0),
    change     TEXT NOT NULL CHECK (change IN
                   ('recorded', 'added', 'updated', 'moved_task', 'moved_group')),
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
    PRIMARY KEY (job_id, seq),
    CHECK ((by_step IS NULL) = (by_attempt IS NULL)),
    CHECK (state IS NULL OR
           ((state IN ('dropped', 'failed')) = (reason IS NOT NULL AND trim(reason) <> '')))
) STRICT;

INSERT INTO job_work_plan_changes_wide (
    job_id, seq, change, by_step, by_attempt, at, approach, task_id, title, detail,
    after_task, state, reason, scope, expects, shown
) SELECT job_id, seq, change, by_step, by_attempt, at, approach, task_id, title, detail,
         after_task, state, reason, scope, expects, shown
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

ALTER TABLE job_work_plan_tasks ADD COLUMN grp INTEGER CHECK (grp IS NULL OR grp > 0);

CREATE TABLE job_step_checks_wide (
    job_id              TEXT NOT NULL REFERENCES jobs(job_id),
    step_id             TEXT NOT NULL,
    attempt             INTEGER NOT NULL CHECK (attempt >= 1),
    grp                 INTEGER NOT NULL DEFAULT 0 CHECK (grp >= 0),
    group_run           INTEGER NOT NULL DEFAULT 0 CHECK (group_run >= 0),
    ordinal             INTEGER NOT NULL,
    name                TEXT NOT NULL,
    outcome             TEXT NOT NULL,
    expected            TEXT,
    produced            TEXT,
    ran_at              TEXT NOT NULL,
    output_path         TEXT,
    reused_from_dry_run TEXT,
    PRIMARY KEY (job_id, step_id, attempt, grp, ordinal),
    CHECK ((grp = 0) = (group_run = 0))
) STRICT;

INSERT INTO job_step_checks_wide (
    job_id, step_id, attempt, ordinal, name, outcome, expected, produced, ran_at,
    output_path, reused_from_dry_run
) SELECT job_id, step_id, attempt, ordinal, name, outcome, expected, produced, ran_at,
         output_path, reused_from_dry_run
  FROM job_step_checks;

DROP TABLE job_step_checks;
ALTER TABLE job_step_checks_wide RENAME TO job_step_checks;

CREATE TABLE job_group_runs (
    job_id       TEXT NOT NULL REFERENCES jobs(job_id),
    seq          INTEGER NOT NULL CHECK (seq > 0),
    grp          INTEGER NOT NULL CHECK (grp > 0),
    run          INTEGER NOT NULL CHECK (run > 0),
    kind         TEXT NOT NULL CHECK (kind IN ('started', 'ended')),
    step_id      TEXT,
    step_attempt INTEGER CHECK (step_attempt IS NULL OR step_attempt > 0),
    at           TEXT NOT NULL,
    verdict      TEXT CHECK (verdict IS NULL OR verdict IN ('passed', 'failed', 'not_reached')),
    trigger      TEXT,
    commit_sha   TEXT,
    event_seq    INTEGER CHECK (event_seq IS NULL OR event_seq > 0),
    PRIMARY KEY (job_id, seq),
    CHECK ((kind = 'started') = (step_id IS NOT NULL AND step_attempt IS NOT NULL)),
    CHECK ((kind = 'ended') = (verdict IS NOT NULL)),
    CHECK ((verdict = 'failed') = (trigger IS NOT NULL))
) STRICT;

CREATE TRIGGER job_group_runs_are_never_edited
BEFORE UPDATE ON job_group_runs
BEGIN
    SELECT RAISE(ABORT, 'a group run is never edited');
END;

CREATE TRIGGER job_group_runs_are_never_removed_from_a_job_that_exists
BEFORE DELETE ON job_group_runs
WHEN EXISTS (SELECT 1 FROM jobs WHERE jobs.job_id = OLD.job_id)
BEGIN
    SELECT RAISE(ABORT, 'a group run is never removed from a Job that exists');
END;
"#;

/// The Record row a group's verdict made, and which run of which group it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupCoord {
    /// `job_events.seq` of the step move the verdict made.
    pub event_seq: i64,
    pub group: GroupId,
    pub run: u32,
}

impl Store {
    /// Append one row of a group's runs. `event_seq` is the step move the
    /// verdict made, where it made one: a round's `retrying`, or a stop.
    pub fn record_group_move(
        &mut self,
        job_id: &JobId,
        moved: &GroupMove,
        event_seq: Option<i64>,
    ) -> Result<(), WriteError> {
        let seq: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(seq), 0) + 1 FROM job_group_runs WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| row.get(0),
            )
            .map_err(fault("numbering a group run"))
            .map_err(WriteError::Database)?;
        let (group, run, kind, step, step_attempt, at, verdict, trigger, commit) = match moved {
            GroupMove::Started {
                group,
                run,
                step,
                step_attempt,
                at,
            } => (
                group,
                run,
                "started",
                Some(step.as_str()),
                Some(step_attempt.number()),
                at,
                None,
                None,
                None,
            ),
            GroupMove::Ended {
                group,
                run,
                verdict,
                commit,
                at,
            } => (
                group,
                run,
                "ended",
                None,
                None,
                at,
                Some(verdict.as_wire()),
                match verdict {
                    StepVerdict::Failed(why) => Some(why.as_wire()),
                    _ => None,
                },
                commit.as_deref(),
            ),
        };
        self.conn
            .execute(
                "INSERT INTO job_group_runs (job_id, seq, grp, run, kind, step_id, step_attempt, \
                 at, verdict, trigger, commit_sha, event_seq) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                rusqlite::params![
                    job_id.as_str(),
                    seq,
                    group.number(),
                    run,
                    kind,
                    step,
                    step_attempt,
                    at.as_str(),
                    verdict,
                    trigger,
                    commit,
                    event_seq,
                ],
            )
            .map_err(fault("appending a group run"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// Every run of every group of a Job's plan, as the rows leave them.
    pub fn group_runs(&self, job_id: &JobId) -> Result<GroupRuns, LoadJobError> {
        let moves = self
            .collect(
                "SELECT grp, run, kind, step_id, step_attempt, at, verdict, trigger, commit_sha \
                 FROM job_group_runs WHERE job_id = ?1 ORDER BY seq",
                job_id,
                "reading a plan's group runs",
                group_move,
            )
            .map_err(LoadJobError::Unreadable)?;
        Ok(GroupRuns::fold(&moves))
    }

    /// The last move of this step into `state`: the Record row a group's
    /// verdict just made, which its end is stamped with.
    pub fn last_step_move_into(
        &self,
        job_id: &JobId,
        step: &StepId,
        state: StepState,
    ) -> Result<Option<i64>, LoadJobError> {
        self.conn
            .query_row(
                "SELECT max(seq) FROM job_events WHERE job_id = ?1 \
                 AND kind = 'step_transition' AND step_id = ?2 AND state_to = ?3",
                (job_id.as_str(), step.as_str(), state.as_wire()),
                |row| row.get(0),
            )
            .map_err(fault("reading the step's last move"))
            .map_err(|why| LoadJobError::Unreadable(RowError::Database(why)))
    }

    /// The Record rows a group's verdict made, each with its group and run.
    pub fn group_coords(&self, job_id: &JobId) -> Result<Vec<GroupCoord>, LoadJobError> {
        self.collect(
            "SELECT event_seq, grp, run FROM job_group_runs \
             WHERE job_id = ?1 AND event_seq IS NOT NULL ORDER BY seq",
            job_id,
            "reading the Record rows a group's runs made",
            |row| {
                Ok(GroupCoord {
                    event_seq: row
                        .get::<_, i64>("event_seq")
                        .map_err(crate::row::column(RUNS, "event_seq"))?,
                    group: group_of(row)?,
                    run: run_of(row)?,
                })
            },
        )
        .map_err(LoadJobError::Unreadable)
    }
}

fn malformed(column: &'static str, detail: &str) -> RowError {
    RowError::MalformedColumn {
        table: RUNS,
        column,
        detail: detail.to_string(),
    }
}

fn group_of(row: &rusqlite::Row<'_>) -> Result<GroupId, RowError> {
    maybe_number(row, "grp")?
        .and_then(NonZeroU32::new)
        .map(GroupId::numbered)
        .ok_or_else(|| malformed("grp", "a group is one-based"))
}

fn run_of(row: &rusqlite::Row<'_>) -> Result<u32, RowError> {
    maybe_number(row, "run")?
        .filter(|run| *run > 0)
        .ok_or_else(|| malformed("run", "a run is one-based"))
}

fn group_move(row: &rusqlite::Row<'_>) -> Result<GroupMove, RowError> {
    let group = group_of(row)?;
    let run = run_of(row)?;
    let at = Timestamp::from_rfc3339(string(row, "at")?);
    match string(row, "kind")?.as_str() {
        "started" => Ok(GroupMove::Started {
            group,
            run,
            step: StepId::new(
                maybe(row, "step_id")?
                    .ok_or_else(|| malformed("step_id", "a start names its step"))?,
            ),
            step_attempt: maybe_number(row, "step_attempt")?
                .and_then(Attempt::stored)
                .ok_or_else(|| malformed("step_attempt", "a start names the step's run"))?,
            at,
        }),
        "ended" => {
            let verdict = match maybe(row, "verdict")?.as_deref() {
                Some("passed") => StepVerdict::Passed,
                Some("not_reached") => StepVerdict::NotReached,
                Some("failed") => StepVerdict::Failed(
                    maybe(row, "trigger")?
                        .as_deref()
                        .and_then(EscalationTrigger::from_wire)
                        .and_then(StepLevelTrigger::of)
                        .ok_or_else(|| malformed("trigger", "a failed run names a step trigger"))?,
                ),
                _ => return Err(malformed("verdict", "an end carries a verdict")),
            };
            Ok(GroupMove::Ended {
                group,
                run,
                verdict,
                commit: maybe(row, "commit_sha")?,
                at,
            })
        }
        other => Err(malformed("kind", &format!("`{other}` is not a group move"))),
    }
}
