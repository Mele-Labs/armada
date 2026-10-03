//! What a person decides about a Job at its approval gate, kept beside it, and
//! the issue its words came from. Spike 022, slice 4.
//!
//! **Settings beside the Job, not columns of it**, `model_per_task`'s shape:
//! each is set after creation, read where it is used, and a row that is not
//! there is the Job doing what every Job did before V98. Nothing here folds.
//!
//! **The edit itself does rewrite the Job's columns** — its title, facts,
//! workflow, criteria and step rows — and only at `awaiting_approval`, the one
//! status at which what a Job is held to may still move (#1581).

use core_model::{
    AutoMerge, Branch, CompleteWhen, IssueSource, Job, JobId, JobStatus, Landing, PolicyOverrides,
    PrMode, ReviewGate, Timestamp,
};

use crate::columns;
use crate::error::{fault, LoadJobError, WriteError};
use crate::open::Store;
use crate::write::write_steps;

/// Version 98 — a Job's landing, its Drone cap, its policy overrides, and the
/// issue its request linked.
///
/// One row per Job in each, keyed by the Job. Nothing is backfilled: a Job
/// with no row is a Job approved before this, which lands as every Job did
/// then, runs under the machine's cap, defers to the repository at every gate
/// and came from no issue anybody recorded.
pub(crate) const V98: &str = r#"
CREATE TABLE job_landing (
    job_id   TEXT PRIMARY KEY REFERENCES jobs(job_id),
    target   TEXT CHECK (target IS NULL OR trim(target) <> ''),
    from_ref TEXT CHECK (from_ref IS NULL OR trim(from_ref) <> ''),
    pr_mode  TEXT NOT NULL CHECK (pr_mode IN ('ready', 'draft'))
) STRICT;

CREATE TABLE job_drone_caps (
    job_id TEXT PRIMARY KEY REFERENCES jobs(job_id),
    cap    INTEGER NOT NULL CHECK (cap > 0)
) STRICT;

CREATE TABLE job_policy_overrides (
    job_id TEXT NOT NULL REFERENCES jobs(job_id),
    policy TEXT NOT NULL CHECK (policy IN ('auto_merge', 'review_gate')),
    said   TEXT NOT NULL CHECK (trim(said) <> ''),
    PRIMARY KEY (job_id, policy)
) STRICT;

CREATE TABLE job_issue_sources (
    job_id    TEXT PRIMARY KEY REFERENCES jobs(job_id),
    reference TEXT NOT NULL CHECK (trim(reference) <> ''),
    url       TEXT NOT NULL CHECK (trim(url) <> ''),
    read_at   TEXT NOT NULL,
    moved_at  TEXT
) STRICT;
"#;

impl Store {
    /// Rewrite what a person edited on a Job at its approval gate, and its step
    /// rows, in one transaction. **Compare-and-swap on `awaiting_approval`**,
    /// for `record_answered`'s reason: an approval landing as the edit arrives
    /// must leave one of the two, never an edit to a Job already released.
    pub fn record_proposal_edit(&mut self, job: &Job) -> Result<(), WriteError> {
        let workflow = job
            .frozen_workflow()
            .ok_or_else(|| WriteError::NotAnAnswer {
                job_id: job.id().clone(),
            })?;
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting a proposal edit"))
            .map_err(WriteError::Database)?;
        let updated = tx
            .execute(
                "UPDATE jobs SET title = ?2, facts = ?3, workflow_id = ?4, workflow = ?5,
                     acceptance_criteria = ?6
                 WHERE job_id = ?1 AND status = ?7",
                rusqlite::params![
                    job.id().as_str(),
                    job.title().as_str(),
                    job.facts().as_str(),
                    job.workflow_id().as_str(),
                    columns::write_workflow(workflow),
                    columns::write_acceptance_criteria(job.acceptance_criteria()),
                    JobStatus::AwaitingApproval.as_wire(),
                ],
            )
            .map_err(fault("writing a proposal edit"))
            .map_err(WriteError::Database)?;
        if updated == 0 {
            return Err(WriteError::NotAtApproval {
                job_id: job.id().clone(),
            });
        }
        tx.execute(
            "DELETE FROM job_steps WHERE job_id = ?1",
            (job.id().as_str(),),
        )
        .map_err(fault("clearing the step rows an edit replaces"))
        .map_err(WriteError::Database)?;
        write_steps(&tx, job)?;
        tx.commit()
            .map_err(fault("committing a proposal edit"))
            .map_err(WriteError::Database)
    }

