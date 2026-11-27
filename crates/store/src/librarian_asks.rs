//! What his asks of a project's librarian keep: the model calls they spent, a count a night, and the drop of a read one
//! filed, for Undo and Move. `docs/concepts/feature-catalog.md`, *Librarian*.

use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    /// The model calls his asks of the project's librarian spent on `night`, a local date as `2026-11-24`.
    pub fn librarian_ask_calls(&self, project: &str, night: &str) -> Result<u32, WriteError> {
        self.conn
            .query_row("SELECT COALESCE(SUM(calls), 0) FROM librarian_ask_calls WHERE project = ?1 AND night = ?2", [project, night], |row| row.get(0))
            .map_err(|why| WriteError::Database(fault("counting a project's ask calls")(why)))
    }

    /// One more call spent by an ask on `night`, and the night's count after it.
    pub fn spend_librarian_ask_call(&mut self, project: &str, night: &str) -> Result<u32, WriteError> {
        self.conn
            .execute(
                "INSERT INTO librarian_ask_calls (project, night, calls) VALUES (?1, ?2, 1) ON CONFLICT (project, night) DO UPDATE SET calls = calls + 1",
                [project, night],
            )
            .map_err(|why| WriteError::Database(fault("spending an ask call")(why)))?;
        self.librarian_ask_calls(project, night)
    }

    /// Drop one feature link, named as its key names it. Whether there was one.
    pub fn drop_feature_link(&mut self, feature_id: &str, kind: &str, target: &str, job: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("DELETE FROM feature_links WHERE feature_id = ?1 AND kind = ?2 AND target = ?3 AND job = ?4", [feature_id, kind, target, job])
            .map(|dropped| dropped > 0)
            .map_err(|why| WriteError::Database(fault("dropping a feature's link")(why)))
    }
}
