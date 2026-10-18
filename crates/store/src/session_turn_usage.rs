//! What a session's turns cost, kept a turn at a time. `docs/concepts/session.md`, *Where the tokens
//! went*. **Names and counts only**: a tool is its name, never its input.
//!
//! A turn is one row per session and message id. A harness writes the same message more than once as
//! it streams, the output count growing and each line carrying some of its calls, so a repeat is
//! merged into the row: the larger count of output wins the figures, and the calls are joined
//! without repeating a prefix already held ([`merged`]).

use rusqlite::{params, Connection, OptionalExtension, Row};

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
    /// A subagent answered the turn.
    pub subagent: bool,
    /// What the turn's first call was for, where that call was a shell command.
    pub bash_use: Option<String>,
}

/// `held` with `heard` taken in. **The figures go with the larger output**, a tie to the later
/// hearing. **The calls**: one that begins the other is the longer of the two (a repeat, or the
/// same message heard further on), and any other is joined on the end (another line of the same
/// message). The label stays where the first call was already held, since it is the first call's.
pub(crate) fn merged(held: &KeptTurnUsage, heard: &KeptTurnUsage) -> KeptTurnUsage {
    let mut turn = if heard.output_tokens >= held.output_tokens { heard.clone() } else { held.clone() };
    turn.tools = if heard.tools.starts_with(&held.tools) {
        heard.tools.clone()
    } else if held.tools.starts_with(&heard.tools) {
        held.tools.clone()
    } else {
        held.tools.iter().chain(&heard.tools).cloned().collect()
    };
    turn.bash_use = if held.tools.is_empty() { heard.bash_use.clone() } else { held.bash_use.clone() };
    turn.subagent = held.subagent || heard.subagent;
    turn
}

const COLUMNS: &str = "message_id, at, model, input_tokens, cache_creation_tokens, cache_read_tokens, \
     output_tokens, tools, subagent, bash_use";

fn kept_turn(row: &Row<'_>) -> rusqlite::Result<KeptTurnUsage> {
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
        subagent: row.get(8)?,
        bash_use: row.get(9)?,
    })
}

fn keeping(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("keeping a job's turn usage")(why))
}

fn reading(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("reading a job's turn usage")(why))
}

fn keep_one(conn: &Connection, session_id: &str, turn: &KeptTurnUsage) -> Result<bool, WriteError> {
    let held = conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM session_turn_usage WHERE session_id = ?1 AND message_id = ?2"),
            params![session_id, turn.message_id],
            kept_turn,
        )
        .optional()
        .map_err(reading)?;
    let kept = match &held {
        Some(held) => merged(held, turn),
        None => turn.clone(),
    };
    if held.as_ref() == Some(&kept) {
        return Ok(false);
    }
    let tools = serde_json::to_string(&kept.tools).unwrap_or_else(|_| "[]".into());
    conn.execute(
        "INSERT OR REPLACE INTO session_turn_usage (session_id, message_id, at, model, input_tokens, \
         cache_creation_tokens, cache_read_tokens, output_tokens, tools, subagent, bash_use)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            session_id,
            kept.message_id,
            kept.at,
            kept.model,
            kept.input_tokens as i64,
            kept.cache_creation_tokens as i64,
            kept.cache_read_tokens as i64,
            kept.output_tokens as i64,
            tools,
            kept.subagent,
            kept.bash_use,
        ],
    )
    .map(|_| true)
    .map_err(keeping)
}

impl Store {
    /// Keep one turn. Whether the store changed: not where the turn is held already, whole.
    pub fn keep_turn_usage(&mut self, session_id: &str, turn: &KeptTurnUsage) -> Result<bool, WriteError> {
        keep_one(&self.conn, session_id, turn)
    }

    /// Keep the turns a read of the transcript at `path` found, and note it was read to `read_to`,
    /// in one transaction: a crash between them re-reads the turns, never skips them. How many
    /// turns changed the store.
    pub fn keep_turns_read(
        &mut self,
        session_id: &str,
        turns: &[KeptTurnUsage],
        path: &str,
        read_to: u64,
    ) -> Result<usize, WriteError> {
        let kept = self.conn.transaction().map_err(keeping)?;
        let mut changed = 0;
        for turn in turns {
            changed += usize::from(keep_one(&kept, session_id, turn)?);
        }
        kept.execute(
            "INSERT INTO transcript_offsets (path, read_to) VALUES (?1, ?2)
             ON CONFLICT (path) DO UPDATE SET read_to = excluded.read_to",
            params![path, read_to as i64],
        )
        .map_err(keeping)?;
        kept.commit().map_err(keeping)?;
        Ok(changed)
    }

    /// How far into the transcript at `path` the last read got, if one did.
    pub fn transcript_offset(&self, path: &str) -> Result<Option<u64>, WriteError> {
        self.conn
            .query_row("SELECT read_to FROM transcript_offsets WHERE path = ?1", params![path], |row| {
                row.get::<_, i64>(0).map(|at| at as u64)
            })
            .optional()
            .map_err(reading)
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
            .prepare(&format!(
                "SELECT {COLUMNS} FROM session_turn_usage
                 WHERE session_id = ?1 AND (?2 IS NULL OR at >= ?2) AND (?3 IS NULL OR at < ?3)
                 ORDER BY at, message_id"
            ))
            .map_err(reading)?;
        let rows = statement.query_map(params![session_id, since, until], kept_turn).map_err(reading)?;
        rows.collect::<Result<_, _>>().map_err(reading)
    }
}
