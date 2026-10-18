//! What a session's turns cost, kept a turn at a time. `docs/concepts/session.md`, *Where the tokens
//! went*. **Names and counts only**: a tool is its name, never its input.
//!
//! A turn is one row per session and message id. A harness writes the same message more than once as
//! it streams, the output count growing, so a repeat replaces the row only where it carries more
//! output than the one held.

use rusqlite::params;

use crate::error::{fault, WriteError};
use crate::open::Store;

/// One turn's usage. `at` is RFC3339, stamped by the reporter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptTurnUsage {
    pub message_id: String,
    pub at: String,
    pub model: String,
    pub input_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub output_tokens: u64,
    pub tools: Vec<String>,
}

fn keeping(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("keeping a job's turn usage")(why))
}

fn reading(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("reading a job's turn usage")(why))
}

impl Store {
    /// Keep one turn. Whether the store changed: not where the turn is held already, or held with
    /// as much output.
    pub fn keep_turn_usage(&mut self, session_id: &str, turn: &KeptTurnUsage) -> Result<bool, WriteError> {
        let tools = serde_json::to_string(&turn.tools).unwrap_or_else(|_| "[]".into());
        self.conn
            .execute(
                "INSERT INTO session_turn_usage (session_id, message_id, at, model, input_tokens, \
                 cache_creation_tokens, cache_read_tokens, output_tokens, tools)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT (session_id, message_id) DO UPDATE SET
                     at = excluded.at, model = excluded.model, input_tokens = excluded.input_tokens,
                     cache_creation_tokens = excluded.cache_creation_tokens,
                     cache_read_tokens = excluded.cache_read_tokens,
                     output_tokens = excluded.output_tokens, tools = excluded.tools
                 WHERE excluded.output_tokens > session_turn_usage.output_tokens",
                params![
                    session_id,
                    turn.message_id,
                    turn.at,
                    turn.model,
                    turn.input_tokens as i64,
                    turn.cache_creation_tokens as i64,
                    turn.cache_read_tokens as i64,
                    turn.output_tokens as i64,
                    tools,
                ],
            )
            .map(|rows| rows > 0)
            .map_err(keeping)
    }

    /// A session's turns from `since` (inclusive) to `until` (exclusive), oldest first. Either bound
    /// left out is open.
    pub fn turn_usage(
        &self,
        session_id: &str,
        since: Option<&str>,
        until: Option<&str>,
    ) -> Result<Vec<KeptTurnUsage>, WriteError> {
        let mut statement = self
            .conn
            .prepare(
                "SELECT message_id, at, model, input_tokens, cache_creation_tokens, cache_read_tokens, \
                 output_tokens, tools FROM session_turn_usage
                 WHERE session_id = ?1 AND (?2 IS NULL OR at >= ?2) AND (?3 IS NULL OR at < ?3)
                 ORDER BY at, message_id",
            )
            .map_err(reading)?;
        let rows = statement
            .query_map(params![session_id, since, until], |row| {
                let tools: String = row.get(7)?;
                Ok(KeptTurnUsage {
                    message_id: row.get(0)?,
                    at: row.get(1)?,
                    model: row.get(2)?,
                    input_tokens: row.get::<_, i64>(3)? as u64,
                    cache_creation_tokens: row.get::<_, i64>(4)? as u64,
                    cache_read_tokens: row.get::<_, i64>(5)? as u64,
                    output_tokens: row.get::<_, i64>(6)? as u64,
                    tools: serde_json::from_str(&tools).unwrap_or_default(),
                })
            })
            .map_err(reading)?;
        rows.collect::<Result<_, _>>().map_err(reading)
    }
}
