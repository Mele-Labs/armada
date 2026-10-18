//! What the kept turns added up to over a window, summed in SQL so a page gets figures and not rows.
//! `docs/concepts/session.md`, *Where the tokens went*.
//!
//! **Burn is in cost units**: a turn's four token counts weighted by [`WEIGHT_INPUT`],
//! [`WEIGHT_CACHE_CREATION`], [`WEIGHT_CACHE_READ`] and [`WEIGHT_OUTPUT`] and divided by
//! [`UNIT_SCALE`], so a unit is a million input tokens. Days, weekdays and three-hour blocks are the
//! caller's local time: the offset is added to each stamp before it is cut.
//!
//! Every split here is a partition of the same total: a turn is in exactly one model, one first
//! tool (`none` where it called nothing) and one day. `by_bash_use` is the exception, and counts
//! only the turns whose first call was a shell command.

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::{fault, WriteError};
use crate::open::Store;

pub const WEIGHT_INPUT: f64 = 1.0;
pub const WEIGHT_CACHE_CREATION: f64 = 1.25;
pub const WEIGHT_CACHE_READ: f64 = 0.1;
pub const WEIGHT_OUTPUT: f64 = 5.0;
/// Weighted tokens per unit.
pub const UNIT_SCALE: f64 = 1_000_000.0;
/// How many Sessions are listed by name; the rest are one entry.
pub const TOP_SESSIONS: usize = 30;
/// A turn's context counts as above this many tokens in the first bucket, and [`LONG_CONTEXT`] in the second.
const WIDE_CONTEXT: u64 = 200_000;
const LONG_CONTEXT: u64 = 300_000;
/// The window when the caller names no start.
pub(crate) const DEFAULT_DAYS: u32 = 7;
pub(crate) const STAMP: &str = "'%Y-%m-%dT%H:%M:%fZ'";

/// Units under a name.
pub type Named = Vec<(String, f64)>;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DayUsage {
    /// `YYYY-MM-DD`, local.
    pub day: String,
    pub units: f64,
    pub by_model: Named,
}

/// One slice of the turns, summed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UsageSplit {
    pub units: f64,
    pub subagent_units: f64,
    pub above_200k_units: f64,
    pub above_300k_units: f64,
    /// The most input, cache-creation and cache-read tokens any one turn held.
    pub peak_context: u64,
    pub input_units: f64,
    pub cache_creation_units: f64,
    pub cache_read_units: f64,
    pub output_units: f64,
    pub by_model: Named,
    pub by_first_tool: Named,
    pub by_bash_use: Named,
    /// Oldest first; a day with no turn is left out.
    pub by_day: Vec<DayUsage>,
    /// Seven rows, Monday first, of eight three-hour blocks from midnight.
    pub hour_grid: Vec<Vec<f64>>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PriorUsage {
    pub since: String,
    pub until: String,
    pub units: f64,
    pub subagent_units: f64,
    pub above_200k_units: f64,
    /// The last whole local day before the window's end, and the one before it.
    pub last_day: (String, f64),
    pub day_before: (String, f64),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UsageSummaryRead {
    pub since: String,
    pub until: String,
    pub window: UsageSplit,
    /// The largest Sessions by units, largest first.
    pub sessions: Vec<(String, UsageSplit)>,
    /// How many Sessions are past the list, and what they added up to.
    pub rest: Option<(u32, UsageSplit)>,
    pub prior: PriorUsage,
}

fn reading(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("summing a session's turn usage")(why))
}

fn units() -> String {
    format!(
        "(({WEIGHT_INPUT:?} * input_tokens + {WEIGHT_CACHE_CREATION:?} * cache_creation_tokens + \
          {WEIGHT_CACHE_READ:?} * cache_read_tokens + {WEIGHT_OUTPUT:?} * output_tokens) / {UNIT_SCALE:?})"
    )
}

/// One kept turn's burn in cost units, weighted as the SQL sums are.
pub fn turn_units(turn: &crate::session_turn_usage::KeptTurnUsage) -> f64 {
    (WEIGHT_INPUT * turn.input_tokens as f64
        + WEIGHT_CACHE_CREATION * turn.cache_creation_tokens as f64
        + WEIGHT_CACHE_READ * turn.cache_read_tokens as f64
        + WEIGHT_OUTPUT * turn.output_tokens as f64)
        / UNIT_SCALE
}

const CONTEXT: &str = "(input_tokens + cache_creation_tokens + cache_read_tokens)";

/// Which turns: the window, the offset, and all Sessions, only those listed, or all but those.
struct Scope {
    since: String,
    until: String,
    offset: String,
    mode: i64,
    ids: String,
}

impl Scope {
    fn params(&self) -> [&dyn rusqlite::ToSql; 5] {
        [&self.since, &self.until, &self.offset, &self.mode, &self.ids]
    }
}

