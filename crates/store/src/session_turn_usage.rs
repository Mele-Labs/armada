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

/// When one turn's model began and its tools ran, as the transcript stamped them. **Kept apart from
/// [`KeptTurnUsage`]** so a turn heard from a notice, which has none, changes none of it. Each is
/// merged into the row: the earliest start and the latest end win, so a message heard in several
/// reads, or with calls that finish apart, is one stretch.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptTurnTimes {
    pub message_id: String,
    pub model_began: Option<String>,
    pub tools_began: Option<String>,
    pub tools_ended: Option<String>,
}

/// A tool result whose call was read before.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptToolResult {
    pub tool_use_id: String,
    pub at: String,
}

/// A call whose result the read did not reach.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptOpenCall {
    pub tool_use_id: String,
    pub message_id: String,
}

/// How one turn's input is shared among the parts it was spent on, read from its transcript.
/// **Kept apart from [`KeptTurnUsage`]**, as the times are, so a turn heard from a notice changes
/// none of it. `docs/concepts/session/where-the-tokens-went.md`, *What the burn was spent on*.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KeptTurnSpent {
    pub message_id: String,
    /// The part of the cache writes held an hour.
    pub cache_creation_1h_tokens: u64,
    /// Each part's share of the cache read, and of the input and writes; absent where this read
    /// did not see the turn's request, which keeps the shares it was given.
    pub shares: Option<(Vec<(String, f64)>, Vec<(String, f64)>)>,
}

/// A compaction's own request, which no transcript line holds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptBackground {
    pub id: String,
    pub at: String,
    pub model: String,
    pub cache_read_tokens: u64,
    pub output_tokens: u64,
}

/// What a read of a transcript learnt beside the turns.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KeptReading {
    pub times: Vec<KeptTurnTimes>,
    pub late: Vec<KeptToolResult>,
    pub open: Vec<KeptOpenCall>,
    /// The last user line the read reached.
    pub input_at: Option<String>,
    /// One for each turn the read gave.
    pub spent: Vec<KeptTurnSpent>,
    pub background: Vec<KeptBackground>,
    /// What the read hands to the next one, as the reader wrote it.
    pub carried: Option<String>,
}

