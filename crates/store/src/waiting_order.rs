//! The order a Job set on its open asks, and why each stands there. `docs/concepts/session/waiting-on-the-person.md`.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One ask the Job placed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptWaitingPlace {
    pub item: String,
    pub why: String,
}

impl Store {
    /// The asks `session` placed, first to last.
    pub fn waiting_order(&self, session: &str) -> Result<Vec<KeptWaitingPlace>, WriteError> {
        let doing = "reading the order of a Job's asks";
        let mut statement = self
            .conn
            .prepare("SELECT item, why FROM waiting_order WHERE session_id = ?1 ORDER BY place")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([session], |row| Ok(KeptWaitingPlace { item: row.get(0)?, why: row.get(1)? }))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// Place `items` in this order, and no other ask of `session`.
    pub fn order_waiting(&mut self, session: &str, items: &[KeptWaitingPlace]) -> Result<(), WriteError> {
        let doing = "ordering a Job's asks";
        let kept = (|| {
            let transaction = self.conn.transaction()?;
            transaction.execute("DELETE FROM waiting_order WHERE session_id = ?1", [session])?;
            for (place, one) in items.iter().enumerate() {
                transaction.execute(
                    "INSERT INTO waiting_order (session_id, item, place, why) VALUES (?1, ?2, ?3, ?4)",
                    params![session, one.item, place as i64, one.why],
                )?;
            }
            transaction.commit()
        })();
        kept.map_err(fault(doing)).map_err(WriteError::Database)
    }
}
