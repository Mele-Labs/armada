//! The librarian's runs, one row each. `docs/concepts/feature-catalog.md`, *Librarian*.
//!
//! **Rows only.** What a run did is Fleet's to say; the store keeps when it ran, what it read through and which parts ran.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// What started a run, as the table names it.
pub mod run_by {
    pub const PRESS: &str = "press";
    pub const AFTER_MERGES: &str = "after_merges";
    pub const NIGHTLY: &str = "nightly";
}

/// One run of a project's librarian.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptLibrarianRun {
    pub project: String,
    /// RFC3339.
    pub ran_at: String,
    /// One of [`run_by`].
    pub by: String,
    /// The newest merged pull request it read through, 0 where the project had none.
    pub through_pr: u64,
    /// When that pull request merged, empty where there was none.
    pub through_at: String,
    /// The commit its checkout stood at, empty where git would not say.
    pub at_commit: String,
    /// The names of the parts that ran, in the order they ran.
    pub parts: Vec<String>,
}

const RUN: &str = "project, ran_at, by, through_pr, through_at, at_commit, parts";

impl Store {
    /// Keep a run. A run at the same instant for the same project is left as it was.
    pub fn keep_librarian_run(&mut self, run: &KeptLibrarianRun) -> Result<(), WriteError> {
        self.conn
            .execute(
                &format!("INSERT OR IGNORE INTO librarian_runs ({RUN}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"),
                params![run.project, run.ran_at, run.by, run.through_pr, run.through_at, run.at_commit, run.parts.join(",")],
            )
            .map(drop)
            .map_err(|why| WriteError::Database(fault("keeping a librarian run")(why)))
    }

    /// The project's latest run, or `None` where its librarian has never run.
    pub fn librarian_last_run(&self, project: &str) -> Result<Option<KeptLibrarianRun>, WriteError> {
        self.conn
            .query_row(
                &format!("SELECT {RUN} FROM librarian_runs WHERE project = ?1 ORDER BY ran_at DESC LIMIT 1"),
                [project],
                |row| {
                    let parts: String = row.get(6)?;
                    Ok(KeptLibrarianRun {
                        project: row.get(0)?,
                        ran_at: row.get(1)?,
                        by: row.get(2)?,
                        through_pr: row.get(3)?,
                        through_at: row.get(4)?,
                        at_commit: row.get(5)?,
                        parts: parts.split(',').filter(|part| !part.is_empty()).map(str::to_string).collect(),
                    })
                },
            )
            .optional()
            .map_err(|why| WriteError::Database(fault("reading a project's latest librarian run")(why)))
    }

    /// How many runs the project's librarian has made.
    pub fn librarian_run_count(&self, project: &str) -> Result<u64, WriteError> {
        self.conn
            .query_row("SELECT COUNT(*) FROM librarian_runs WHERE project = ?1", [project], |row| row.get(0))
            .map_err(|why| WriteError::Database(fault("counting a project's librarian runs")(why)))
    }
}
