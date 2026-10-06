//! What the forge says about each repository's base branch: the commit, whether
//! its CI is green, red or still running, and for a red which jobs failed and
//! which merge turned it red. `docs/concepts/fleet.md`, *What Fleet knows about
//! main's CI*.
//!
//! **One row per repository, replaced whole.** Fleet restarts often and the
//! reading takes forge calls to rebuild, so it is kept; but only the newest
//! commit's reading is, because an older commit's red is not what main is.
//!
//! **The culprit Job is plain text and no foreign key.** A Job `armada clean`
//! forgets leaves its name here, and the row says what was true when it was
//! read.

use core_model::{JobId, Timestamp, Ulid};
use rusqlite::OptionalExtension;

use crate::error::{fault, LoadAllError, WriteError};
use crate::open::Store;

/// Version 114 — main's CI, one row per repository, and the failed jobs of a
/// red. Nothing is backfilled: no reading was kept before it.
pub(crate) const V114: &str = r#"
CREATE TABLE main_ci (
    repository   TEXT PRIMARY KEY,
    base         TEXT NOT NULL,
    commit_sha   TEXT NOT NULL,
    state        TEXT NOT NULL CHECK (state IN ('green', 'red', 'running', 'nothing_ran')),
    read_at      TEXT NOT NULL,
    red_at       TEXT,
    merge_number INTEGER CHECK (merge_number IS NULL OR merge_number > 0),
    merge_url    TEXT,
    merge_branch TEXT,
    merge_job    TEXT,
    CHECK ((state = 'red') = (red_at IS NOT NULL))
) STRICT;

CREATE TABLE main_ci_failed (
    repository TEXT NOT NULL REFERENCES main_ci(repository) ON DELETE CASCADE,
    position   INTEGER NOT NULL,
    job_name   TEXT NOT NULL,
    check_name TEXT,
    log_url    TEXT,
    tests      TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (repository, position)
) STRICT;
"#;

/// How main's newest commit stands on the forge's CI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainState {
    Green,
    /// At least one job failed. **Read before unfinished**: a failed job does
    /// not become a pass when the rest finish.
    Red,
    Running,
    /// The forge ran nothing on it. **Not green**: nothing was proved.
    NothingRan,
}

impl MainState {
    pub fn as_str(self) -> &'static str {
        match self {
            MainState::Green => "green",
            MainState::Red => "red",
            MainState::Running => "running",
            MainState::NothingRan => "nothing_ran",
        }
    }

    fn from_column(text: &str) -> Option<MainState> {
        [
            MainState::Green,
            MainState::Red,
            MainState::Running,
            MainState::NothingRan,
        ]
        .into_iter()
        .find(|state| state.as_str() == text)
    }
}

/// One CI job that failed on main's commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MainFailedJob {
    /// The job's name as the forge reports it.
    pub name: String,
    /// The Manifest Check it maps to, where one does. `None` is normal.
    pub check: Option<String>,
    pub log_url: Option<String>,
    /// Tests read out of the job's log, in the order printed. **Empty where
    /// none could be read, and never a guess.**
    pub tests: Vec<String>,
}

/// The merge that put the commit on main.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MainMerge {
    pub number: u64,
    pub url: Option<String>,
    pub branch: Option<String>,
    /// The Fleet Job whose pull request that is, where one is.
    pub job: Option<JobId>,
}

/// A repository's main, as last read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MainCi {
    pub repository: String,
    pub base: String,
    pub commit: String,
    pub state: MainState,
    /// When Fleet read this.
    pub read_at: Timestamp,
    /// When Fleet first read this commit red. Set exactly where `state` is `Red`.
    pub red_at: Option<Timestamp>,
    pub failed: Vec<MainFailedJob>,
    pub merge: Option<MainMerge>,
}

