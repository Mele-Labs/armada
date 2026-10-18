//! The ended sessions that were never anything: no directory, no title, no turn
//! and nothing held. An unattended `claude -p` whose mod reported only its end
//! left one every call, and they buried the Ended list.

use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    /// Deletes every blank ended session, and answers how many went.
    pub fn forget_blank_sessions(&mut self) -> Result<usize, WriteError> {
        self.conn
            .execute(
                "DELETE FROM sessions
                 WHERE state = 'ended' AND cwd = '' AND title IS NULL AND last_turn_at IS NULL
                   AND NOT EXISTS (
                       SELECT 1 FROM ledger_attachments a
                       WHERE a.holder_kind = 'session' AND a.holder_id = sessions.id)",
                [],
            )
            .map_err(fault("forgetting the blank ended jobs"))
            .map_err(WriteError::Database)
    }
}
