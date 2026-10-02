//! What a gate's two policies resolved to, kept on the run that passed it.
//!
//! `auto_merge` and `review_gate` are both `Live`, and `fleet::policy` folds
//! them fresh at every gate. Until this table the answer lasted one decision,
//! so the Record could not say why a step waited. #1683.
//!
//! **One row per run, keyed like every per-attempt table**, so a step's second
//! run under a changed policy sits beside its first rather than over it. The
//! run is counted inside the writing transaction, for
//! [`attempt_now`](crate::attempt::attempt_now)'s reason.
//!
//! **Every run a gate reached has one since V88**, the owner's decision of 2 Oct
//! 2026: a run its Checks, its Judge or its gaming check stopped keeps what the
//! rules said too, and `decided` says it never reached the rule.
//!
//! **No row is the answer for every run before this existed**, and it reads as
//! absent rather than as either policy's default. Nothing is backfilled, by V5's
//! rule: what those gates resolved to was never observed.

use core_model::{AutoMerge, JobId, ResolvedPolicies, ReviewGate, StepId, Timestamp};

use crate::attempt::{attempt_now, coordinate, Attempted};
use crate::error::{fault, LoadJobError, WriteError};
use crate::open::Store;
use crate::row::{column, enum_value, string};

/// Version 87 — the policies each gate resolved to, per run.
///
/// Its own table rather than columns on another: no table holds one row per
/// run. `job_step_evidence` is the nearest, and a gate can pass on a run whose
/// evidence row a later submission in that same run replaced.
pub(crate) const V87: &str = r#"
CREATE TABLE job_step_policies (
    job_id      TEXT NOT NULL REFERENCES jobs(job_id),
    step_id     TEXT NOT NULL,
    attempt     INTEGER NOT NULL CHECK (attempt >= 1),
    auto_merge  TEXT NOT NULL,
    review_gate TEXT NOT NULL,
    resolved_at TEXT NOT NULL,
    PRIMARY KEY (job_id, step_id, attempt)
) STRICT;
"#;

/// Version 88 — whether the run reached the advance gate, where the rule
/// decides. Rows are now written on a run an earlier gate stopped too, and
/// they say `0`.
///
/// **Every row V87 wrote reads `1`, and that is a fact rather than a
/// default.** V87 was written only by a ruling that read the advance gate,
/// held or advanced, so every row already in the table is one the rule
/// decided. The column default is what says so for them, and every write since
/// names the value.
pub(crate) const V88: &str = r#"
ALTER TABLE job_step_policies
    ADD COLUMN decided INTEGER NOT NULL DEFAULT 1 CHECK (decided IN (0, 1));
"#;

impl Store {
    /// Keep what both policies resolved to on the run of `step_id` in
    /// progress, and whether the rule decided. A second gate on the same run, which a re-run of its Checks
    /// is, replaces the first: the later answer is the one the run ended on.
    pub fn record_resolved_policies(
        &mut self,
        job_id: &JobId,
        step_id: &StepId,
        resolved: ResolvedPolicies,
        at: &Timestamp,
    ) -> Result<(), WriteError> {
        let failed = |doing| move |why| WriteError::Database(fault(doing)(why));
        let tx = self
            .conn
            .transaction()
            .map_err(failed("starting the resolved-policy record"))?;
        let attempt = attempt_now(&tx, job_id, step_id).map_err(WriteError::Database)?;
        tx.execute(
            "INSERT OR REPLACE INTO job_step_policies
                 (job_id, step_id, attempt, auto_merge, review_gate, resolved_at, decided)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                job_id.as_str(),
                step_id.as_str(),
                attempt.number(),
                resolved.auto_merge.as_written(),
                resolved.review_gate.as_written(),
                at.as_str(),
                resolved.decided,
            ],
        )
        .map_err(failed("writing what a gate's policies resolved to"))?;
        tx.commit()
            .map_err(failed("committing the resolved-policy record"))
    }

    /// What each run of each step resolved its policies to, oldest run first.
    /// A run with no row reached no gate, or reached one before this was kept.
    pub fn resolved_policies_every_attempt(
        &self,
        job_id: &JobId,
    ) -> Result<Vec<Attempted<ResolvedPolicies>>, LoadJobError> {
        self.collect(
            "SELECT step_id, attempt, resolved_at, auto_merge, review_gate, decided
             FROM job_step_policies WHERE job_id = ?1 ORDER BY step_id, attempt",
            job_id,
            "reading resolved policies",
            |row| {
                let (step_id, attempt, at) = coordinate(row, "job_step_policies", "resolved_at")?;
                let auto_merge = string(row, "auto_merge")?;
                let review_gate = string(row, "review_gate")?;
                let decided: bool = row
                    .get("decided")
                    .map_err(column("job_step_policies", "decided"))?;
                Ok(Attempted {
                    step_id,
                    attempt,
                    at,
                    record: ResolvedPolicies {
                        auto_merge: enum_value(
                            AutoMerge::from_written,
                            "job_step_policies",
                            "auto_merge",
                            &auto_merge,
                        )?,
                        review_gate: enum_value(
                            ReviewGate::from_written,
                            "job_step_policies",
                            "review_gate",
                            &review_gate,
                        )?,
                        decided,
                    },
                })
            },
        )
        .map_err(LoadJobError::Unreadable)
    }
}