const FROM: &str = "FROM session_turn_usage WHERE at >= ?1 AND at < ?2 AND (?4 = 0
    OR (?4 = 1 AND session_id IN (SELECT value FROM json_each(?5)))
    OR (?4 = 2 AND session_id NOT IN (SELECT value FROM json_each(?5))))";

fn named(conn: &Connection, scope: &Scope, key: &str, also: &str) -> Result<Named, WriteError> {
    let sql = format!("SELECT {key} AS k, SUM({}) AS u {FROM} {also} GROUP BY k ORDER BY u DESC, k", units());
    let mut statement = conn.prepare(&sql).map_err(reading)?;
    let rows = statement
        .query_map(scope.params(), |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?)))
        .map_err(reading)?;
    rows.collect::<Result<_, _>>().map_err(reading)
}

fn split(conn: &Connection, scope: &Scope) -> Result<UsageSplit, WriteError> {
    let u = units();
    let tokens = |weight: f64, column: &str| format!("COALESCE(SUM({weight:?} * {column}), 0) / {UNIT_SCALE:?}");
    let sql = format!(
        "SELECT COALESCE(SUM({u}), 0),
                COALESCE(SUM(CASE WHEN subagent THEN {u} END), 0),
                COALESCE(SUM(CASE WHEN {CONTEXT} > {WIDE_CONTEXT} THEN {u} END), 0),
                COALESCE(SUM(CASE WHEN {CONTEXT} > {LONG_CONTEXT} THEN {u} END), 0),
                COALESCE(MAX({CONTEXT}), 0), {}, {}, {}, {} {FROM}",
        tokens(WEIGHT_INPUT, "input_tokens"),
        tokens(WEIGHT_CACHE_CREATION, "cache_creation_tokens"),
        tokens(WEIGHT_CACHE_READ, "cache_read_tokens"),
        tokens(WEIGHT_OUTPUT, "output_tokens"),
    );
    let mut one = conn
        .query_row(&sql, scope.params(), |row| {
            Ok(UsageSplit {
                units: row.get(0)?,
                subagent_units: row.get(1)?,
                above_200k_units: row.get(2)?,
                above_300k_units: row.get(3)?,
                peak_context: row.get::<_, i64>(4)? as u64,
                input_units: row.get(5)?,
                cache_creation_units: row.get(6)?,
                cache_read_units: row.get(7)?,
                output_units: row.get(8)?,
                ..UsageSplit::default()
            })
        })
        .map_err(reading)?;
    one.hour_grid = vec![vec![0.0; 8]; 7];
    if one.units == 0.0 {
        return Ok(one);
    }
    one.by_model = named(conn, scope, "model", "")?;
    one.by_first_tool = named(conn, scope, "COALESCE(json_extract(tools, '$[0]'), 'none')", "")?;
    one.by_bash_use = named(conn, scope, "bash_use", "AND bash_use IS NOT NULL")?;

    let sql = format!("SELECT date(at, ?3), model, SUM({u}) {FROM} GROUP BY 1, 2 ORDER BY 1, 2");
    let mut statement = conn.prepare(&sql).map_err(reading)?;
    let rows = statement
        .query_map(scope.params(), |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, f64>(2)?)))
        .map_err(reading)?;
    for row in rows {
        let (day, model, sum) = row.map_err(reading)?;
        if one.by_day.last().map(|last| last.day != day).unwrap_or(true) {
            one.by_day.push(DayUsage { day, ..DayUsage::default() });
        }
        let last = one.by_day.last_mut().expect("pushed above");
        last.units += sum;
        last.by_model.push((model, sum));
    }

    let sql = format!(
        "SELECT (CAST(strftime('%w', at, ?3) AS INTEGER) + 6) % 7, CAST(strftime('%H', at, ?3) AS INTEGER) / 3,
                SUM({u}) {FROM} GROUP BY 1, 2"
    );
    let mut statement = conn.prepare(&sql).map_err(reading)?;
    let rows = statement
        .query_map(scope.params(), |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, f64>(2)?)))
        .map_err(reading)?;
    for row in rows {
        let (weekday, block, sum) = row.map_err(reading)?;
        if let Some(cell) = one.hour_grid.get_mut(weekday as usize).and_then(|row| row.get_mut(block as usize)) {
            *cell += sum;
        }
    }
    Ok(one)
}

pub(crate) fn text(conn: &Connection, sql: &str, args: &[&dyn rusqlite::ToSql]) -> Result<Option<String>, WriteError> {
    conn.query_row(sql, args, |row| row.get::<_, Option<String>>(0)).optional().map(Option::flatten).map_err(reading)
}

