//! The Jobs Fleet compacted while they waited on the owner, and what the request after each cost.
//! `docs/concepts/session/where-the-tokens-went.md`, *A Job compacted while it waits*.

use rusqlite::{params, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;
use crate::usage_summary::STAMP;

/// A compaction Fleet sent, and the Job's first main-thread turn after it once one is read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptCompaction {
    pub session_id: String,
    pub at: String,
    /// The last main-thread turn's input, cache-creation and cache-read tokens before it.
    pub context_before: u64,
    pub next: Option<NextRequest>,
}

/// The first main-thread request after a compaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NextRequest {
    pub at: String,
    pub input: u64,
    pub cache_write: u64,
    pub cache_read: u64,
}

const COLUMNS: &str = "session_id, at, context_before, next_at, next_input, next_cache_write, next_cache_read";

fn kept(row: &rusqlite::Row<'_>) -> rusqlite::Result<KeptCompaction> {
    let next_at: Option<String> = row.get(3)?;
    let tokens = |at: usize| -> rusqlite::Result<u64> { Ok(row.get::<_, Option<i64>>(at)?.unwrap_or(0) as u64) };
    Ok(KeptCompaction {
        session_id: row.get(0)?,
        at: row.get(1)?,
        context_before: row.get::<_, i64>(2)? as u64,
        next: match next_at {
            Some(at) => Some(NextRequest {
                at,
                input: tokens(4)?,
                cache_write: tokens(5)?,
                cache_read: tokens(6)?,
            }),
            None => None,
        },
    })
}

fn reading(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("reading a job's waiting compactions")(why))
}

impl Store {
    /// The context of the Job's newest main-thread turn: its input, cache-creation and cache-read
    /// tokens. `None` where it has none.
    pub fn last_main_context(&self, session_id: &str) -> Result<Option<u64>, WriteError> {
        self.conn
            .query_row(
                "SELECT input_tokens + cache_creation_tokens + cache_read_tokens FROM session_turn_usage
                 WHERE session_id = ?1 AND subagent = 0 ORDER BY at DESC, message_id DESC LIMIT 1",
                params![session_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map(|found| found.map(|tokens| tokens.max(0) as u64))
            .map_err(reading)
    }

    /// Keep that Fleet compacted a waiting Job. `at` is kept in the turns' own stamp, so the turn
    /// after it is found by comparing the two.
    pub fn keep_waiting_compaction(&mut self, session_id: &str, at: &str, context_before: u64) -> Result<(), WriteError> {
        self.conn
            .execute(
                &format!(
                    "INSERT OR IGNORE INTO waiting_compactions (session_id, at, context_before)
                     VALUES (?1, strftime({STAMP}, ?2), ?3)"
                ),
                params![session_id, at, i64::try_from(context_before).unwrap_or(i64::MAX)],
            )
            .map(|_| ())
            .map_err(fault("keeping a waiting job's compaction"))
            .map_err(WriteError::Database)
    }

    /// Every compaction of a Job, oldest first.
    pub fn waiting_compactions(&self, session_id: &str) -> Result<Vec<KeptCompaction>, WriteError> {
        let mut statement = self
            .conn
            .prepare(&format!("SELECT {COLUMNS} FROM waiting_compactions WHERE session_id = ?1 ORDER BY at"))
            .map_err(reading)?;
        let rows = statement.query_map(params![session_id], kept).map_err(reading)?;
        rows.collect::<Result<_, _>>().map_err(reading)
    }

    /// Fill each of the Job's compactions that has none with its first main-thread turn after it,
    /// where one has been read.
    pub fn waiting_compactions_measured(&mut self, session_id: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "UPDATE waiting_compactions
                 SET (next_at, next_input, next_cache_write, next_cache_read) = (
                     SELECT turn.at, turn.input_tokens, turn.cache_creation_tokens, turn.cache_read_tokens
                     FROM session_turn_usage AS turn
                     WHERE turn.session_id = waiting_compactions.session_id AND turn.subagent = 0
                       AND turn.at > waiting_compactions.at
                     ORDER BY turn.at, turn.message_id LIMIT 1
                 )
                 WHERE session_id = ?1 AND next_at IS NULL AND EXISTS (
                     SELECT 1 FROM session_turn_usage AS turn
                     WHERE turn.session_id = waiting_compactions.session_id AND turn.subagent = 0
                       AND turn.at > waiting_compactions.at
                 )",
                params![session_id],
            )
            .map(|_| ())
            .map_err(fault("keeping the request after a waiting job's compaction"))
            .map_err(WriteError::Database)
    }
}
