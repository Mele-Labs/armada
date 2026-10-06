//! Every run of a Check that reached an exit code or timed out, kept for
//! analysis later. Beside `check_timings`, never in place of it: that table
//! is the last few durations Fleet orders by, and this one is the history it
//! forgets.
//!
//! **Append-only, and pruned on write** to [`KEPT_DAYS`]. Nothing reads it
//! back here; the table is for whoever analyses it.

use std::time::Duration;

use core_model::{JobId, ManifestId, Timestamp};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// How long a row is kept, counted from the run being written.
const KEPT_DAYS: u32 = 90;

/// Version 109 — one row per run of a Check. No row points at `jobs`, so
/// forgetting a Job keeps its runs: the history outlives what it was about.
pub(crate) const V109: &str = r#"
CREATE TABLE check_runs (
    repository        TEXT NOT NULL,
    check_name        TEXT NOT NULL,
    job_id            TEXT,
    started_at        TEXT NOT NULL,
    outcome           TEXT NOT NULL CHECK (outcome IN ('passed', 'failed', 'timed_out')),
    narrowed          INTEGER NOT NULL CHECK (narrowed IN (0, 1)),
    narrowed_to       TEXT,
    queue_wait_ms     INTEGER NOT NULL CHECK (queue_wait_ms >= 0),
    took_ms           INTEGER NOT NULL CHECK (took_ms >= 0),
    tests_run         INTEGER,
    tests_failed      INTEGER,
    load_avg_at_start REAL,
    cores             INTEGER NOT NULL,
    width             INTEGER NOT NULL,
    places_held       INTEGER NOT NULL
) STRICT;
CREATE INDEX check_runs_by_check ON check_runs (repository, check_name, started_at);
"#;

/// How one run ended, as far as a Check's own expectation reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckOutcome {
    Passed,
    Failed,
    TimedOut,
}

impl CheckOutcome {
    fn as_str(self) -> &'static str {
        match self {
            CheckOutcome::Passed => "passed",
            CheckOutcome::Failed => "failed",
            CheckOutcome::TimedOut => "timed_out",
        }
    }
}

/// One run of one Check: what was cheaply known when it ended. The repository
/// and Job are the writer's to name, not the run's.
#[derive(Clone, Debug)]
pub struct CheckRun {
    pub check: String,
    pub started_at: Timestamp,
    pub outcome: CheckOutcome,
    /// The narrowed command it ran instead of the Check's own line.
    pub narrowed_to: Option<String>,
    /// From asking for a place to being given one.
    pub queue_wait: Duration,
    pub took: Duration,
    /// `None` until the runner's summary is read where the run ends.
    pub tests_run: Option<u32>,
    pub tests_failed: Option<u32>,
    pub load_avg_at_start: Option<f64>,
    pub cores: u32,
    pub width: u32,
    pub places_held: u32,
}

impl Store {
    /// Append one run, and drop what is older than [`KEPT_DAYS`] before it.
    pub fn append_check_run(
        &mut self,
        repository: &ManifestId,
        job: Option<&JobId>,
        run: &CheckRun,
    ) -> Result<(), WriteError> {
        let failed = |why| WriteError::Database(fault("appending a Check run to the history")(why));
        let millis = |took: Duration| i64::try_from(took.as_millis()).unwrap_or(i64::MAX);
        let kept = self.conn.transaction().map_err(failed)?;
        kept.execute(
            "INSERT INTO check_runs (repository, check_name, job_id, started_at, outcome,
                 narrowed, narrowed_to, queue_wait_ms, took_ms, tests_run, tests_failed,
                 load_avg_at_start, cores, width, places_held)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            rusqlite::params![
                repository.as_str(),
                run.check,
                job.map(|job| job.as_ulid().as_str()),
                run.started_at.as_str(),
                run.outcome.as_str(),
                run.narrowed_to.is_some(),
                run.narrowed_to,
                millis(run.queue_wait),
                millis(run.took),
                run.tests_run,
                run.tests_failed,
                run.load_avg_at_start,
                run.cores,
                run.width,
                run.places_held,
            ],
        )
        .map_err(failed)?;
        // Compared as text: every stamp is the same-shape UTC RFC3339.
        kept.execute(
            "DELETE FROM check_runs
             WHERE started_at < strftime('%Y-%m-%dT%H:%M:%SZ', ?1, ?2)",
            rusqlite::params![run.started_at.as_str(), format!("-{KEPT_DAYS} days")],
        )
        .map_err(failed)?;
        kept.commit().map_err(failed)
    }
}
