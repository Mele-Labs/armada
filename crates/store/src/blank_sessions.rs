//! The ended sessions that were never anything: no directory, no title, no turn
//! and nothing held. An unattended `claude -p` whose mod reported only its end
//! left one every call, and they buried the Ended list.

use crate::error::{fault, WriteError};
use crate::open::Store;

/// The sessions to forget: ended, empty, holding nothing.
const BLANK: &str = "state = 'ended' AND cwd = '' AND title IS NULL AND last_turn_at IS NULL
                   AND NOT EXISTS (
                       SELECT 1 FROM ledger_attachments a
                       WHERE a.holder_kind = 'session' AND a.holder_id = sessions.id)";

impl Store {
    /// Deletes every blank ended session, and answers how many went.
    pub fn forget_blank_sessions(&mut self) -> Result<usize, WriteError> {
        let doing = "forgetting the blank ended jobs";
        let wrap = |why| WriteError::Database(fault(doing)(why));
        let transaction = self.conn.transaction().map_err(wrap)?;
        // The versions go first: once the session is gone nothing says which were its.
        transaction
            .execute(
                &format!(
                    "DELETE FROM ledger_attachment_version_origins
                     WHERE seq IN (
                         SELECT seq FROM ledger_attachment_versions
                         WHERE holder_kind = 'session' AND holder_id IN (
                             SELECT id FROM sessions WHERE {BLANK}))"
                ),
                [],
            )
            .map_err(wrap)?;
        transaction
            .execute(
                &format!(
                    "DELETE FROM ledger_attachment_versions
                     WHERE holder_kind = 'session' AND holder_id IN (
                         SELECT id FROM sessions WHERE {BLANK})"
                ),
                [],
            )
            .map_err(wrap)?;
        transaction
            .execute(
                &format!("DELETE FROM session_focus WHERE session_id IN (SELECT id FROM sessions WHERE {BLANK})"),
                [],
            )
            .map_err(wrap)?;
        let forgotten = transaction
            .execute(&format!("DELETE FROM sessions WHERE {BLANK}"), [])
            .map_err(wrap)?;
        transaction.commit().map_err(wrap)?;
        Ok(forgotten)
    }
}
