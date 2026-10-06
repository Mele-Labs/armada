//! What a step's worktree held when the step began, kept for every later entry
//! into the same step.
//!
//! **Why it is a row.** `diff_nonempty` asks whether the worktree moved while
//! the step ran. A Drone that died after writing, and a step put back to work
//! (a requeue, a retry, a Fleet that restarted), has to be asked against the
//! same start, or the work it already did reads as inherited and the step can
//! never pass. See `fleet::dispatch::Fleet::marked`.
//!
//! **One row per step.** The row is dropped when the step advances, so a step a
//! later one sends work back to starts from what the worktree holds then.

use adapter_traits::Footprint;
use core_model::{JobId, StepId};

use crate::error::{fault, LoadJobError, RowError, WriteError};
use crate::open::Store;

const TABLE: &str = "job_step_baselines";
const FOREIGN_KEY_VIOLATION: i32 = 787;

/// Version 115 — a step's entry baseline. Nothing is backfilled: a step that
/// began before this reads as one Fleet never saw start, which fails closed.
pub(crate) const V115: &str = r#"
CREATE TABLE job_step_baselines (
    job_id  TEXT NOT NULL REFERENCES jobs(job_id),
    step_id TEXT NOT NULL,
    entries TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id)
) STRICT;
"#;

impl Store {
    /// Write the baseline down, replacing one already kept for this step.
    pub fn keep_step_baseline(
        &mut self,
        job_id: &JobId,
        step: &StepId,
        baseline: &Footprint,
    ) -> Result<(), WriteError> {
        // A list of string pairs always encodes; the fallback is an empty list
        // rather than a panic, and reads back as "nothing held".
        let entries = serde_json::to_string(baseline.entries()).unwrap_or_else(|_| "[]".into());
        self.conn
            .execute(
                "INSERT INTO job_step_baselines (job_id, step_id, entries) \
                 VALUES (?1, ?2, ?3) \
                 ON CONFLICT (job_id, step_id) DO UPDATE SET entries = excluded.entries",
                (job_id.as_str(), step.as_str(), entries),
            )
            .map_err(|why| match why {
                rusqlite::Error::SqliteFailure(err, _)
                    if err.extended_code == FOREIGN_KEY_VIOLATION =>
                {
                    WriteError::NoSuchJob {
                        job_id: job_id.clone(),
                    }
                }
                other => WriteError::Database(fault("keeping a step baseline")(other)),
            })?;
        Ok(())
    }

    /// The baseline kept for this step, or `None` where none was.
    ///
    /// **`None` is "never seen", and an empty footprint is an answer**: a
    /// worktree that held nothing when the step began is `Some` of an empty one.
    pub fn step_baseline(
        &self,
        job_id: &JobId,
        step: &StepId,
    ) -> Result<Option<Footprint>, LoadJobError> {
        let kept = self.conn.query_row(
            "SELECT entries FROM job_step_baselines WHERE job_id = ?1 AND step_id = ?2",
            (job_id.as_str(), step.as_str()),
            |row| row.get::<_, String>("entries"),
        );
        let text = match kept {
            Ok(text) => text,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(other) => {
                return Err(LoadJobError::Database(fault("reading a step baseline")(
                    other,
                )))
            }
        };
        let entries: Vec<(String, String)> = serde_json::from_str(&text).map_err(|why| {
            LoadJobError::Unreadable(RowError::MalformedColumn {
                table: TABLE,
                column: "entries",
                detail: why.to_string(),
            })
        })?;
        Ok(Some(Footprint::of(entries)))
    }

    /// The step advanced, so the next entry into it is new work. **A step with
    /// no row is `Ok`**, which is every step that advanced before this existed.
    pub fn forget_step_baseline(
        &mut self,
        job_id: &JobId,
        step: &StepId,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "DELETE FROM job_step_baselines WHERE job_id = ?1 AND step_id = ?2",
                (job_id.as_str(), step.as_str()),
            )
            .map_err(fault("forgetting a step baseline"))
            .map_err(WriteError::Database)?;
        Ok(())
    }
}
