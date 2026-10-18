//! What a session's agent said it is waiting on the person for, kept whole.
//! `docs/concepts/session.md`.
//!
//! **Only the agent's items live here.** Fleet's own, an open card or an
//! unapproved walk, are derived on every read and never kept.

use rusqlite::{params, OptionalExtension};

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
    pub steps: Vec<String>,
}

impl Store {
    /// The agent's items for a session, in the order it gave them.
    pub fn waiting_for(&self, session_id: &str) -> Result<Vec<KeptWaiting>, WriteError> {
        let doing = "reading what a session waits on";
        let mut statement = self
            .conn
            .prepare(
                "SELECT item_id, text, since, act_kind, act_target, options, steps
                 FROM session_waiting WHERE session_id = ?1 ORDER BY position",
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([session_id], |row| {
                let kind: Option<String> = row.get(3)?;
                let target: Option<String> = row.get(4)?;
                let options: String = row.get(5)?;
                let steps: String = row.get(6)?;
                Ok(KeptWaiting {
                    item_id: row.get(0)?,
                    text: row.get(1)?,
                    since: row.get(2)?,
                    act: kind.zip(target),
                    options: serde_json::from_str(&options).unwrap_or_default(),
                    steps: serde_json::from_str(&steps).unwrap_or_default(),
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
                     (session_id, position, item_id, text, since, act_kind, act_target, options, steps)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        session_id,
                        position as i64,
                        item.item_id,
                        item.text,
                        item.since,
                        item.act.as_ref().map(|(kind, _)| kind),
                        item.act.as_ref().map(|(_, target)| target),
                        serde_json::to_string(&item.options).unwrap_or_else(|_| "[]".into()),
                        serde_json::to_string(&item.steps).unwrap_or_else(|_| "[]".into()),
                    ],
                )?;
            }
            transaction.commit()
        })();
        kept.map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// The item ids the person dismissed for good from a session's list.
    pub fn dismissed_waiting(&self, session_id: &str) -> Result<Vec<String>, WriteError> {
        let doing = "reading what was dismissed from a session's waiting list";
        let mut statement = self
            .conn
            .prepare("SELECT item_id FROM session_waiting_dismissed WHERE session_id = ?1")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([session_id], |row| row.get(0))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>()
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// When the session's agent said it was done, if it still stands. RFC3339.
    pub fn done_since(&self, session_id: &str) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row(
                "SELECT since FROM session_done WHERE session_id = ?1",
                [session_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(fault("reading when a session said it was done"))
            .map_err(WriteError::Database)
    }

    /// Keep that the agent said it was done at `since`. **A second call keeps the first**, so the
    /// callout does not move under the person.
    pub fn keep_done(&mut self, session_id: &str, since: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO session_done (session_id, since) VALUES (?1, ?2)",
                params![session_id, since],
            )
            .map(|_| ())
            .map_err(fault("keeping that a session said it was done"))
            .map_err(WriteError::Database)
    }

    /// Forget that the agent said it was done. Whether there was anything to forget.
    pub fn clear_done(&mut self, session_id: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("DELETE FROM session_done WHERE session_id = ?1", [session_id])
            .map(|gone| gone > 0)
            .map_err(fault("clearing that a session said it was done"))
            .map_err(WriteError::Database)
    }

    /// How the session's last turn ended, while nothing has followed it: when, and the end of what
    /// the agent last said. RFC3339.
    pub fn turn_end(&self, session_id: &str) -> Result<Option<(String, String)>, WriteError> {
        self.conn
            .query_row(
                "SELECT ended_at, said FROM session_turn_end WHERE session_id = ?1",
                [session_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(fault("reading how a session's turn ended"))
            .map_err(WriteError::Database)
    }

    /// Keep that a turn ended at `ended_at` on `said`, in place of any earlier one.
    pub fn keep_turn_end(&mut self, session_id: &str, ended_at: &str, said: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO session_turn_end (session_id, ended_at, said) VALUES (?1, ?2, ?3)
                 ON CONFLICT (session_id) DO UPDATE SET ended_at = ?2, said = ?3",
                params![session_id, ended_at, said],
            )
            .map(|_| ())
            .map_err(fault("keeping how a session's turn ended"))
            .map_err(WriteError::Database)
    }

    /// Forget how the last turn ended: the person spoke or another turn began. Whether there was anything.
    pub fn clear_turn_end(&mut self, session_id: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("DELETE FROM session_turn_end WHERE session_id = ?1", [session_id])
            .map(|gone| gone > 0)
            .map_err(fault("forgetting how a session's turn ended"))
            .map_err(WriteError::Database)
    }

    /// Dismiss an item for good: its id is kept, and the agent's item under it is removed.
    pub fn dismiss_waiting(&mut self, session_id: &str, item_id: &str) -> Result<(), WriteError> {
        let doing = "dismissing what a session waits on";
        let kept = (|| {
            let transaction = self.conn.transaction()?;
            transaction.execute(
                "INSERT OR IGNORE INTO session_waiting_dismissed (session_id, item_id) VALUES (?1, ?2)",
                params![session_id, item_id],
            )?;
            transaction.execute(
                "DELETE FROM session_waiting WHERE session_id = ?1 AND item_id = ?2",
                params![session_id, item_id],
            )?;
            transaction.commit()
        })();
        kept.map_err(fault(doing)).map_err(WriteError::Database)
    }
}
