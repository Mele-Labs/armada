//! The features a Job's commits named, kept as the commit gate lets each commit through. `docs/concepts/feature-catalog.md`,
//! *The hold*.
//!
//! **Rows only**: a naming is kept the first time and a repeat leaves it as it was.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

impl Store {
    /// Keep that a commit of `job` named the feature. Whether it was the first time.
    pub fn keep_feature_job_named(&mut self, feature_id: &str, job: &str, at: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("INSERT OR IGNORE INTO feature_job_named (feature_id, job, at) VALUES (?1, ?2, ?3)", params![feature_id, job, at])
            .map(|added| added > 0)
            .map_err(failed("keeping a feature a Job named"))
    }

    /// The ids of the features `job`'s commits named, oldest naming first.
    pub fn feature_ids_named_by_job(&self, job: &str) -> Result<Vec<String>, WriteError> {
        let doing = "reading the features a Job named";
        let mut statement = self.conn.prepare("SELECT feature_id FROM feature_job_named WHERE job = ?1 ORDER BY at, feature_id").map_err(failed(doing))?;
        let rows = statement.query_map([job], |row| row.get(0)).map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }
}
