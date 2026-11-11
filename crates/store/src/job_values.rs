//! The values a Job froze when it started, one row for each key with the layer it came from. **The
//! words are Fleet's and are kept as written**: the store names no key and reads none.

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One key of a Job's frozen values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptValue {
    pub key: String,
    /// `None` is a key no layer set.
    pub value: Option<String>,
    pub layer: String,
}

impl Store {
    /// Keep `values` as what `session` froze, **replacing what it had**, in one step.
    pub fn keep_job_values(&mut self, session: &str, values: &[KeptValue]) -> Result<(), WriteError> {
        let doing = "keeping what a Job froze";
        let stored = |why| WriteError::Database(fault(doing)(why));
        let tx = self.conn.transaction().map_err(stored)?;
        tx.execute("DELETE FROM job_values WHERE session_id = ?1", [session]).map_err(stored)?;
        for one in values {
            tx.execute(
                "INSERT INTO job_values (session_id, key, value, layer) VALUES (?1, ?2, ?3, ?4)",
                (session, &one.key, &one.value, &one.layer),
            )
            .map_err(stored)?;
        }
        tx.commit().map_err(stored)
    }

    /// What `session` froze, by key. Empty for a Job that froze nothing.
    pub fn job_values(&self, session: &str) -> Result<Vec<KeptValue>, WriteError> {
        let doing = "reading what a Job froze";
        let stored = |why| WriteError::Database(fault(doing)(why));
        let mut statement = self.conn.prepare("SELECT key, value, layer FROM job_values WHERE session_id = ?1 ORDER BY key").map_err(stored)?;
        let rows = statement
            .query_map([session], |row| Ok(KeptValue { key: row.get(0)?, value: row.get(1)?, layer: row.get(2)? }))
            .map_err(stored)?;
        rows.collect::<Result<_, _>>().map_err(stored)
    }
}
