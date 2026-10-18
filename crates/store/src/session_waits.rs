//! A Session dispatched to wait on another, until its condition is met. `docs/concepts/session.md`,
//! *A Session that waits on another*.
//!
//! **The row stays once the Session has started** (`started_at`), so a restart cannot start it twice.
//! The condition is kept as the words the wire spells it in and read back by Fleet; nothing here
//! knows what one means.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One waiting Session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptWait {
    /// The Session that waits.
    pub session_id: String,
    /// The Session it waits on.
    pub on_session: String,
    /// `ends`, `group_done`, `task_done`, `pr_pushed` or `pr_lands`.
    pub condition: String,
    /// The plan group or task id a `group_done` or `task_done` names. Empty otherwise.
    pub part: String,
    /// `main` or `branch`.
    pub base: String,
    pub needs: String,
    pub brief: String,
    /// An agent started it: once the condition is met its first message is held for the person.
    pub held: bool,
    /// The Session waited on has been told of the wait.
    pub told: bool,
    /// `you` or `agent`: who last set the condition.
    pub by: String,
    pub pull_number: Option<String>,
    pub pull_branch: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
}

const COLUMNS: &str = "session_id, on_session, condition, part, base, needs, brief, held, told, by, pull_number, pull_branch, created_at, started_at";

fn kept(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptWait> {
    Ok(KeptWait {
        session_id: row.get(0)?,
        on_session: row.get(1)?,
        condition: row.get(2)?,
        part: row.get(3)?,
        base: row.get(4)?,
        needs: row.get(5)?,
        brief: row.get(6)?,
        held: row.get::<_, i64>(7)? != 0,
        told: row.get::<_, i64>(8)? != 0,
        by: row.get(9)?,
        pull_number: row.get(10)?,
        pull_branch: row.get(11)?,
        created_at: row.get(12)?,
        started_at: row.get(13)?,
    })
}

impl Store {
    /// Keep a wait, replacing the one of the same Session.
    pub fn keep_wait(&mut self, wait: &KeptWait) -> Result<(), WriteError> {
        self.conn
            .execute(
                &format!("INSERT OR REPLACE INTO session_waits ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"),
                params![
                    wait.session_id,
                    wait.on_session,
                    wait.condition,
                    wait.part,
                    wait.base,
                    wait.needs,
                    wait.brief,
                    i64::from(wait.held),
                    i64::from(wait.told),
                    wait.by,
                    wait.pull_number,
                    wait.pull_branch,
                    wait.created_at,
                    wait.started_at,
                ],
            )
            .map(|_| ())
            .map_err(fault("keeping a waiting job"))
            .map_err(WriteError::Database)
    }

    /// The wait a Session was dispatched with, started or not.
    pub fn wait_of(&self, session_id: &str) -> Result<Option<KeptWait>, WriteError> {
        self.conn
            .query_row(&format!("SELECT {COLUMNS} FROM session_waits WHERE session_id = ?1"), [session_id], kept)
            .optional()
            .map_err(fault("reading a waiting job"))
            .map_err(WriteError::Database)
    }

    /// The Sessions that have not started and wait on `on`, oldest first.
    pub fn waits_on(&self, on: &str) -> Result<Vec<KeptWait>, WriteError> {
        self.waits_where("on_session = ?1 AND started_at IS NULL", Some(on))
    }

    /// Every Session that has not started and waits, oldest first.
    pub fn open_waits(&self) -> Result<Vec<KeptWait>, WriteError> {
        self.waits_where("started_at IS NULL", None)
    }

    fn waits_where(&self, condition: &str, bound: Option<&str>) -> Result<Vec<KeptWait>, WriteError> {
        let doing = "reading the waiting jobs";
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM session_waits WHERE {condition} ORDER BY created_at, session_id"))
            .map_err(fault(doing))
            .map_err(WriteError::Database)?;
        let rows = match bound {
            Some(bound) => statement.query_map([bound], kept),
            None => statement.query_map([], kept),
        }
        .map_err(fault(doing))
        .map_err(WriteError::Database)?;
        rows.collect::<Result<_, _>>().map_err(fault(doing)).map_err(WriteError::Database)
    }

    /// Mark the Session waited on told. **`true` only for the call that did it.**
    pub fn claim_wait_told(&mut self, session_id: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("UPDATE session_waits SET told = 1 WHERE session_id = ?1 AND told = 0", [session_id])
            .map(|changed| changed == 1)
            .map_err(fault("telling the job waited on"))
            .map_err(WriteError::Database)
    }

    /// Mark the wait started. **`true` only for the call that did it**, so a Session is started once
    /// however many events find its condition met.
    pub fn claim_wait_start(&mut self, session_id: &str, at: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("UPDATE session_waits SET started_at = ?2 WHERE session_id = ?1 AND started_at IS NULL", params![session_id, at])
            .map(|changed| changed == 1)
            .map_err(fault("starting a waiting job"))
            .map_err(WriteError::Database)
    }
}