/// `{"read": {part: share}, "write": {part: share}}`.
fn spent_json(read: &[(String, f64)], write: &[(String, f64)]) -> String {
    let side = |shares: &[(String, f64)]| {
        serde_json::Value::Object(shares.iter().map(|(part, share)| (part.clone(), serde_json::Value::from(*share))).collect())
    };
    serde_json::json!({ "read": side(read), "write": side(write) }).to_string()
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

pub(crate) const COLUMNS: &str = "message_id, at, model, input_tokens, cache_creation_tokens, cache_read_tokens, \
     output_tokens, tools, subagent, bash_use";

pub(crate) fn kept_turn(row: &Row<'_>) -> rusqlite::Result<KeptTurnUsage> {
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

/// Merge `times` into the row of its message: the earliest start and the latest end. SQLite's
/// two-argument `min` and `max` answer NULL when either is, hence the `COALESCE`s.
fn keep_times(conn: &Connection, session_id: &str, times: &KeptTurnTimes) -> Result<(), WriteError> {
    conn.execute(
        "UPDATE session_turn_usage SET
             model_began = COALESCE(min(model_began, ?3), model_began, ?3),
             tools_began = COALESCE(min(tools_began, ?4), tools_began, ?4),
             tools_ended = COALESCE(max(tools_ended, ?5), tools_ended, ?5)
         WHERE session_id = ?1 AND message_id = ?2",
        params![session_id, times.message_id, times.model_began, times.tools_began, times.tools_ended],
    )
    .map(|_| ())
    .map_err(keeping)
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
    // An upsert and not a replace: a replace would drop the times the row already holds.
    conn.execute(
        "INSERT INTO session_turn_usage (session_id, message_id, at, model, input_tokens, \
         cache_creation_tokens, cache_read_tokens, output_tokens, tools, subagent, bash_use)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT (session_id, message_id) DO UPDATE SET
             at = excluded.at, model = excluded.model, input_tokens = excluded.input_tokens,
             cache_creation_tokens = excluded.cache_creation_tokens,
             cache_read_tokens = excluded.cache_read_tokens, output_tokens = excluded.output_tokens,
             tools = excluded.tools, subagent = excluded.subagent, bash_use = excluded.bash_use",
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

    /// Keep the turns a read of the transcript at `path` found, what it learnt of when they ran, and
    /// note it was read to `read_to`, in one transaction: a crash between them re-reads the turns,
    /// never skips them. How many turns changed the store.
    pub fn keep_turns_read(
        &mut self,
        session_id: &str,
        turns: &[KeptTurnUsage],
        learnt: &KeptReading,
        path: &str,
        read_to: u64,
    ) -> Result<usize, WriteError> {
        let kept = self.conn.transaction().map_err(keeping)?;
        let mut changed = 0;
        for turn in turns {
            changed += usize::from(keep_one(&kept, session_id, turn)?);
        }
        // Results first: a message heard again in this read must not lose the call they answer.
        for result in &learnt.late {
            let call: Option<String> = kept
                .query_row(
                    "SELECT message_id FROM session_open_tools WHERE session_id = ?1 AND tool_use_id = ?2",
                    params![session_id, result.tool_use_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(reading)?;
            let Some(message_id) = call else { continue };
            keep_times(
                &kept,
                session_id,
                &KeptTurnTimes { message_id, tools_ended: Some(result.at.clone()), ..Default::default() },
            )?;
            kept.execute(
                "DELETE FROM session_open_tools WHERE session_id = ?1 AND tool_use_id = ?2",
                params![session_id, result.tool_use_id],
            )
            .map_err(keeping)?;
        }
        for times in &learnt.times {
            keep_times(&kept, session_id, times)?;
            // A read from the top finds again a call an earlier read left open.
            kept.execute(
                "DELETE FROM session_open_tools WHERE session_id = ?1 AND message_id = ?2",
                params![session_id, times.message_id],
            )
            .map_err(keeping)?;
        }
        for one in &learnt.spent {
            let shares = one.shares.as_ref().map(|(read, write)| spent_json(read, write));
            kept.execute(
                "UPDATE session_turn_usage SET cache_creation_1h_tokens = min(?3, cache_creation_tokens),
                     spent_on = COALESCE(?4, spent_on)
                 WHERE session_id = ?1 AND message_id = ?2",
                params![session_id, one.message_id, one.cache_creation_1h_tokens as i64, shares],
            )
            .map_err(keeping)?;
        }
        for one in &learnt.background {
            kept.execute(
                "INSERT INTO session_background_usage (session_id, id, at, model, cache_read_tokens, output_tokens)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT (session_id, id) DO UPDATE SET at = excluded.at, model = excluded.model,
                     cache_read_tokens = excluded.cache_read_tokens, output_tokens = excluded.output_tokens",
                params![session_id, one.id, one.at, one.model, one.cache_read_tokens as i64, one.output_tokens as i64],
            )
            .map_err(keeping)?;
        }
        for call in &learnt.open {
            kept.execute(
                "INSERT OR REPLACE INTO session_open_tools (session_id, tool_use_id, message_id) VALUES (?1, ?2, ?3)",
                params![session_id, call.tool_use_id, call.message_id],
            )
            .map_err(keeping)?;
        }
        kept.execute(
            "INSERT INTO transcript_offsets (path, read_to, input_at, carried) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (path) DO UPDATE SET read_to = excluded.read_to, input_at = excluded.input_at,
                 carried = excluded.carried",
            params![path, read_to as i64, learnt.input_at, learnt.carried],
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

    /// The last user line the read of the transcript at `path` reached.
    pub fn transcript_input_at(&self, path: &str) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row("SELECT input_at FROM transcript_offsets WHERE path = ?1", params![path], |row| row.get(0))
            .optional()
            .map(Option::flatten)
            .map_err(reading)
    }

    /// What the last read of the transcript at `path` handed on.
    pub fn transcript_carried(&self, path: &str) -> Result<Option<String>, WriteError> {
        self.conn
            .query_row("SELECT carried FROM transcript_offsets WHERE path = ?1", params![path], |row| row.get(0))
            .optional()
            .map(Option::flatten)
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
