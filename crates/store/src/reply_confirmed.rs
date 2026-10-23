//! The sentences Fleet read from a reply as asking the person that he confirmed with thumbs up.
//!
//! **Kept for good, one row a sentence, and of no Session**, as `reply_misreads` keeps
//! the ones he marked the other way: `tools/lm-train` reads them as asking.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    /// Keep `sentence` as one that asks, confirmed on `session_id` at `at`. Whether it was new.
    pub fn keep_confirmed(&mut self, session_id: &str, sentence: &str, at: &str) -> Result<bool, WriteError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO reply_confirmed (sentence, session_id, marked_at) VALUES (?1, ?2, ?3)",
                params![sentence, session_id, at],
            )
            .map(|kept| kept > 0)
            .map_err(fault("keeping a sentence confirmed as a question"))
            .map_err(WriteError::Database)
    }

    /// Every sentence confirmed as a question, oldest first.
    pub fn confirmed(&self) -> Result<Vec<String>, WriteError> {
        let doing = "reading the sentences confirmed as a question";
        let mut statement = self
            .conn
            .prepare("SELECT sentence FROM reply_confirmed ORDER BY marked_at, sentence")
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
