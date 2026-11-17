//! The code searches each Job ran before its first Edit or Write, read off its turns. `docs/concepts/feature-catalog.md`,
//! *How we know it worked*.
//!
//! A Job's first edit is the earliest turn, its own or a subagent's, that called Edit, Write, MultiEdit or NotebookEdit.
//! Its figure is what the turns up to and including that one counted. **A turn read before searches were counted has
//! no figure**, and a Job with one before its first edit is unmeasured, never a zero.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One Job's first edit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptFirstEdit {
    pub session_id: String,
    /// The Monday, UTC, of the week of its first edit: `YYYY-MM-DD`.
    pub week: String,
    /// The code searches before that edit.
    pub searches: u32,
    /// Every turn up to the edit carries a figure.
    pub measured: bool,
}

impl Store {
    /// The Jobs whose first edit fell in the `weeks` weeks ending with the week `now` is in, oldest first.
    pub fn first_edits(&self, now: &str, weeks: u32) -> Result<Vec<KeptFirstEdit>, WriteError> {
        let doing = "reading the searches before each job's first edit";
        let back = format!("-{} days", weeks.saturating_sub(1) * 7);
        let mut statement = self
            .conn
            .prepare(
                "WITH first_edit AS (
                     SELECT session_id, MIN(at) AS at FROM session_turn_usage
                     WHERE tools LIKE '%\"Edit\"%' OR tools LIKE '%\"Write\"%' OR tools LIKE '%\"MultiEdit\"%' OR tools LIKE '%\"NotebookEdit\"%'
                     GROUP BY session_id
                 )
                 SELECT f.session_id, date(f.at, '-6 days', 'weekday 1') AS week,
                        COALESCE(SUM(t.searches), 0), SUM(CASE WHEN t.searches IS NULL THEN 1 ELSE 0 END)
                 FROM first_edit f JOIN session_turn_usage t ON t.session_id = f.session_id AND t.at <= f.at
                 WHERE date(f.at, '-6 days', 'weekday 1') >= date(?1, '-6 days', 'weekday 1', ?2)
                 GROUP BY f.session_id ORDER BY week, f.session_id",
            )
            .map_err(failed(doing))?;
        let rows = statement
            .query_map(params![now, back], |row| {
                Ok(KeptFirstEdit {
                    session_id: row.get(0)?,
                    week: row.get(1)?,
                    searches: row.get::<_, i64>(2)? as u32,
                    measured: row.get::<_, i64>(3)? == 0,
                })
            })
            .map_err(failed(doing))?;
        rows.collect::<Result<_, _>>().map_err(failed(doing))
    }
}

fn failed(doing: &'static str) -> impl Fn(rusqlite::Error) -> WriteError {
    move |why| WriteError::Database(fault(doing)(why))
}
