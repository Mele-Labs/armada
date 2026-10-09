//! What a session's agent said it is waiting on the person for, kept whole.
//! `docs/concepts/session.md`.
//!
//! **Only the agent's items live here.** Fleet's own, an open card or an
//! unapproved walk, are derived on every read and never kept.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One item the agent stated. `since` is RFC3339, stamped by Fleet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptWaiting {
    pub item_id: String,
    pub text: String,
    pub since: String,
    pub act: Option<(String, String)>,
    pub options: Vec<String>,
}

impl Store {
    /// The agent's items for a session, in the order it gave them.
    pub fn waiting_for(&self, session_id: &str) -> Result<Vec<KeptWaiting>, WriteError> {
        let doing = "reading what a session waits on";
        let mut statement = self
            .conn
            .prepare(
                "SELECT item_id, text, since, act_kind, act_target, options
                 FROM session_waiting WHERE session_id = ?1 ORDER BY position",
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([session_id], |row| {
                let kind: Option<String> = row.get(3)?;
                let target: Option<String> = row.get(4)?;
                let options: String = row.get(5)?;
                Ok(KeptWaiting {
                    item_id: row.get(0)?,
                    text: row.get(1)?,
                    since: row.get(2)?,
                    act: kind.zip(target),
                    options: serde_json::from_str(&options).unwrap_or_default(),
                })
            })
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>()
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// Replace a session's whole list. An empty one clears it.
    pub fn replace_waiting_for(
        &mut self,
        session_id: &str,
        items: &[KeptWaiting],
    ) -> Result<(), WriteError> {
        let doing = "keeping what a session waits on";
        let kept = (|| {
            let transaction = self.conn.transaction()?;
            transaction.execute("DELETE FROM session_waiting WHERE session_id = ?1", [session_id])?;
            for (position, item) in items.iter().enumerate() {
                transaction.execute(
                    "INSERT INTO session_waiting
                     (session_id, position, item_id, text, since, act_kind, act_target, options)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        session_id,
                        position as i64,
                        item.item_id,
                        item.text,
                        item.since,
                        item.act.as_ref().map(|(kind, _)| kind),
                        item.act.as_ref().map(|(_, target)| target),
                        serde_json::to_string(&item.options).unwrap_or_else(|_| "[]".into()),
                    ],
                )?;
            }
            transaction.commit()
        })();
        kept.map_err(fault(doing)).map_err(WriteError::Database)
    }
}
