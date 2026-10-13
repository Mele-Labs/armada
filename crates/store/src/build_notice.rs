//! What Fleet has told a session about the build it now runs. `docs/concepts/fleet.md`, *Telling
//! sessions the build they now run*.
//!
//! **One row a (build commit, session)**, which is what makes the notice once-only: a Fleet
//! restarted onto that build reads the rows the last one wrote. The session is plain text and no
//! foreign key, as in `pull_notice`: a session the ledger forgets leaves its row.

use core_model::Timestamp;

use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    /// Keep that `recipient` was told of the build at `commit`. **`false` where it already
    /// was**, and nothing is written.
    pub fn keep_build_notice(
        &mut self,
        commit: &str,
        recipient: &str,
        at: &Timestamp,
    ) -> Result<bool, WriteError> {
        let kept = self
            .conn
            .execute(
                "INSERT INTO build_notices (commit_id, recipient, noted_at) \
                 VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
                (commit, recipient, at.as_str()),
            )
            .map_err(fault("keeping a build notice"))?;
        Ok(kept == 1)
    }

    /// Whether `recipient` was told of the build at `commit`.
    pub fn build_notice_kept(&self, commit: &str, recipient: &str) -> Result<bool, WriteError> {
        self.conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM build_notices WHERE commit_id = ?1 AND recipient = ?2)",
                (commit, recipient),
                |row| row.get::<_, bool>(0),
            )
            .map_err(fault("reading a build notice"))
            .map_err(WriteError::Database)
    }
}
