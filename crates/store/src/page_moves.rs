//! A Job's page moved to another id: its row, its history and everything kept by its id go with it. A Job holds
//! one page a file name, kept in Fleet's own folder, so a page an earlier build kept at its slot path is moved
//! there on its next revision. `docs/concepts/document.md`, *Where a page lives*.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    /// Job `session`'s page at `from` is the page at `to` from now: its row, its versions, what he read, wrote,
    /// dismissed and archived of it, and the parts that joined it. **Where the Job holds a row at `to` already**,
    /// the row at `from` goes, and what `to` holds of its own is kept over what `from` held.
    pub fn move_page(&mut self, session: &str, from: &str, to: &str) -> Result<(), WriteError> {
        let doing = "moving a page to its own id";
        let wrap = |why| WriteError::Database(fault(doing)(why));
        let transaction = self.conn.transaction().map_err(wrap)?;
        let held = "holder_kind = 'session' AND holder_id = ?1 AND kind = 'artifact'";
        transaction
            .execute(&format!("UPDATE OR IGNORE ledger_attachments SET target = ?3 WHERE {held} AND target = ?2"), params![session, from, to])
            .map_err(wrap)?;
        transaction.execute(&format!("DELETE FROM ledger_attachments WHERE {held} AND target = ?2"), params![session, from]).map_err(wrap)?;
        transaction
            .execute(&format!("UPDATE ledger_attachment_versions SET target = ?3 WHERE {held} AND target = ?2"), params![session, from, to])
            .map_err(wrap)?;
        // A part that joined the page names it as its artifact.
        transaction
            .execute(
                &format!("UPDATE ledger_attachments SET detail = json_set(detail, '$.in', ?3) WHERE {held} AND json_extract(detail, '$.in') = ?2"),
                params![session, from, to],
            )
            .map_err(wrap)?;
        for table in ["artifact_seen", "artifact_owner_revisions", "artifact_archives", "page_presses", "page_votes"] {
            transaction
                .execute(&format!("UPDATE OR IGNORE {table} SET artifact = ?3 WHERE session_id = ?1 AND artifact = ?2"), params![session, from, to])
                .map_err(wrap)?;
            transaction.execute(&format!("DELETE FROM {table} WHERE session_id = ?1 AND artifact = ?2"), params![session, from]).map_err(wrap)?;
        }
        transaction
            .execute(
                "UPDATE artifact_archives SET item = 'document:' || ?3 WHERE session_id = ?1 AND item = 'document:' || ?2",
                params![session, from, to],
            )
            .map_err(wrap)?;
        for prefix in ["document:", "read:"] {
            let (old, new) = (format!("{prefix}{from}"), format!("{prefix}{to}"));
            transaction
                .execute("UPDATE OR IGNORE session_waiting_dismissed SET item_id = ?3 WHERE session_id = ?1 AND item_id = ?2", params![session, old, new])
                .map_err(wrap)?;
            transaction.execute("DELETE FROM session_waiting_dismissed WHERE session_id = ?1 AND item_id = ?2", params![session, old]).map_err(wrap)?;
        }
        transaction.commit().map_err(wrap)
    }
}
