//! Every build Fleet has run, and the parts of a Job's split that are cut by them. The fold is
//! Fleet's (`job_time.rs`); this crate keeps strings and milliseconds. `docs/concepts/fleet.md`.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// A build as Fleet saw it begin. `None` is unknown, not absent.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NewBuild {
    pub commit_id: String,
    pub started_at: String,
    /// `main` or `preview`.
    pub source: Option<String>,
    pub main_commit: Option<String>,
    pub branches: Vec<String>,
    /// Git was asked about it and had the commit.
    pub looked: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptFleetBuild {
    pub id: i64,
    pub build: NewBuild,
}

/// A Job's time in each bucket inside one build, in milliseconds. `build_id` 0 is before the first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeptBuildSplit {
    pub build_id: i64,
    pub model_ms: i64,
    pub tools_ms: i64,
    pub checks_ms: i64,
    pub subagents_ms: i64,
    pub slot_ms: i64,
    pub you_ms: i64,
    pub pull_request_ms: i64,
    pub stopped_ms: i64,
    pub idle_ms: i64,
}

/// One pull request's stint in a Job and the pull request bucket's time inside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptPullRequestWait {
    pub target: String,
    pub began_at: String,
    pub ended_at: String,
    pub wait_ms: i64,
}

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

const COLUMNS: &str = "id, commit_id, started_at, source, main_commit, branches, looked";

fn build_of(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptFleetBuild> {
    Ok(KeptFleetBuild {
        id: row.get(0)?,
        build: NewBuild {
            commit_id: row.get(1)?,
            started_at: row.get(2)?,
            source: row.get(3)?,
            main_commit: row.get(4)?,
            branches: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
            looked: row.get(6)?,
        },
    })
}

/// Keep a Job's split by build and its pull request waits in place of any it had, inside the
/// transaction that keeps its row.
pub(crate) fn write_parts(
    tx: &rusqlite::Transaction<'_>,
    session_id: &str,
    builds: &[KeptBuildSplit],
    waits: &[KeptPullRequestWait],
) -> rusqlite::Result<()> {
    tx.execute("DELETE FROM job_time_builds WHERE session_id = ?1", [session_id])?;
    tx.execute("DELETE FROM job_time_pull_requests WHERE session_id = ?1", [session_id])?;
    for one in builds {
        tx.execute(
            "INSERT INTO job_time_builds (session_id, build_id, model_ms, tools_ms, checks_ms, subagents_ms, slot_ms, you_ms,
             pull_request_ms, stopped_ms, idle_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                session_id, one.build_id, one.model_ms, one.tools_ms, one.checks_ms, one.subagents_ms, one.slot_ms, one.you_ms,
                one.pull_request_ms, one.stopped_ms, one.idle_ms
            ],
        )?;
    }
    for wait in waits {
        tx.execute(
            "INSERT OR REPLACE INTO job_time_pull_requests (session_id, target, began_at, ended_at, wait_ms) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![session_id, wait.target, wait.began_at, wait.ended_at, wait.wait_ms],
        )?;
    }
    Ok(())
}

impl Store {
    /// Keep a build, unless the newest one is of the same commit: a restart onto the commit already
    /// running is not another build. Whether it was kept.
    pub fn keep_build(&mut self, build: &NewBuild) -> Result<bool, WriteError> {
        let doing = "keeping a build Fleet runs";
        let newest: Option<String> = self
            .conn
            .query_row("SELECT commit_id FROM fleet_builds ORDER BY started_at DESC, id DESC LIMIT 1", [], |row| row.get(0))
            .optional()
            .map_err(failed(doing))?;
        if newest.as_deref() == Some(build.commit_id.as_str()) {
            return Ok(false);
        }
        self.conn
            .execute(
                "INSERT OR IGNORE INTO fleet_builds (commit_id, started_at, source, main_commit, branches, looked)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    build.commit_id,
                    build.started_at,
                    build.source,
                    build.main_commit,
                    serde_json::to_string(&build.branches).unwrap_or_else(|_| "[]".into()),
                    build.looked,
                ],
            )
            .map(|changed| changed > 0)
            .map_err(failed(doing))
    }

    /// Every build, in the order they began.
    pub fn builds(&self) -> Result<Vec<KeptFleetBuild>, WriteError> {
        let doing = "reading the builds Fleet has run";
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM fleet_builds ORDER BY started_at, id"))
            .map_err(failed(doing))?;
        statement
            .query_map([], build_of)
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(failed(doing))
    }

    /// The builds git has not been asked about.
    pub fn builds_unlooked(&self) -> Result<Vec<KeptFleetBuild>, WriteError> {
        Ok(self.builds()?.into_iter().filter(|one| !one.build.looked).collect())
    }

    /// What git said of a build: where it came from, the main commit under it and the branches in it.
    pub fn build_looked(&mut self, id: i64, build: &NewBuild) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE fleet_builds SET source = ?2, main_commit = ?3, branches = ?4, looked = 1 WHERE id = ?1",
                params![id, build.source, build.main_commit, serde_json::to_string(&build.branches).unwrap_or_else(|_| "[]".into())],
            )
            .map(|_| ())
            .map_err(failed("keeping what git says of a build"))
    }

    /// A Job's pull request waits, oldest first.
    pub fn job_time_waits_of(&self, session_id: &str) -> Result<Vec<KeptPullRequestWait>, WriteError> {
        let doing = "reading a job's pull request waits";
        let mut statement = self
            .conn
            .prepare("SELECT target, began_at, ended_at, wait_ms FROM job_time_pull_requests WHERE session_id = ?1 ORDER BY began_at")
            .map_err(failed(doing))?;
        statement
            .query_map([session_id], |row| {
                Ok(KeptPullRequestWait { target: row.get(0)?, began_at: row.get(1)?, ended_at: row.get(2)?, wait_ms: row.get(3)? })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(failed(doing))
    }

    /// A Job's split by build, oldest build first.
    pub fn job_time_builds_of(&self, session_id: &str) -> Result<Vec<KeptBuildSplit>, WriteError> {
        let doing = "reading a job's time by build";
        let mut statement = self
            .conn
            .prepare(
                "SELECT build_id, model_ms, tools_ms, checks_ms, subagents_ms, slot_ms, you_ms, pull_request_ms, stopped_ms, idle_ms
                 FROM job_time_builds WHERE session_id = ?1 ORDER BY build_id",
            )
            .map_err(failed(doing))?;
        statement
            .query_map([session_id], |row| {
                Ok(KeptBuildSplit {
                    build_id: row.get(0)?,
                    model_ms: row.get(1)?,
                    tools_ms: row.get(2)?,
                    checks_ms: row.get(3)?,
                    subagents_ms: row.get(4)?,
                    slot_ms: row.get(5)?,
                    you_ms: row.get(6)?,
                    pull_request_ms: row.get(7)?,
                    stopped_ms: row.get(8)?,
                    idle_ms: row.get(9)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(failed(doing))
    }
}
