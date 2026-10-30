//! What Fleet told a Session about a Session it follows, so each is told once and a restart does not
//! tell it again. `docs/concepts/session.md`, *Linked Sessions*.

use core_model::Timestamp;

use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    /// Keep that the parent of `child` was told of `event` under `key`. **`false` where it already
    /// was**, and nothing is written, so the caller tells only on `true`.
    pub fn keep_linked_told(&mut self, child: &str, event: &str, key: &str, at: &Timestamp) -> Result<bool, WriteError> {
        let kept = self
            .conn
            .execute(
                "INSERT INTO session_linked_told (child, event, key, told_at) VALUES (?1, ?2, ?3, ?4) ON CONFLICT DO NOTHING",
                (child, event, key, at.as_str()),
            )
            .map_err(fault("keeping what a linked Job's parent was told"))?;
        Ok(kept == 1)
    }

    /// Forget each `merged` kept for a pull request its child's row says did not merge, so the real
    /// merge is told when it comes. A row the mod spent off a `gh pr merge --disable-auto` was read
    /// as merged and kept (#3042).
    pub fn forget_linked_merged_misread(&mut self) -> Result<usize, WriteError> {
        self.conn
            .execute(
                "DELETE FROM session_linked_told WHERE event = 'merged' AND EXISTS (
                     SELECT 1 FROM ledger_attachments
                     WHERE holder_kind = 'session' AND holder_id = session_linked_told.child
                       AND kind = 'pr' AND target = session_linked_told.key
                       AND json_extract(detail, '$.state') IN ('open', 'draft', 'closed'))",
                (),
            )
            .map_err(fault("forgetting a merge a linked Job's parent was told wrongly"))
            .map_err(WriteError::Database)
    }
}
