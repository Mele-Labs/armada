//! The notices a Fleet mod's rule raised as calls on a Job's Needs you list.
//!
//! **One row a notice**, kept for good: the list shows the ones the person has not dismissed, and
//! dismissing is the dismissed-ids list's, as for every item Fleet derives.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One call, as it was raised.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptCall {
    /// The item's id on the Needs you list: `call:<n>`.
    pub id: String,
    pub session_id: String,
    /// The word of the notice that was raised.
    pub notice: String,
    pub text: String,
    pub raised_at: String,
}

impl Store {
    /// Raise a call for `session_id`: the notice `notice` and the words the Session was told.
    pub fn keep_call(&mut self, session_id: &str, notice: &str, text: &str, at: &str) -> Result<KeptCall, WriteError> {
        self.conn
            .execute(
                "INSERT INTO fleet_calls (session_id, notice, text, raised_at) VALUES (?1, ?2, ?3, ?4)",
                params![session_id, notice, text, at],
            )
            .map_err(fault("keeping a call a Fleet rule raised"))
            .map_err(WriteError::Database)?;
        let n = self.conn.last_insert_rowid();
        Ok(KeptCall { id: format!("call:{n}"), session_id: session_id.into(), notice: notice.into(), text: text.into(), raised_at: at.into() })
    }

    /// Take back every call of `notice`, for whichever Sessions hold one: **a call whose cause
    /// has gone**. The Sessions that held one, each once, so their lists are read again.
    pub fn drop_calls(&mut self, notice: &str) -> Result<Vec<String>, WriteError> {
        let doing = "taking back the calls of a notice";
        let mut statement = self
            .conn
            .prepare("SELECT DISTINCT session_id FROM fleet_calls WHERE notice = ?1")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let held = statement
            .query_map([notice], |row| row.get::<_, String>(0))
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        drop(statement);
        self.conn
            .execute("DELETE FROM fleet_calls WHERE notice = ?1", [notice])
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        Ok(held)
    }

    /// The calls raised for `session_id`, oldest first.
    pub fn calls_of(&self, session_id: &str) -> Result<Vec<KeptCall>, WriteError> {
        let doing = "reading the calls a Fleet rule raised";
        let mut statement = self
            .conn
            .prepare("SELECT n, notice, text, raised_at FROM fleet_calls WHERE session_id = ?1 ORDER BY n")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        statement
            .query_map([session_id], |row| {
                Ok(KeptCall {
                    id: format!("call:{}", row.get::<_, i64>(0)?),
                    session_id: session_id.to_string(),
                    notice: row.get(1)?,
                    text: row.get(2)?,
                    raised_at: row.get(3)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }
}
