//! What the model found of a feature's docs against its code, on the owner's press of Check, and the calls the presses
//! spent. `docs/concepts/feature-catalog.md`, *Librarian*. **Nothing here edits a doc or the code**: a disagreement waits
//! on his Start a Job or Dismiss.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One doc of a feature as the last check of it found it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptDocCheck {
    pub feature_id: String,
    /// The doc as it was linked: a path in the repository.
    pub target: String,
    pub checked_at: String,
    /// What the doc says that the code does not bear out, quoted short. Empty where they agree.
    pub doc_quote: String,
    /// `path:line` in the code. Empty where they agree.
    pub code_at: String,
    /// What the code there does, quoted or said short.
    pub code_quote: String,
    /// Git's ids of the doc's text and the feature's paths, as the check read them.
    pub stamp: String,
    /// The stamp he dismissed the disagreement at. Absent until he does; a check that reads another stamp clears it.
    pub dismissed: Option<String>,
}

impl KeptDocCheck {
    /// The check found the doc and the code disagree.
    pub fn disagrees(&self) -> bool {
        !self.code_at.is_empty()
    }

    /// The disagreement stands: he has not dismissed it at the stamp it was found at.
    pub fn flagged(&self) -> bool {
        self.disagrees() && self.dismissed.as_deref() != Some(self.stamp.as_str())
    }
}

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}

impl Store {
    /// Keep what a check of `check.target` found, in place of what was kept. **A dismissal holds while the stamp is the same**,
    /// and any other stamp clears it; `check.dismissed` is not read.
    pub fn keep_doc_check(&mut self, check: &KeptDocCheck) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO feature_doc_checks (feature_id, target, checked_at, doc_quote, code_at, code_quote, stamp, dismissed) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)
                 ON CONFLICT (feature_id, target) DO UPDATE SET
                    dismissed = CASE WHEN stamp = excluded.stamp THEN dismissed ELSE NULL END,
                    checked_at = excluded.checked_at, doc_quote = excluded.doc_quote, code_at = excluded.code_at,
                    code_quote = excluded.code_quote, stamp = excluded.stamp",
                params![check.feature_id, check.target, check.checked_at, check.doc_quote, check.code_at, check.code_quote, check.stamp],
            )
            .map(drop)
            .map_err(failed("keeping what a check of a doc found"))
    }

    /// Every doc of the feature `feature_id` a check has read, by target.
    pub fn doc_checks_of(&self, feature_id: &str) -> Result<Vec<KeptDocCheck>, WriteError> {
        let doing = "reading what the checks of a feature's docs found";
        let mut statement = self
            .conn
            .prepare("SELECT target, checked_at, doc_quote, code_at, code_quote, stamp, dismissed FROM feature_doc_checks WHERE feature_id = ?1 ORDER BY target")
            .map_err(failed(doing))?;
        let rows = statement
            .query_map([feature_id], |row| {
                Ok(KeptDocCheck {
                    feature_id: feature_id.to_string(),
                    target: row.get(0)?,
                    checked_at: row.get(1)?,
                    doc_quote: row.get(2)?,
                    code_at: row.get(3)?,
                    code_quote: row.get(4)?,
                    stamp: row.get(5)?,
                    dismissed: row.get(6)?,
                })
            })
            .map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }

    /// He dismissed the disagreement found of `target`: it stays quiet while a check reads the same stamp. Whether one stood.
    pub fn dismiss_doc_check(&mut self, feature_id: &str, target: &str) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "UPDATE feature_doc_checks SET dismissed = stamp WHERE feature_id = ?1 AND target = ?2 AND code_at != '' AND (dismissed IS NULL OR dismissed != stamp)",
                params![feature_id, target],
            )
            .map(|changed| changed > 0)
            .map_err(failed("dismissing what a check of a doc found"))
    }

    /// The model calls a press of Check on `project` spent at `at`, on `night`, a local date as `2026-11-18`.
    pub fn keep_librarian_check_calls(&mut self, project: &str, night: &str, at: &str, calls: u32) -> Result<(), WriteError> {
        self.conn
            .execute("INSERT INTO librarian_check_calls (project, night, at, calls) VALUES (?1, ?2, ?3, ?4)", params![project, night, at, calls])
            .map(drop)
            .map_err(failed("keeping the calls a check of docs spent"))
    }

    /// The model calls the presses of Check on `project` spent on `night`. The night's share counts them beside its runs'.
    pub fn librarian_check_calls(&self, project: &str, night: &str) -> Result<u32, WriteError> {
        self.conn
            .query_row("SELECT COALESCE(SUM(calls), 0) FROM librarian_check_calls WHERE project = ?1 AND night = ?2", [project, night], |row| row.get(0))
            .map_err(failed("counting the calls the checks of docs spent"))
    }
}