impl Store {
    /// The turns in `[since, until)` summed. `until` is `now` and `since` a week before it where the
    /// caller names none. `None` where a bound is not a time. `tz_offset_minutes` is how far the
    /// caller's local time is ahead of UTC.
    pub fn usage_summary(
        &self,
        since: Option<&str>,
        until: Option<&str>,
        now: &str,
        tz_offset_minutes: i32,
    ) -> Result<Option<UsageSummaryRead>, WriteError> {
        let conn = &self.conn;
        let Some(until) = text(conn, &format!("SELECT strftime({STAMP}, ?1)"), &[&until.unwrap_or(now)])? else {
            return Ok(None);
        };
        let since = match since {
            Some(since) => text(conn, &format!("SELECT strftime({STAMP}, ?1)"), &[&since])?,
            None => text(conn, &format!("SELECT strftime({STAMP}, ?1, '-{DEFAULT_DAYS} days')"), &[&until])?,
        };
        let Some(since) = since else {
            return Ok(None);
        };
        let offset = format!("{tz_offset_minutes:+} minutes");
        let scope = |since: &str, until: &str, mode: i64, ids: &str| Scope {
            since: since.into(),
            until: until.into(),
            offset: offset.clone(),
            mode,
            ids: ids.into(),
        };

        let window = split(conn, &scope(&since, &until, 0, "[]"))?;
        let sql = format!(
            "SELECT session_id, SUM({}) AS u {FROM} GROUP BY session_id ORDER BY u DESC, session_id",
            units()
        );
        let all = scope(&since, &until, 0, "[]");
        let mut statement = conn.prepare(&sql).map_err(reading)?;
        let rows = statement.query_map(all.params(), |row| row.get::<_, String>(0)).map_err(reading)?;
        let ids: Vec<String> = rows.collect::<Result<_, _>>().map_err(reading)?;
        let past = ids.len().saturating_sub(TOP_SESSIONS);
        let top = &ids[..ids.len() - past];
        let mut sessions = Vec::new();
        for id in top {
            let only = serde_json::to_string(&[id]).unwrap_or_else(|_| "[]".into());
            sessions.push((id.clone(), split(conn, &scope(&since, &until, 1, &only))?));
        }
        let rest = if past > 0 {
            let top = serde_json::to_string(top).unwrap_or_else(|_| "[]".into());
            Some((past as u32, split(conn, &scope(&since, &until, 2, &top))?))
        } else {
            None
        };
        let prior = prior_of(conn, &since, &until, &offset)?;
        Ok(Some(UsageSummaryRead { since, until, window, sessions, rest, prior }))
    }
}

/// The period of equal length just before `since`, and the last two whole local days before `until`.
fn prior_of(conn: &Connection, since: &str, until: &str, offset: &str) -> Result<PriorUsage, WriteError> {
    let span = conn
        .query_row("SELECT CAST(ROUND((julianday(?2) - julianday(?1)) * 86400) AS INTEGER)", params![since, until], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(reading)?
        .max(0);
    let before = text(conn, &format!("SELECT strftime({STAMP}, ?1, ?2)"), &[&since, &format!("-{span} seconds")])?
        .unwrap_or_else(|| since.to_string());
    let period = split_totals(conn, &Scope { since: before.clone(), until: since.into(), offset: offset.into(), mode: 0, ids: "[]".into() })?;
    let day_of = |back: u32| -> Result<String, WriteError> {
        Ok(text(conn, "SELECT date(?1, ?2, ?3)", &[&until, &offset, &format!("-{back} days")])?.unwrap_or_default())
    };
    let (last, earlier) = (day_of(1)?, day_of(2)?);
    let sql = format!(
        "SELECT date(at, ?3) AS d, SUM({}) FROM session_turn_usage
         WHERE at >= strftime({STAMP}, ?2, '-4 days') AND at < ?2 GROUP BY d",
        units()
    );
    let mut statement = conn.prepare(&sql).map_err(reading)?;
    let rows = statement
        .query_map(params![since, until, offset], |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?)))
        .map_err(reading)?;
    let (mut last_units, mut earlier_units) = (0.0, 0.0);
    for row in rows {
        let (day, sum) = row.map_err(reading)?;
        if day == last {
            last_units = sum;
        } else if day == earlier {
            earlier_units = sum;
        }
    }
    Ok(PriorUsage {
        since: before,
        until: since.into(),
        units: period.units,
        subagent_units: period.subagent_units,
        above_200k_units: period.above_200k_units,
        last_day: (last, last_units),
        day_before: (earlier, earlier_units),
    })
}

/// Only the totals of a slice: what the prior period needs.
fn split_totals(conn: &Connection, scope: &Scope) -> Result<UsageSplit, WriteError> {
    let u = units();
    let sql = format!(
        "SELECT COALESCE(SUM({u}), 0), COALESCE(SUM(CASE WHEN subagent THEN {u} END), 0),
                COALESCE(SUM(CASE WHEN {CONTEXT} > {WIDE_CONTEXT} THEN {u} END), 0) {FROM}"
    );
    conn.query_row(&sql, scope.params(), |row| {
        Ok(UsageSplit { units: row.get(0)?, subagent_units: row.get(1)?, above_200k_units: row.get(2)?, ..UsageSplit::default() })
    })
    .map_err(reading)
}
