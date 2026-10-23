//! The sentences Fleet read from a reply as asking the person that he marked Not a question.
//!
//! **Kept for good, one row a sentence, and of no Session.** A misread is the reader's, not the job's:
//! every Session's replies are read against all of them, and `tools/lm-train` reads them as asking nothing.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    /// Keep `sentence` as one that asks nothing, marked on `session_id` at `at`. Whether it was new.
    pub fn keep_misread(&mut self, session_id: &str, sentence: &str, at: &str) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO reply_misreads (sentence, session_id, marked_at) VALUES (?1, ?2, ?3)",
                params![sentence, session_id, at],
            )
            .map(|kept| kept > 0)
            .map_err(fault("keeping a sentence marked as not a question"))
            .map_err(WriteError::Database)
    }

    /// Every sentence marked as not a question, oldest first.
    pub fn misreads(&self) -> Result<Vec<String>, WriteError> {
        let doing = "reading the sentences marked as not a question";
        let mut statement = self
            .conn
            .prepare("SELECT sentence FROM reply_misreads ORDER BY marked_at, sentence")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([], |row| row.get(0))
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        Ok(rows)
    }
}
