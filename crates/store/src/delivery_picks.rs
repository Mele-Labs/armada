//! How the person chose to land work, per repository and kind of work, so a start card can offer
//! the usual one.
//!
//! **The choice is the wire's word and is not looked up here**: Fleet names it and decides what
//! the usual is. This keeps the picks, newest last.

use crate::error::{fault, WriteError};
use crate::open::Store;

impl Store {
    /// Keep one pick of `workflow`'s landing in `manifest_id`.
    pub fn keep_delivery_pick(&mut self, manifest_id: &str, workflow: &str, choice: &str, at: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO delivery_picks (manifest_id, workflow, choice, picked_at) VALUES (?1, ?2, ?3, ?4)",
                (manifest_id, workflow, choice, at),
            )
            .map(|_| ())
            .map_err(fault("keeping how work lands"))
            .map_err(WriteError::Database)
    }

    /// The last `limit` picks for `workflow` in `manifest_id`, newest first.
    pub fn delivery_picks(&self, manifest_id: &str, workflow: &str, limit: u32) -> Result<Vec<String>, WriteError> {
        let doing = "reading how work lands";
        let mut statement = self
            .conn
            .prepare("SELECT choice FROM delivery_picks WHERE manifest_id = ?1 AND workflow = ?2 ORDER BY id DESC LIMIT ?3")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map((manifest_id, workflow, limit), |row| row.get::<_, String>(0))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>()
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }
}