    /// Keep how this Job lands, replacing what was kept.
    pub fn set_landing(&mut self, job_id: &JobId, landing: &Landing) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO job_landing (job_id, target, from_ref, pr_mode, complete_when)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (job_id) DO UPDATE SET
                     target = excluded.target, from_ref = excluded.from_ref,
                     pr_mode = excluded.pr_mode, complete_when = excluded.complete_when",
                rusqlite::params![
                    job_id.as_str(),
                    landing.target.as_ref().map(Branch::as_str),
                    landing.from_ref.as_ref().map(Branch::as_str),
                    landing.pr_mode.as_wire(),
                    landing.complete_when.as_wire(),
                ],
            )
            .map_err(fault("keeping how a job lands"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// How this Job lands, or `None` where nothing was kept.
    pub fn landing(&self, job_id: &JobId) -> Result<Option<Landing>, LoadJobError> {
        let reading = "reading how a job lands";
        let row = self
            .conn
            .query_row(
                "SELECT target, from_ref, pr_mode, complete_when FROM job_landing
                 WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                    ))
                },
            )
            .map(Some)
            .or_else(|cause| match cause {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                cause => Err(cause),
            })
            .map_err(fault(reading))
            .map_err(LoadJobError::Database)?;
        Ok(row.map(|(target, from_ref, mode, when)| Landing {
            target: core_model::branch_named(target.as_deref()),
            from_ref: core_model::branch_named(from_ref.as_deref()),
            // The table's check admits nothing else.
            pr_mode: PrMode::from_wire(&mode).unwrap_or_default(),
            // Null on a row kept before V101, which completed when delivered.
            complete_when: when
                .as_deref()
                .and_then(CompleteWhen::from_wire)
                .unwrap_or_default(),
        }))
    }

    /// Keep this Job's Drone cap, or clear it where `None`.
    pub fn set_drone_cap(&mut self, job_id: &JobId, cap: Option<u32>) -> Result<(), WriteError> {
        let written = match cap {
            Some(cap) => self.conn.execute(
                "INSERT INTO job_drone_caps (job_id, cap) VALUES (?1, ?2)
                 ON CONFLICT (job_id) DO UPDATE SET cap = excluded.cap",
                rusqlite::params![job_id.as_str(), cap],
            ),
            None => self.conn.execute(
                "DELETE FROM job_drone_caps WHERE job_id = ?1",
                (job_id.as_str(),),
            ),
        };
        written
            .map_err(fault("keeping a job's drone cap"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// This Job's Drone cap, or `None` where the machine's holds.
    pub fn drone_cap(&self, job_id: &JobId) -> Result<Option<u32>, LoadJobError> {
        self.conn
            .query_row(
                "SELECT cap FROM job_drone_caps WHERE job_id = ?1",
                (job_id.as_str(),),
                |row| row.get::<_, u32>(0),
            )
            .map(Some)
            .or_else(|cause| match cause {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                cause => Err(cause),
            })
            .map_err(fault("reading a job's drone cap"))
            .map_err(LoadJobError::Database)
    }

    /// Replace what this Job's approval said in place of the repository.
    pub fn set_policy_overrides(
        &mut self,
        job_id: &JobId,
        overrides: &PolicyOverrides,
    ) -> Result<(), WriteError> {
        let tx = self
            .conn
            .transaction()
            .map_err(fault("starting a policy override"))
            .map_err(WriteError::Database)?;
        tx.execute(
            "DELETE FROM job_policy_overrides WHERE job_id = ?1",
            (job_id.as_str(),),
        )
        .map_err(fault("clearing a job's policy overrides"))
        .map_err(WriteError::Database)?;
        let said = [
            ("auto_merge", overrides.auto_merge.map(|p| p.as_written())),
            ("review_gate", overrides.review_gate.map(|p| p.as_written())),
        ];
        for (policy, said) in said {
            let Some(said) = said else { continue };
            tx.execute(
                "INSERT INTO job_policy_overrides (job_id, policy, said) VALUES (?1, ?2, ?3)",
                (job_id.as_str(), policy, said),
            )
            .map_err(fault("keeping a job's policy override"))
            .map_err(WriteError::Database)?;
        }
        tx.commit()
            .map_err(fault("committing a job's policy overrides"))
            .map_err(WriteError::Database)
    }

    /// What this Job's approval said in place of the repository. **Empty where
    /// nothing was kept**, which is the repository deciding at every gate.
    pub fn policy_overrides(&self, job_id: &JobId) -> Result<PolicyOverrides, LoadJobError> {
        let reading = "reading a job's policy overrides";
        let mut asked = self
            .conn
            .prepare("SELECT policy, said FROM job_policy_overrides WHERE job_id = ?1")
            .map_err(fault(reading))
            .map_err(LoadJobError::Database)?;
        let rows = asked
            .query_map((job_id.as_str(),), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(fault(reading))
            .map_err(LoadJobError::Database)?;
        let mut overrides = PolicyOverrides::none();
        for row in rows {
            let (policy, said) = row
                .map_err(fault(reading))
                .map_err(LoadJobError::Database)?;
            // A word no build spells is skipped rather than failing every gate
            // of the Job: the repository decides that policy, as before.
            match policy.as_str() {
                "auto_merge" => overrides.auto_merge = AutoMerge::from_written(&said),
                "review_gate" => overrides.review_gate = ReviewGate::from_written(&said),
                _ => {}
            }
        }
        Ok(overrides)
    }

    /// Keep the issue a Job's request linked, as Fleet read it.
    pub fn record_issue_source(
        &mut self,
        job_id: &JobId,
        source: &IssueSource,
    ) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO job_issue_sources (job_id, reference, url, read_at, moved_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (job_id) DO UPDATE SET
                     reference = excluded.reference, url = excluded.url,
                     read_at = excluded.read_at, moved_at = excluded.moved_at",
                rusqlite::params![
                    job_id.as_str(),
                    source.reference,
                    source.url,
                    source.read_at.as_str(),
                    source.moved_at.as_ref().map(Timestamp::as_str),
                ],
            )
            .map_err(fault("keeping the issue a job came from"))
            .map_err(WriteError::Database)?;
        Ok(())
    }

    /// The issue a Job's request linked, where one was kept.
    pub fn issue_source(&self, job_id: &JobId) -> Result<Option<IssueSource>, LoadJobError> {
        Ok(self
            .issue_sources(Some(job_id))?
            .into_iter()
            .next()
            .map(|(_, source)| source))
    }

    /// Every Job still in flight whose request linked an issue, oldest first:
    /// **the set the issue rotation walks** (spike 022, answer 5). A finished
    /// Job leaves it, as a settled pull request leaves its own.
    pub fn issue_sources_in_flight(&self) -> Result<Vec<(JobId, IssueSource)>, LoadJobError> {
        self.issue_sources(None)
    }

    fn issue_sources(
        &self,
        one: Option<&JobId>,
    ) -> Result<Vec<(JobId, IssueSource)>, LoadJobError> {
        let reading = "reading the issues jobs came from";
        let mut asked = self
            .conn
            .prepare(
                "SELECT s.job_id, s.reference, s.url, s.read_at, s.moved_at, j.status
                 FROM job_issue_sources AS s JOIN jobs AS j ON j.job_id = s.job_id
                 WHERE ?1 IS NULL OR s.job_id = ?1
                 ORDER BY j.created_at, s.job_id",
            )
            .map_err(fault(reading))
            .map_err(LoadJobError::Database)?;
        let rows = asked
            .query_map((one.map(JobId::as_str),), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .map_err(fault(reading))
            .map_err(LoadJobError::Database)?;
        let mut sources = Vec::new();
        for row in rows {
            let (job, reference, url, read_at, moved_at, status) = row
                .map_err(fault(reading))
                .map_err(LoadJobError::Database)?;
            let finished = JobStatus::from_wire(&status).is_some_and(|s| s.is_terminal());
            if one.is_none() && finished {
                continue;
            }
            sources.push((
                JobId::carried(core_model::Ulid::carried(job)),
                IssueSource {
                    reference,
                    url,
                    read_at: Timestamp::from_rfc3339(read_at),
                    moved_at: moved_at.map(Timestamp::from_rfc3339),
                },
            ));
        }
        Ok(sources)
    }
}
