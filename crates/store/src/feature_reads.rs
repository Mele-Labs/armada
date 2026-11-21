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
    /// [`crate::feature_gist_by`]: empty for the result's first sentence.
    pub gist_by: String,
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
            .prepare("SELECT job, kind, target, title, gist, at, gist_by FROM feature_job_reads WHERE job = ?1 ORDER BY at, kind, target")
            .map_err(failed(doing))?;
        let rows = statement
            .query_map([job], |row| {
                Ok(KeptJobRead { job: row.get(0)?, kind: row.get(1)?, target: row.get(2)?, title: row.get(3)?, gist: row.get(4)?, at: row.get(5)?, gist_by: row.get(6)? })
            })
            .map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// Keep `gist` as what `job` says of the doc or ticket it read by `target`, on every read link of the Job and on the
    /// read itself, so a link made later carries it. **A first sentence heard later does not replace it**: links and reads
    /// are only ever added, never rewritten, by a read. How many links it was kept on; none where the Job never linked it.
    pub fn keep_feature_job_gist(&mut self, job: &str, target: &str, gist: &str) -> Result<usize, WriteError> {
        let by = crate::feature_gist_by::JOB;
        let linked = self
            .conn
            .execute(
                "UPDATE feature_links SET gist = ?1, gist_by = ?2 WHERE job = ?3 AND target = ?4 AND via = 'read' AND kind IN ('doc', 'ticket')",
                params![gist, by, job, target],
            )
            .map_err(failed("keeping a Job's gist of what it read"))?;
        if linked > 0 {
            self.conn
                .execute("UPDATE feature_job_reads SET gist = ?1, gist_by = ?2 WHERE job = ?3 AND target = ?4", params![gist, by, job, target])
                .map_err(failed("keeping a Job's gist of what it read"))?;
        }
        Ok(linked)
    }

    /// Keep `gist` as the librarian's of the doc at `target` in `project`: on each of its read links that has no gist and
    /// is not a Job's, and on the reads behind them. **A link that already says something is left as it was**, and a Job's
    /// gist written later replaces this one. How many links it was kept on.
    pub fn keep_feature_librarian_gist(&mut self, project: &str, target: &str, gist: &str) -> Result<usize, WriteError> {
        let doing = "keeping the librarian's gist of a doc";
        let by = crate::feature_gist_by::LIBRARIAN;
        let open = "l.kind = 'doc' AND l.via = 'read' AND l.target = ?2 AND l.gist = '' AND l.gist_by != 'job' AND l.feature_id IN (SELECT id FROM features WHERE project = ?1)";
        self.conn
            .execute(
                &format!(
                    "UPDATE feature_job_reads SET gist = ?3, gist_by = '{by}' WHERE target = ?2 AND gist = '' AND gist_by != 'job' \
                     AND job IN (SELECT l.job FROM feature_links l WHERE {open})"
                ),
                params![project, target, gist],
            )
            .map_err(failed(doing))?;
        self.conn
            .execute(
                &format!("UPDATE feature_links SET gist = ?3, gist_by = '{by}' WHERE rowid IN (SELECT l.rowid FROM feature_links l WHERE {open})"),
                params![project, target, gist],
            )
            .map_err(failed(doing))
    }
}
