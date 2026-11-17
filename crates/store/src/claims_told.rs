//! Each claim Fleet told a Job another holds, so a restart, which forgets the claims Fleet keeps in
//! memory, does not tell it again. `docs/concepts/fleet.md`, *Claims a Job holds*.

use core_model::Timestamp;

use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    /// Whether `session` was told that `holder` holds the claim on `about`.
    pub fn claims_told_before(&self, session: &str, about: &str, holder: &str) -> Result<bool, WriteError> {
        self.conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM claims_told WHERE session_id = ?1 AND about = ?2 AND holder = ?3)",
                (session, about, holder),
                |row| row.get::<_, bool>(0),
            )
            .map_err(fault("reading whether a Job was told of a claim"))
            .map_err(WriteError::Database)
    }

    /// Keep that `session` was told that `holder` holds the claim on `about`.
    pub fn keep_claims_told(&mut self, session: &str, about: &str, holder: &str, at: &Timestamp) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO claims_told (session_id, about, holder, told_at) VALUES (?1, ?2, ?3, ?4) ON CONFLICT DO NOTHING",
                (session, about, holder, at.as_str()),
            )
            .map(|_| ())
            .map_err(fault("keeping that a Job was told of a claim"))
            .map_err(WriteError::Database)
    }

    /// Forget what `session` was told of `about`, or of every claim where `about` is `None`: its
    /// wait ended, or the Job did.
    pub fn forget_claims_told(&mut self, session: &str, about: Option<&str>) -> Result<(), WriteError> {
        self.conn
            .execute(
                "DELETE FROM claims_told WHERE session_id = ?1 AND (?2 IS NULL OR about = ?2)",
                (session, about),
            )
            .map(|_| ())
            .map_err(fault("forgetting what a Job was told of its claims"))
            .map_err(WriteError::Database)
    }
}

