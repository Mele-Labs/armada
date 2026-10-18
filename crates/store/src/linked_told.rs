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
}
