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
    /// This is the repository's Helm: told Helm's note, and found by the dock.
    pub helm: bool,
    /// Started by Setup (`start_session`'s `root`), so it keeps writing the repository's
    /// setup files in the checkout after its Manifest is served.
    pub setup: bool,
    /// The workflow id the Session runs under. `None` is a custom Session.
    pub workflow: Option<String>,
    /// The fields the proposer picked and the person has not changed since, comma separated:
    /// `title`, `workflow`, `model`, `effort`. Empty when none.
    pub proposed: String,
    /// The model the last turn started on, as that turn resolved it. `None` until a turn has
    /// started, so the first turn compares against nothing and tells nothing.
    pub turn_model: Option<String>,
    /// The effort the last turn started on. `None` is the CLI's own; read only where `turn_model` is.
    pub turn_effort: Option<String>,
    /// When the person paused the session: its slot is given back and its work is parked on
    /// `lease_branch`. `None` is not paused.
    pub paused_at: Option<String>,
    /// A resume is waiting for a slot. Read only where `paused_at` is.
    pub paused_resuming: bool,
}

const COLUMNS: &str =
    "session_id, manifest_id, model, effort, mode, ran, lease_slot, lease_branch, fork_of, helm, \
     setup, workflow, proposed, turn_model, turn_effort, paused_at, paused_resuming";

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
        helm: row.get::<_, i64>(9)? != 0,
        setup: row.get::<_, i64>(10)? != 0,
        workflow: row.get(11)?,
        proposed: row.get(12)?,
        turn_model: row.get(13)?,
        turn_effort: row.get(14)?,
        paused_at: row.get(15)?,
        paused_resuming: row.get::<_, i64>(16)? != 0,
    })
}

impl Store {
    pub fn keep_hosting(&mut self, hosting: &KeptHosting) -> Result<(), WriteError> {
        self.conn
            .execute(
                &format!(
                    "INSERT OR REPLACE INTO hosted_sessions ({COLUMNS})
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)"
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
                    i64::from(hosting.helm),
                    i64::from(hosting.setup),
                    hosting.workflow,
                    hosting.proposed,
                    hosting.turn_model,
                    hosting.turn_effort,
                    hosting.paused_at,
                    i64::from(hosting.paused_resuming),
                ],
            )
            .map(|_| ())
            .map_err(fault("keeping a hosted job"))
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
            .map_err(fault("reading a hosted job"))
            .map_err(WriteError::Database)
    }

    /// The live Helm Session of a repository, if it has one. At most one is ever kept live.
    pub fn live_helm_of(&self, manifest_id: &str) -> Result<Option<KeptHosting>, WriteError> {
        self.helm_hosting(manifest_id, true)
    }

    /// The newest Helm Session of a repository, live or ended, so the next one starts on what the
    /// last was tuned to.
    pub fn latest_helm_of(&self, manifest_id: &str) -> Result<Option<KeptHosting>, WriteError> {
        self.helm_hosting(manifest_id, false)
    }

    fn helm_hosting(&self, manifest_id: &str, live: bool) -> Result<Option<KeptHosting>, WriteError> {
        let columns = COLUMNS
            .split(", ")
            .map(|column| format!("h.{column}"))
            .collect::<Vec<_>>()
            .join(", ");
        let wanted = if live { "AND s.state = 'live'" } else { "" };
        self.conn
            .query_row(
                &format!(
                    "SELECT {columns} FROM hosted_sessions h JOIN sessions s ON s.id = h.session_id
                     WHERE h.helm = 1 AND h.manifest_id = ?1 {wanted}
                     ORDER BY s.started_at DESC, s.id DESC LIMIT 1"
                ),
                [manifest_id],
                kept,
            )
            .optional()
            .map_err(fault("reading a repository's Helm job"))
            .map_err(WriteError::Database)
    }

    /// Every hosted session, so a Fleet that restarts knows which ledger rows
    /// are its own.
    pub fn hostings(&self) -> Result<Vec<KeptHosting>, WriteError> {
        let doing = "reading the hosted jobs";
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
        let doing = "keeping a job row";
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
        let doing = "reading a job's thread";
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
        let doing = "keeping a terminal job's question";
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
        let doing = "reading a terminal job's question";
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
        let doing = "keeping the answer to a terminal job's question";
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
        let doing = "dropping a terminal job's question";
        self.conn
            .execute("DELETE FROM terminal_asks WHERE session_id = ?1", [session_id])
            .map(|_| ())
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }
}
