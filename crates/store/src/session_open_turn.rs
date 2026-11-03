//! The turn a hosted Job has open, one row a Job, so a Fleet that comes back finds the turn a dead
//! process cut off. `docs/concepts/session/a-job-fleet-hosts.md`, *What a restart does to a job*.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

/// A turn open when it was last heard of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptOpenTurn {
    pub session_id: String,
    /// When the turn was first heard to begin. RFC3339.
    pub since: String,
    /// When the process was found dead with the turn open. RFC3339.
    pub ended_at: Option<String>,
    /// How many times Fleet started the process again for this turn.
    pub started_again: u32,
}

const COLUMNS: &str = "session_id, since, ended_at, started_again";

fn kept(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptOpenTurn> {
    Ok(KeptOpenTurn {
        session_id: row.get(0)?,
        since: row.get(1)?,
        ended_at: row.get(2)?,
        started_again: row.get(3)?,
    })
}

impl Store {
    /// The turn this Job has open, if it has one.
    pub fn open_turn(&self, session_id: &str) -> Result<Option<KeptOpenTurn>, WriteError> {
        self.conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM session_open_turn WHERE session_id = ?1"),
                [session_id],
                kept,
            )
            .optional()
            .map_err(fault("reading a job's open turn"))
            .map_err(WriteError::Database)
    }

    /// Every Job with a turn open, oldest first.
    pub fn open_turns(&self) -> Result<Vec<KeptOpenTurn>, WriteError> {
        let doing = "listing the jobs with a turn open";
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM session_open_turn ORDER BY since, session_id"))
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

    /// A turn began at `at`. One already open keeps its `since` and its count, and is no longer
    /// ended: the process speaking is what lets a dead one's row go.
    pub fn turn_opened(&mut self, session_id: &str, at: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO session_open_turn (session_id, since) VALUES (?1, ?2) \
                 ON CONFLICT (session_id) DO UPDATE SET ended_at = NULL",
                params![session_id, at],
            )
            .map(|_| ())
            .map_err(fault("keeping a job's turn open"))
            .map_err(WriteError::Database)
    }

    /// The turn stopped counting as open without a death. Whether there was one.
    pub fn turn_closed(&mut self, session_id: &str) -> Result<bool, WriteError> {
        self.conn
            .execute("DELETE FROM session_open_turn WHERE session_id = ?1", [session_id])
            .map(|gone| gone > 0)
            .map_err(fault("closing a job's open turn"))
            .map_err(WriteError::Database)
    }

    /// The process was found dead with the turn open, at `at`. How many times Fleet has started it
    /// again since the turn began.
    pub fn turn_ended_dead(&mut self, session_id: &str, at: &str) -> Result<u32, WriteError> {
        let doing = "keeping that a job's process ended in a turn";
        self.conn
            .query_row(
                "INSERT INTO session_open_turn (session_id, since, ended_at) VALUES (?1, ?2, ?2) \
                 ON CONFLICT (session_id) DO UPDATE SET ended_at = ?2 \
                 RETURNING started_again",
                params![session_id, at],
                |row| row.get(0),
            )
            .map_err(fault(doing))
            .map_err(WriteError::Database)
    }

    /// Fleet started the process again for this turn.
    pub fn turn_started_again(&mut self, session_id: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE session_open_turn SET started_again = started_again + 1 WHERE session_id = ?1",
                [session_id],
            )
            .map(|_| ())
            .map_err(fault("counting a job's start again"))
            .map_err(WriteError::Database)
    }
}
