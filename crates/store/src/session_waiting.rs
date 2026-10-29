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

/// An item the person closed, kept whole at the moment he did. `source` is the wire name of the item's
/// source (`agent`, `walk`, ...), `since` when it began to wait and `closed_at` when it was closed, both RFC3339.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptClosed {
    pub item_id: String,
    pub text: String,
    pub source: String,
    pub act: Option<(String, String)>,
    pub options: Vec<String>,
    pub steps: Vec<String>,
    pub since: String,
    pub closed_at: String,
}

/// The most closed items kept for a session. An older one is still dismissed, and no longer listed.
const CLOSED_KEPT: i64 = 50;

impl Store {
    /// The agent's items for a session, in the order it gave them.
    pub fn waiting_for(&self, session_id: &str) -> Result<Vec<KeptWaiting>, WriteError> {
        let doing = "reading what a job waits on";
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
        let doing = "keeping what a job waits on";
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
        let doing = "reading what was dismissed from a job's waiting list";
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
            .map_err(fault("reading when a job said it was done"))
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
            .map_err(fault("keeping that a job said it was done"))
            .map_err(WriteError::Database)
    }

    /// Forget that the agent said it was done. Whether there was anything to forget.
    pub fn clear_done(&mut self, session_id: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("DELETE FROM session_done WHERE session_id = ?1", [session_id])
            .map(|gone| gone > 0)
            .map_err(fault("clearing that a job said it was done"))
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
            .map_err(fault("reading how a job's turn ended"))
            .map_err(WriteError::Database)
    }

    /// Keep that a turn ended at `ended_at` on `said`, in place of any earlier one. **The first reply
    /// after a tell** is marked on the tell, which is the one reply read for a refusal of it.
    pub fn keep_turn_end(&mut self, session_id: &str, ended_at: &str, said: &str) -> Result<(), WriteError> {
        let kept = (|| {
            let transaction = self.conn.transaction()?;
            transaction.execute(
                "INSERT INTO session_turn_end (session_id, ended_at, said) VALUES (?1, ?2, ?3)
                 ON CONFLICT (session_id) DO UPDATE SET ended_at = ?2, said = ?3",
                params![session_id, ended_at, said],
            )?;
            transaction.execute(
                "UPDATE session_tells SET first_end = ?2 WHERE session_id = ?1 AND first_end IS NULL AND at < ?2",
                params![session_id, ended_at],
            )?;
            transaction.commit()
        })();
        kept.map_err(fault("keeping how a job's turn ended")).map_err(WriteError::Database)
    }

    /// Forget how the last turn ended: the person spoke or another turn began. Whether there was anything.
    pub fn clear_turn_end(&mut self, session_id: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("DELETE FROM session_turn_end WHERE session_id = ?1", [session_id])
            .map(|gone| gone > 0)
            .map_err(fault("forgetting how a job's turn ended"))
            .map_err(WriteError::Database)
    }

    /// The items the person closed from a session's list, newest first.
    pub fn closed_waiting(&self, session_id: &str) -> Result<Vec<KeptClosed>, WriteError> {
        let doing = "reading what was closed from a job's waiting list";
        let mut statement = self
            .conn
            .prepare(
                "SELECT item_id, text, source, act_kind, act_target, options, steps, since, closed_at
                 FROM session_waiting_dismissed WHERE session_id = ?1 AND closed_at IS NOT NULL
                 ORDER BY closed_at DESC, rowid DESC",
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([session_id], |row| {
                let kind: Option<String> = row.get(3)?;
                let target: Option<String> = row.get(4)?;
                let options: String = row.get(5)?;
                let steps: String = row.get(6)?;
                Ok(KeptClosed {
                    item_id: row.get(0)?,
                    text: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    source: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    act: kind.zip(target),
                    options: serde_json::from_str(&options).unwrap_or_default(),
                    steps: serde_json::from_str(&steps).unwrap_or_default(),
                    since: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                    closed_at: row.get(8)?,
                })
            })
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>()
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// Keep an item whole as the person closes it, and dismiss its id if it was not. A second close of the
    /// same id keeps the later one. Only the newest [`CLOSED_KEPT`] stay listed; the rest stay dismissed.
    pub fn keep_closed_waiting(&mut self, session_id: &str, closed: &KeptClosed) -> Result<(), WriteError> {
        let doing = "keeping an item the person closed";
        let kept = (|| {
            let transaction = self.conn.transaction()?;
            transaction.execute(
                "INSERT INTO session_waiting_dismissed
                 (session_id, item_id, text, source, act_kind, act_target, options, steps, since, closed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT (session_id, item_id) DO UPDATE SET
                 text = ?3, source = ?4, act_kind = ?5, act_target = ?6, options = ?7, steps = ?8, since = ?9, closed_at = ?10",
                params![
                    session_id,
                    closed.item_id,
                    closed.text,
                    closed.source,
                    closed.act.as_ref().map(|(kind, _)| kind),
                    closed.act.as_ref().map(|(_, target)| target),
                    serde_json::to_string(&closed.options).unwrap_or_else(|_| "[]".into()),
                    serde_json::to_string(&closed.steps).unwrap_or_else(|_| "[]".into()),
                    closed.since,
                    closed.closed_at,
                ],
            )?;
            transaction.execute(
                "UPDATE session_waiting_dismissed SET closed_at = NULL
                 WHERE session_id = ?1 AND closed_at IS NOT NULL AND rowid NOT IN (
                     SELECT rowid FROM session_waiting_dismissed WHERE session_id = ?1 AND closed_at IS NOT NULL
                     ORDER BY closed_at DESC, rowid DESC LIMIT ?2)",
                params![session_id, CLOSED_KEPT],
            )?;
            transaction.commit()
        })();
        kept.map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// Reopen a closed item: let its id wait again, and put an agent's own item back at the end of its
    /// list from `restore` unless the list holds it already.
    pub fn reopen_waiting(
        &mut self,
        session_id: &str,
        item_id: &str,
        restore: Option<&KeptWaiting>,
    ) -> Result<(), WriteError> {
        let doing = "reopening an item the person closed";
        let kept = (|| {
            let transaction = self.conn.transaction()?;
            transaction.execute(
                "DELETE FROM session_waiting_dismissed WHERE session_id = ?1 AND item_id = ?2",
                params![session_id, item_id],
            )?;
            if let Some(item) = restore {
                let held: i64 = transaction.query_row(
                    "SELECT COUNT(*) FROM session_waiting WHERE session_id = ?1 AND item_id = ?2",
                    params![session_id, item_id],
                    |row| row.get(0),
                )?;
                if held == 0 {
                    transaction.execute(
                        "INSERT INTO session_waiting
                         (session_id, position, item_id, text, since, act_kind, act_target, options, steps)
                         VALUES (?1, (SELECT COALESCE(MAX(position) + 1, 0) FROM session_waiting WHERE session_id = ?1),
                                 ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                        params![
                            session_id,
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
            }
            transaction.commit()
        })();
        kept.map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// Let a dismissed id wait again. Where it was not dismissed, nothing happens.
    pub fn restore_waiting(&mut self, session_id: &str, item_id: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "DELETE FROM session_waiting_dismissed WHERE session_id = ?1 AND item_id = ?2",
                params![session_id, item_id],
            )
            .map(|_| ())
            .map_err(fault("letting a dismissed item wait again"))
            .map_err(WriteError::Database)
    }

    /// Dismiss an item for good: its id is kept, and the agent's item under it is removed.
    pub fn dismiss_waiting(&mut self, session_id: &str, item_id: &str) -> Result<(), WriteError> {
        let doing = "dismissing what a job waits on";
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
