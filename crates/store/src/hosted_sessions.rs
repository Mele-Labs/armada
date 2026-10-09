//! What a session Fleet hosts keeps beside its ledger row: how it was started,
//! whether its process has run, the slot it leased, and its thread.
//! `docs/concepts/session.md`.
//!
//! **A row of the thread is opaque here.** The wire's row is Fleet's to encode
//! and decode; the store keeps it whole, in order, and replaces one by its id.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One hosted session's own facts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptHosting {
    pub session_id: String,
    pub manifest_id: String,
    pub model: Option<String>,
    pub effort: Option<String>,
    /// `ask`, `auto`, `accept_edits` or `plan`.
    pub mode: String,
    /// Whether the agent's process has ever started.
    pub ran: bool,
    pub lease_slot: Option<u32>,
    pub lease_branch: Option<String>,
    /// The session whose conversation this one began as a copy of. Read until
    /// the process has run, after which the session resumes its own.
    pub fork_of: Option<String>,
}

const COLUMNS: &str = "session_id, manifest_id, model, effort, mode, ran, lease_slot, lease_branch, fork_of";

fn kept(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptHosting> {
    Ok(KeptHosting {
        session_id: row.get(0)?,
        manifest_id: row.get(1)?,
        model: row.get(2)?,
        effort: row.get(3)?,
        mode: row.get(4)?,
        ran: row.get::<_, i64>(5)? != 0,
        lease_slot: row.get::<_, Option<i64>>(6)?.map(|slot| slot as u32),
        lease_branch: row.get(7)?,
        fork_of: row.get(8)?,
    })
}

impl Store {
    pub fn keep_hosting(&mut self, hosting: &KeptHosting) -> Result<(), WriteError> {
        self.conn
            .execute(
                &format!(
                    "INSERT OR REPLACE INTO hosted_sessions ({COLUMNS})
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
                ),
                params![
                    hosting.session_id,
                    hosting.manifest_id,
                    hosting.model,
                    hosting.effort,
                    hosting.mode,
                    i64::from(hosting.ran),
                    hosting.lease_slot.map(i64::from),
                    hosting.lease_branch,
                    hosting.fork_of,
                ],
            )
            .map(|_| ())
            .map_err(fault("keeping a hosted session"))
            .map_err(WriteError::Database)
    }

    pub fn hosting(&self, session_id: &str) -> Result<Option<KeptHosting>, WriteError> {
        self.conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM hosted_sessions WHERE session_id = ?1"),
                [session_id],
                kept,
            )
            .optional()
            .map_err(fault("reading a hosted session"))
            .map_err(WriteError::Database)
    }

    /// Every hosted session, so a Fleet that restarts knows which ledger rows
    /// are its own.
    pub fn hostings(&self) -> Result<Vec<KeptHosting>, WriteError> {
        let doing = "reading the hosted sessions";
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM hosted_sessions"))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([], kept)
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>()
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// Put a row on the end of a thread, or replace the one of the same id.
    pub fn keep_session_row(
        &mut self,
        session_id: &str,
        row_id: &str,
        body: &str,
    ) -> Result<(), WriteError> {
        let doing = "keeping a session row";
        let replaced = self
            .conn
            .execute(
                "UPDATE session_rows SET body = ?3 WHERE session_id = ?1 AND row_id = ?2",
                params![session_id, row_id, body],
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        if replaced > 0 {
            return Ok(());
        }
        self.conn
            .execute(
                "INSERT INTO session_rows (session_id, seq, row_id, body)
                 VALUES (?1, (SELECT COALESCE(MAX(seq), 0) + 1 FROM session_rows
                              WHERE session_id = ?1), ?2, ?3)",
                params![session_id, row_id, body],
            )
            .map(|_| ())
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// A session's thread, oldest first.
    pub fn session_rows(&self, session_id: &str) -> Result<Vec<String>, WriteError> {
        let doing = "reading a session's thread";
        let mut statement = self
            .conn
            .prepare("SELECT body FROM session_rows WHERE session_id = ?1 ORDER BY seq")
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = statement
            .query_map([session_id], |row| row.get::<_, String>(0))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>()
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }
}

/// A terminal session's question, as the store keeps it. **Opaque here**, as a
/// row of the thread is: Fleet encodes and decodes the JSON.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptTerminalAsk {
    pub call: String,
    pub asking: String,
    pub in_flight: String,
    pub answer: Option<String>,
}

impl Store {
    /// Keep a session's question, replacing the last it asked.
    pub fn keep_terminal_ask(
        &mut self,
        session_id: &str,
        ask: &KeptTerminalAsk,
    ) -> Result<(), WriteError> {
        let doing = "keeping a terminal session's question";
        self.conn
            .execute(
                "INSERT OR REPLACE INTO terminal_asks (session_id, call, asking, in_flight, answer)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![session_id, ask.call, ask.asking, ask.in_flight, ask.answer],
            )
            .map(|_| ())
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    pub fn terminal_ask(&self, session_id: &str) -> Result<Option<KeptTerminalAsk>, WriteError> {
        let doing = "reading a terminal session's question";
        self.conn
            .query_row(
                "SELECT call, asking, in_flight, answer FROM terminal_asks WHERE session_id = ?1",
                [session_id],
                |row| {
                    Ok(KeptTerminalAsk {
                        call: row.get(0)?,
                        asking: row.get(1)?,
                        in_flight: row.get(2)?,
                        answer: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// Keep the answer to the question `call` named. False where that question
    /// is no longer the session's.
    pub fn answer_terminal_ask(
        &mut self,
        session_id: &str,
        call: &str,
        answer: &str,
    ) -> Result<bool, WriteError> {
        let doing = "keeping the answer to a terminal session's question";
        self.conn
            .execute(
                "UPDATE terminal_asks SET answer = ?3 WHERE session_id = ?1 AND call = ?2",
                params![session_id, call, answer],
            )
            .map(|changed| changed > 0)
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    pub fn drop_terminal_ask(&mut self, session_id: &str) -> Result<(), WriteError> {
        let doing = "dropping a terminal session's question";
        self.conn
            .execute("DELETE FROM terminal_asks WHERE session_id = ?1", [session_id])
            .map(|_| ())
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }
}
