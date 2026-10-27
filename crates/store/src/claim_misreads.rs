//! The sentences of a reply that set off a walk or done reminder, which a job disputed and the model
//! agreed asked nothing.
//!
//! **Kept for good, one row a sentence and claim, and of no Session.** A misread is the reader's, not
//! the job's: every Session's replies are read against all of them.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    /// Keep `sentence` as one that does not make `claim`, disputed on `session_id` at `at`. Whether it
    /// was new.
    pub fn keep_claim_misread(&mut self, session_id: &str, claim: &str, sentence: &str, at: &str) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO claim_misreads (sentence, claim, session_id, marked_at) VALUES (?1, ?2, ?3, ?4)",
                params![sentence, claim, session_id, at],
            )
            .map(|kept| kept > 0)
            .map_err(fault("keeping a sentence a reminder misread"))
            .map_err(WriteError::Database)
    }

    /// Every sentence kept as not making `claim`, oldest first.
    pub fn claim_misreads(&self, claim: &str) -> Result<Vec<String>, WriteError> {
        let doing = "reading the sentences a reminder misread";
        let mut statement = self
            .conn
            .prepare("SELECT sentence FROM claim_misreads WHERE claim = ?1 ORDER BY marked_at, sentence")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([claim], |row| row.get(0))
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        Ok(rows)
    }
}