impl Store {
    /// Keep a repository's reading, replacing the one before it.
    pub fn record_main_ci(&mut self, main: &MainCi) -> Result<(), WriteError> {
        let doing = fault("keeping main's CI");
        let tx = self.conn.transaction().map_err(doing)?;
        let keeping = fault("keeping main's CI");
        tx.execute(
            "DELETE FROM main_ci_failed WHERE repository = ?1",
            (&main.repository,),
        )
        .map_err(fault("keeping main's CI"))?;
        let merge = main.merge.as_ref();
        tx.execute(
            "INSERT INTO main_ci (repository, base, commit_sha, state, read_at, red_at, \
             merge_number, merge_url, merge_branch, merge_job) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) \
             ON CONFLICT (repository) DO UPDATE SET base = ?2, commit_sha = ?3, state = ?4, \
             read_at = ?5, red_at = ?6, merge_number = ?7, merge_url = ?8, merge_branch = ?9, \
             merge_job = ?10",
            (
                &main.repository,
                &main.base,
                &main.commit,
                main.state.as_str(),
                main.read_at.as_str(),
                main.red_at.as_ref().map(|at| at.as_str().to_string()),
                merge.map(|merge| merge.number as i64),
                merge.and_then(|merge| merge.url.clone()),
                merge.and_then(|merge| merge.branch.clone()),
                merge
                    .and_then(|merge| merge.job.as_ref())
                    .map(|job| job.as_str().to_string()),
            ),
        )
        .map_err(keeping)?;
        for (position, job) in main.failed.iter().enumerate() {
            tx.execute(
                "INSERT INTO main_ci_failed (repository, position, job_name, check_name, \
                 log_url, tests) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                (
                    &main.repository,
                    position as i64,
                    &job.name,
                    &job.check,
                    &job.log_url,
                    job.tests.join("\n"),
                ),
            )
            .map_err(fault("keeping main's CI"))?;
        }
        tx.commit().map_err(fault("keeping main's CI"))?;
        Ok(())
    }

    /// What was last kept for a repository, or `None` where Fleet has never
    /// read its main.
    pub fn main_ci(&self, repository: &str) -> Result<Option<MainCi>, LoadAllError> {
        let reading = fault("reading main's CI");
        let head = self
            .conn
            .query_row(
                "SELECT base, commit_sha, state, read_at, red_at, merge_number, merge_url, \
                 merge_branch, merge_job FROM main_ci WHERE repository = ?1",
                (repository,),
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<i64>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<String>>(7)?,
                        row.get::<_, Option<String>>(8)?,
                    ))
                },
            )
            .optional()
            .map_err(reading)
            .map_err(LoadAllError::Database)?;
        let Some((base, commit, state, read_at, red_at, number, url, branch, job)) = head else {
            return Ok(None);
        };
        let state = MainState::from_column(&state).ok_or_else(|| {
            LoadAllError::Database(fault("reading main's CI")(
                rusqlite::Error::InvalidColumnType(
                    2,
                    format!("a state {state} this build does not know"),
                    rusqlite::types::Type::Text,
                ),
            ))
        })?;
        let mut asking = self
            .conn
            .prepare(
                "SELECT job_name, check_name, log_url, tests FROM main_ci_failed \
                 WHERE repository = ?1 ORDER BY position",
            )
            .map_err(fault("reading main's failed jobs"))
            .map_err(LoadAllError::Database)?;
        let failed = asking
            .query_map((repository,), |row| {
                let tests: String = row.get(3)?;
                Ok(MainFailedJob {
                    name: row.get(0)?,
                    check: row.get(1)?,
                    log_url: row.get(2)?,
                    tests: tests
                        .lines()
                        .filter(|name| !name.is_empty())
                        .map(str::to_string)
                        .collect(),
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault("reading main's failed jobs"))
            .map_err(LoadAllError::Database)?;
        Ok(Some(MainCi {
            repository: repository.to_string(),
            base,
            commit,
            state,
            read_at: Timestamp::from_rfc3339(read_at),
            red_at: red_at.map(Timestamp::from_rfc3339),
            failed,
            merge: number.map(|number| MainMerge {
                number: number as u64,
                url,
                branch,
                job: job.map(|job| JobId::carried(Ulid::carried(job))),
            }),
        }))
    }

    /// The Jobs whose pull request has this number, newest first. **The number
    /// alone cannot name a repository**, so a caller that serves more than one
    /// keeps only the Jobs that belong to the repository it is asking about.
    pub fn jobs_with_pull_request_number(&self, number: u64) -> Result<Vec<JobId>, LoadAllError> {
        let reading = fault("finding the Job of a pull request");
        let mut asking = self
            .conn
            .prepare(
                "SELECT job_id FROM jobs WHERE delivery_pull_request LIKE ?1 \
                 ORDER BY job_id DESC",
            )
            .map_err(reading)
            .map_err(LoadAllError::Database)?;
        let rows = asking
            .query_map((format!("%/pull/{number}"),), |row| {
                Ok(JobId::carried(Ulid::carried(row.get::<_, String>(0)?)))
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault("finding the Job of a pull request"))
            .map_err(LoadAllError::Database)?;
        Ok(rows)
    }
}
