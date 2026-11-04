//! Where each Job's time went, kept for good, and the reads the split is made from. Fleet's fold
//! is `job_time.rs`; `docs/concepts/session.md`.
//!
//! **Strings, as in `timeline.rs`**: this crate does not know the fold, so a stamp is the RFC3339
//! text it was kept as and a figure is milliseconds.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::fleet_builds::{write_parts, KeptBuildSplit, KeptPullRequestWait};
use crate::open::Store;

/// One Job's split. The figures sum to `ended_at` less `began_at`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptJobTime {
    pub session_id: String,
    pub repository: String,
    pub workflow: String,
    pub began_at: String,
    pub ended_at: String,
    pub model_ms: i64,
    pub tools_ms: i64,
    pub checks_ms: i64,
    pub subagents_ms: i64,
    pub slot_ms: i64,
    pub you_ms: i64,
    pub pull_request_ms: i64,
    pub stopped_ms: i64,
    pub idle_ms: i64,
    pub tool_times_missing: bool,
    pub model_starts_guessed: bool,
    pub timeline_cut: bool,
    pub kept_at: String,
    /// The reading of the fold that made the row (`job_time::FOLD`).
    pub fold: i64,
    /// The split cut at each build, summing to the figures above, and each pull request's wait.
    /// Kept with the row and replaced with it.
    pub builds: Vec<KeptBuildSplit>,
    pub waits: Vec<KeptPullRequestWait>,
}

/// A turn as the split reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptTurnTimed {
    pub at: String,
    /// The names of the tools it called, in order.
    pub tools: Vec<String>,
    pub model_began: Option<String>,
    pub tools_began: Option<String>,
    pub tools_ended: Option<String>,
}

/// A run of a Check, as the split reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptCheckSpan {
    pub started_at: String,
    pub ended_at: Option<String>,
    pub took_ms: Option<u64>,
}

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

const COLUMNS: &str = "session_id, repository, workflow, began_at, ended_at, model_ms, tools_ms, checks_ms, \
     subagents_ms, slot_ms, you_ms, pull_request_ms, stopped_ms, idle_ms, tool_times_missing, \
     model_starts_guessed, timeline_cut, kept_at, fold";

impl Store {
    /// Keep a Job's split, replacing the one it had, with its parts in the same transaction.
    pub fn keep_job_time(&mut self, time: &KeptJobTime) -> Result<(), WriteError> {
        let doing = "keeping where a job's time went";
        let tx = self.conn.transaction().map_err(failed(doing))?;
        tx.execute(
            &format!("INSERT OR REPLACE INTO job_times ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)"),
            params![
                time.session_id,
                time.repository,
                time.workflow,
                time.began_at,
                time.ended_at,
                time.model_ms,
                time.tools_ms,
                time.checks_ms,
                time.subagents_ms,
                time.slot_ms,
                time.you_ms,
                time.pull_request_ms,
                time.stopped_ms,
                time.idle_ms,
                time.tool_times_missing,
                time.model_starts_guessed,
                time.timeline_cut,
                time.kept_at,
                time.fold,
            ],
        )
        .map_err(failed(doing))?;
        write_parts(&tx, &time.session_id, &time.builds, &time.waits).map_err(failed(doing))?;
        tx.commit().map_err(failed(doing))
    }

    /// A Job's split, if one is kept.
    pub fn job_time(&self, session_id: &str) -> Result<Option<KeptJobTime>, WriteError> {
        let kept = self.conn
            .query_row(&format!("SELECT {COLUMNS} FROM job_times WHERE session_id = ?1"), [session_id], |row| {
                Ok(KeptJobTime {
                    session_id: row.get(0)?,
                    repository: row.get(1)?,
                    workflow: row.get(2)?,
                    began_at: row.get(3)?,
                    ended_at: row.get(4)?,
                    model_ms: row.get(5)?,
                    tools_ms: row.get(6)?,
                    checks_ms: row.get(7)?,
                    subagents_ms: row.get(8)?,
                    slot_ms: row.get(9)?,
                    you_ms: row.get(10)?,
                    pull_request_ms: row.get(11)?,
                    stopped_ms: row.get(12)?,
                    idle_ms: row.get(13)?,
                    tool_times_missing: row.get(14)?,
                    model_starts_guessed: row.get(15)?,
                    timeline_cut: row.get(16)?,
                    kept_at: row.get(17)?,
                    fold: row.get(18)?,
                    builds: Vec::new(),
                    waits: Vec::new(),
                })
            })
            .optional()
            .map_err(failed("reading where a job's time went"))?;
        let Some(mut kept) = kept else {
            return Ok(None);
        };
        kept.builds = self.job_time_builds_of(session_id)?;
        kept.waits = self.job_time_waits_of(session_id)?;
        Ok(Some(kept))
    }

