//! A Job's adversarial reviews of its plan, one a plan version, with their findings.
//! `docs/concepts/adversarial-review.md`, *Kept per subject and version*.
//!
//! **Strings and numbers, not the wire's types**: this crate does not know `ipc`. A finding's anchor is
//! `plan`, `group` or `task` beside the id it names, and its cites are one per line.

use rusqlite::{params, OptionalExtension, Row};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One review of one version of a Job's plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptAdversarialReview {
    /// The plan version it read.
    pub version: u32,
    /// `running`, `done` or `failed`.
    pub state: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    /// Why a failed review failed, in Fleet's words.
    pub failed: Option<String>,
    pub cost_micros: Option<u64>,
    pub findings: Vec<KeptAdversarialFinding>,
}

/// One finding of a review.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptAdversarialFinding {
    pub id: String,
    /// `plan`, `group` or `task`.
    pub anchor: String,
    /// The group's or task's id; empty for the plan.
    pub anchor_id: String,
    /// The words of the part it was pinned to as the review read them: the task's, or the group's title.
    /// Empty for the plan. What the agent is told the finding is about, whatever the plan says later.
    pub part: String,
    pub consequence: String,
    pub expected: String,
    pub found: String,
    pub cites: Vec<String>,
    /// `sent` or `dismissed`, and when. Absent while it waits.
    pub settled: Option<(String, String)>,
}

fn reading(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("reading an adversarial review")(why))
}

fn keeping(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("keeping an adversarial review")(why))
}

const REVIEW: &str = "version, state, started_at, finished_at, failed, cost_micros";
const FINDING: &str = "id, anchor, anchor_id, part, consequence, expected, found, cites, how, settled_at";

fn review(row: &Row<'_>) -> rusqlite::Result<KeptAdversarialReview> {
    Ok(KeptAdversarialReview {
        version: row.get(0)?,
        state: row.get(1)?,
        started_at: row.get(2)?,
        finished_at: row.get(3)?,
        failed: row.get(4)?,
        cost_micros: row.get::<_, Option<i64>>(5)?.and_then(|micros| micros.try_into().ok()),
        findings: Vec::new(),
    })
}

fn finding(row: &Row<'_>) -> rusqlite::Result<KeptAdversarialFinding> {
    let cites: String = row.get(7)?;
    let how: Option<String> = row.get(8)?;
    let at: Option<String> = row.get(9)?;
    Ok(KeptAdversarialFinding {
        id: row.get(0)?,
        anchor: row.get(1)?,
        anchor_id: row.get(2)?,
        part: row.get(3)?,
        consequence: row.get(4)?,
        expected: row.get(5)?,
        found: row.get(6)?,
        cites: cites.lines().filter(|line| !line.is_empty()).map(str::to_string).collect(),
        settled: how.zip(at),
    })
}

impl Store {
    /// Keep a review whole, replacing the one kept for its version, findings and all, in one transaction.
    pub fn keep_adversarial_review(&mut self, session_id: &str, kept: &KeptAdversarialReview) -> Result<(), WriteError> {
        let held = self.conn.transaction().map_err(keeping)?;
        for statement in [
            "DELETE FROM adversarial_reviews WHERE session_id = ?1 AND version = ?2",
            "DELETE FROM adversarial_findings WHERE session_id = ?1 AND version = ?2",
        ] {
            held.execute(statement, params![session_id, kept.version]).map_err(keeping)?;
        }
        held.execute(
            &format!("INSERT INTO adversarial_reviews (session_id, {REVIEW}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"),
            params![
                session_id,
                kept.version,
                kept.state,
                kept.started_at,
                kept.finished_at,
                kept.failed,
                kept.cost_micros.and_then(|micros| i64::try_from(micros).ok()),
            ],
        )
        .map_err(keeping)?;
        for (position, one) in kept.findings.iter().enumerate() {
            held.execute(
                &format!(
                    "INSERT INTO adversarial_findings (session_id, version, position, {FINDING})
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)"
                ),
                params![
                    session_id,
                    kept.version,
                    position as i64,
                    one.id,
                    one.anchor,
                    one.anchor_id,
                    one.part,
                    one.consequence,
                    one.expected,
                    one.found,
                    one.cites.join("\n"),
                    one.settled.as_ref().map(|(how, _)| how.as_str()),
                    one.settled.as_ref().map(|(_, at)| at.as_str()),
                ],
            )
            .map_err(keeping)?;
        }
        held.commit().map_err(keeping)
    }

    /// The review kept for one version of the Job's plan.
    pub fn adversarial_review(&self, session_id: &str, version: u32) -> Result<Option<KeptAdversarialReview>, WriteError> {
        self.reviewed(session_id, "AND version = ?2", Some(version))
    }

    /// The review of the latest version that has one, which is the one the Job's record carries.
    pub fn latest_adversarial_review(&self, session_id: &str) -> Result<Option<KeptAdversarialReview>, WriteError> {
        self.reviewed(session_id, "", None)
    }

    fn reviewed(&self, session_id: &str, and: &str, version: Option<u32>) -> Result<Option<KeptAdversarialReview>, WriteError> {
        let sql = format!(
            "SELECT {REVIEW} FROM adversarial_reviews WHERE session_id = ?1 {and} ORDER BY version DESC LIMIT 1"
        );
        let found = match version {
            Some(version) => self.conn.query_row(&sql, params![session_id, version], review),
            None => self.conn.query_row(&sql, params![session_id], review),
        };
        let Some(mut kept) = found.optional().map_err(reading)? else {
            return Ok(None);
        };
        let mut read = self
            .conn
            .prepare(&format!(
                "SELECT {FINDING} FROM adversarial_findings WHERE session_id = ?1 AND version = ?2 ORDER BY position"
            ))
            .map_err(reading)?;
        kept.findings = read
            .query_map(params![session_id, kept.version], finding)
            .map_err(reading)?
            .collect::<Result<_, _>>()
            .map_err(reading)?;
        Ok(Some(kept))
    }

    /// Settle finding `id` of the version's review as `how` (`sent` or `dismissed`) at `at`, if it waits.
    /// Whether it did.
    pub fn settle_adversarial_finding(
        &mut self,
        session_id: &str,
        version: u32,
        id: &str,
        how: &str,
        at: &str,
    ) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE adversarial_findings SET how = ?4, settled_at = ?5
                 WHERE session_id = ?1 AND version = ?2 AND id = ?3 AND how IS NULL",
                params![session_id, version, id, how, at],
            )
            .map(|changed| changed == 1)
            .map_err(keeping)
    }
}
