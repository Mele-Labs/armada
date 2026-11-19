//! What a Job read from outside the code, kept until a write of its reaches a feature. `docs/concepts/feature-catalog.md`,
//! *Docs and tickets a Job read*.
//!
//! **Rows only**, in the order they were heard: a read is kept the first time and a repeat leaves it as it was, so a
//! read that came after a write cannot be moved before it by being read again.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

/// A document, page or ticket a Job read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptJobRead {
    pub job: String,
    /// `doc` or `ticket`, as `feature_link_kind` names them.
    pub kind: String,
    /// The address, path or id it was read by.
    pub target: String,
    pub title: String,
    pub gist: String,
    pub at: String,
}

impl Store {
    /// Keep that `read.job` read something. Whether it was the first time.
    pub fn keep_feature_job_read(&mut self, read: &KeptJobRead) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO feature_job_reads (job, kind, target, title, gist, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![read.job, read.kind, read.target, read.title, read.gist, read.at],
            )
            .map(|added| added > 0)
            .map_err(failed("keeping what a Job read"))
    }

    /// What `job` has read so far, oldest first.
    pub fn feature_job_reads(&self, job: &str) -> Result<Vec<KeptJobRead>, WriteError> {
        let doing = "reading what a Job read";
        let mut statement = self
            .conn
            .prepare("SELECT job, kind, target, title, gist, at FROM feature_job_reads WHERE job = ?1 ORDER BY at, kind, target")
            .map_err(failed(doing))?;
        let rows = statement
            .query_map([job], |row| {
                Ok(KeptJobRead { job: row.get(0)?, kind: row.get(1)?, target: row.get(2)?, title: row.get(3)?, gist: row.get(4)?, at: row.get(5)? })
            })
            .map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }
}
