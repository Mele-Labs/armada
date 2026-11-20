//! The names a linked document gives that the code no longer has, kept by the librarian's drift part.
//! `docs/concepts/feature-catalog.md`, *Librarian*. One row a (project, document); a check that finds nothing clears it.

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One document's drift as the last check found it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptDocDrift {
    pub project: String,
    /// The document as it was linked: a path in the repository.
    pub target: String,
    /// The names it gives that the code had when it was last changed and has not now, in the order it names them.
    pub names: Vec<String>,
    /// The commit the document was last changed in.
    pub doc_commit: String,
    pub checked_at: String,
}

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

impl Store {
    /// Keep what a check of `drift.target` found, in place of what was kept. **No names clears the row.**
    pub fn keep_doc_drift(&mut self, drift: &KeptDocDrift) -> Result<(), WriteError> {
        if drift.names.is_empty() {
            return self.conn.execute("DELETE FROM feature_doc_drift WHERE project = ?1 AND target = ?2", [&drift.project, &drift.target]).map(drop).map_err(failed("clearing a document's drift"));
        }
        let names = serde_json::to_string(&drift.names).unwrap_or_else(|_| String::from("[]"));
        self.conn
            .execute(
                "INSERT INTO feature_doc_drift (project, target, names, doc_commit, checked_at) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (project, target) DO UPDATE SET names = excluded.names, doc_commit = excluded.doc_commit, checked_at = excluded.checked_at",
                rusqlite::params![drift.project, drift.target, names, drift.doc_commit, drift.checked_at],
            )
            .map(drop)
            .map_err(failed("keeping a document's drift"))
    }

    /// Every document of `project` whose last check found names, by target.
    pub fn doc_drift_in(&self, project: &str) -> Result<Vec<KeptDocDrift>, WriteError> {
        let doing = "reading a project's document drift";
        let mut statement = self.conn.prepare("SELECT target, names, doc_commit, checked_at FROM feature_doc_drift WHERE project = ?1 ORDER BY target").map_err(failed(doing))?;
        let rows = statement
            .query_map([project], |row| {
                let names: String = row.get(1)?;
                Ok(KeptDocDrift {
                    project: project.to_string(),
                    target: row.get(0)?,
                    names: serde_json::from_str(&names).unwrap_or_default(),
                    doc_commit: row.get(2)?,
                    checked_at: row.get(3)?,
                })
            })
            .map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }
}