    /// The most recently ended Job with no split, or whose split was made for an earlier end (one
    /// that was reopened and ended again) or by a reading of the fold before `fold`.
    pub fn job_awaiting_time(&self, fold: i64) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row(
                "SELECT s.id FROM sessions s LEFT JOIN job_times j ON j.session_id = s.id
                 WHERE s.state = 'ended'
                   AND (j.session_id IS NULL OR j.ended_at <> COALESCE(s.ended_at, s.last_seen_at) OR j.fold < ?1)
                 ORDER BY COALESCE(s.ended_at, s.last_seen_at) DESC LIMIT 1",
                [fold],
                |row| row.get(0),
            )
            .optional()
            .map_err(failed("finding a job whose time is not split"))
    }

    /// A session's turns for the split, oldest first, the main thread's only.
    pub fn turns_timed(&self, session_id: &str) -> Result<Vec<KeptTurnTimed>, WriteError> {
        let doing = "reading when a job's turns ran";
        let mut statement = self
            .conn
            .prepare(
                "SELECT at, tools, model_began, tools_began, tools_ended FROM session_turn_usage
                 WHERE session_id = ?1 AND subagent = 0 ORDER BY at, message_id",
            )
            .map_err(failed(doing))?;
        statement
            .query_map([session_id], |row| {
                Ok(KeptTurnTimed {
                    at: row.get(0)?,
                    tools: serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or_default(),
                    model_began: row.get(2)?,
                    tools_began: row.get(3)?,
                    tools_ended: row.get(4)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(failed(doing))
    }

    /// Whether a main-thread turn of the session is missing the stamps a read of its transcript
    /// from the top would give it.
    pub fn turns_lack_times(&self, session_id: &str) -> Result<bool, WriteError> {
        self.conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM session_turn_usage WHERE session_id = ?1 AND subagent = 0
                                AND (model_began IS NULL OR (tools <> '[]' AND tools_began IS NULL)))",
                [session_id],
                |row| row.get(0),
            )
            .map_err(failed("checking a job's turns for their times"))
    }

    /// Every run of a Check the session made, oldest first.
    pub fn check_spans_of(&self, session_id: &str) -> Result<Vec<KeptCheckSpan>, WriteError> {
        let doing = "reading when a job's Checks ran";
        let mut statement = self
            .conn
            .prepare("SELECT started_at, ended_at, took_ms FROM session_check_runs WHERE session_id = ?1 ORDER BY id")
            .map_err(failed(doing))?;
        statement
            .query_map([session_id], |row| {
                Ok(KeptCheckSpan {
                    started_at: row.get(0)?,
                    ended_at: row.get(1)?,
                    took_ms: row.get::<_, Option<i64>>(2)?.map(|ms| ms as u64),
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(failed(doing))
    }

    /// When each of the Session's timeline entries of these `kinds` was written, oldest first.
    pub fn timeline_stamps(&self, session_id: &str, kinds: &[&str]) -> Result<Vec<String>, WriteError> {
        let doing = "reading what the timeline kept of a job";
        let marks = vec!["?"; kinds.len()].join(", ");
        let mut statement = self
            .conn
            .prepare(&format!(
                "SELECT at FROM timeline_events WHERE subject_kind = 'session' AND subject_id = ? AND kind IN ({marks}) ORDER BY at"
            ))
            .map_err(failed(doing))?;
        let arguments = std::iter::once(session_id).chain(kinds.iter().copied());
        statement
            .query_map(rusqlite::params_from_iter(arguments), |row| row.get(0))
            .and_then(|rows| rows.collect::<Result<Vec<String>, _>>())
            .map_err(failed(doing))
    }

    /// The oldest entry the timeline still holds. Older ones were pruned.
    pub fn timeline_oldest(&self) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row("SELECT MIN(at) FROM timeline_events", [], |row| row.get(0))
            .map_err(failed("reading how far back the timeline goes"))
    }
}
