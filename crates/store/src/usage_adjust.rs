//! What a setting would save, and what a change to one did. `docs/concepts/session/where-the-tokens-went.md`,
//! *Settings that change it*.
//!
//! **An estimate is a sum over the window's own requests**, never a guess at what a Job might do: the
//! subagents' writes priced both ways, and the units of the main thread's requests past a point less what the
//! extra compactions cost. Each is `None` where the window holds nothing to price it from.

use std::collections::HashMap;

use rusqlite::Connection;

use crate::error::{fault, WriteError};
use crate::open::Store;
use crate::usage_spent::SPENT_WINDOWS;
use crate::usage_summary::{
    model_weight, text, units, WEIGHT_CACHE_CREATION, WEIGHT_CACHE_CREATION_1H, WEIGHT_CACHE_READ, WEIGHT_INPUT, STAMP, UNIT_SCALE,
};
use crate::usage_waste::{requests, subagent_threads, WasteKind};

/// A cache held for five minutes is read again by a request inside the hour, which the hour's cache keeps.
const HOUR_SECONDS: f64 = 3_600.0;
/// The days before a change its share is read over.
const BEFORE_DAYS: u32 = 7;
/// A change is read after a day, and not before.
const SINCE_AFTER_DAYS: f64 = 1.0;
/// The changes kept in view: those of the last thirty days, newest first, no more than this many.
const MOST_CHANGES: usize = 6;
const CHANGES_FOR_DAYS: u32 = 30;

fn reading(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("pricing a setting from the kept turns")(why))
}

/// What holding every subagent write for an hour is worth over a window, in units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CacheEstimate {
    /// What the cold rewrites after a pause under an hour would have cost less, read from the cache.
    pub saved: f64,
    /// What the other writes would cost more, weighed 2 where they are weighed 1.25.
    pub extra: f64,
    /// The cold rewrites that became reads.
    pub rewrites: u32,
}

/// What compacting the main thread at a point is worth over a window, in units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CompactionEstimate {
    /// The units of the requests past the point, as the part of each request's burn past it.
    pub past: f64,
    /// The long stretches between compactions that went past the point, each of which is compacted once more.
    pub stretches: u32,
    /// What one compaction costs: the average of its own request and of the start after it.
    pub each: f64,
}

/// A change to a setting and what the waste it cuts was before it and since.
#[derive(Clone, Debug, PartialEq)]
pub struct ChangeRead {
    pub key: String,
    pub at: String,
    pub from: String,
    pub to: String,
    pub kind: WasteKind,
    /// The kind's share of the burn in the seven days before `at`.
    pub before_share: f64,
    /// The same share from `at` to now, once a day has passed.
    pub since_share: Option<f64>,
    pub since_days: f64,
}

/// A request of a subagent, as the cache estimate reads it.
struct Subagent {
    session: String,
    message: String,
    secs: f64,
    counted: bool,
    weight: f64,
    input: f64,
    five: f64,
    hour: f64,
    cold: f64,
}

impl Store {
    /// Keep that `key` went from `from` to `to` at `at`. A second change at the same stamp keeps the later.
    pub fn keep_settings_change(&mut self, key: &str, at: &str, from: &str, to: &str) -> Result<(), WriteError> {
        self.conn
            .execute(
                "INSERT INTO settings_changes (key, at, from_value, to_value) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (key, at) DO UPDATE SET to_value = ?4",
                (key, at, from, to),
            )
            .map(|_| ())
            .map_err(fault("keeping a change to a setting"))
            .map_err(WriteError::Database)
    }

    /// What holding a subagent's cache for an hour is worth over the last `window` up to `now`. `None` for a
    /// window that is not one of the three, and where no subagent made a request in it.
    ///
    /// Each subagent request is priced both ways. **A cold rewrite**, a request Spent on gave its writes as
    /// `cold_cache`, whose thread's request before it was less than an hour earlier, is read from the cache
    /// under the hour: its input and writes cost the cache-read weight instead of their own. **Every other
    /// request's writes held five minutes** cost `2 - 1.25` more. A cold request that made no call has no known
    /// thread, so its gap is not known and it is not counted as saved.
    pub fn subagent_cache_estimate(&self, window: &str, now: &str) -> Result<Option<CacheEstimate>, WriteError> {
        let conn = &self.conn;
        let Some((since, until)) = bounds(conn, window, now)? else {
            return Ok(None);
        };
        // The thread's request before the first of the window can be an hour back.
        let from = text(conn, &format!("SELECT strftime({STAMP}, ?1, '-1 hours')"), &[&since])?.unwrap_or_default();
        let threads = subagent_threads(conn, &from, &until)?;
        let sql = format!(
            "SELECT session_id, message_id, julianday(at) * 86400.0, at >= ?3, {model}, input_tokens,
                    cache_creation_tokens - cache_creation_1h_tokens, cache_creation_1h_tokens,
                    COALESCE(json_extract(spent_on, '$.write.cold_cache'), 0)
               FROM session_turn_usage WHERE subagent AND at >= ?1 AND at < ?2 ORDER BY session_id, at",
            model = model_weight()
        );
        let mut statement = conn.prepare(&sql).map_err(reading)?;
        let rows = statement
            .query_map([&from, &until, &since], |row| {
                Ok(Subagent {
                    session: row.get(0)?,
                    message: row.get(1)?,
                    secs: row.get(2)?,
                    counted: row.get(3)?,
                    weight: row.get(4)?,
                    input: row.get::<_, i64>(5)? as f64,
                    five: row.get::<_, i64>(6)?.max(0) as f64,
                    hour: row.get::<_, i64>(7)? as f64,
                    cold: row.get(8)?,
                })
            })
            .map_err(reading)?;
        let mut last: HashMap<(String, String), f64> = HashMap::new();
        let (mut seen, mut estimate) = (false, CacheEstimate { saved: 0.0, extra: 0.0, rewrites: 0 });
        for row in rows {
            let one = row.map_err(reading)?;
            let thread = threads.get(&(one.session.clone(), one.message.clone()));
            let gap = thread.and_then(|(name, _)| last.insert((one.session.clone(), name.clone()), one.secs).map(|before| one.secs - before));
            if !one.counted {
                continue;
            }
            seen = true;
            let tokens = one.input + one.five + one.hour;
            let written = one.weight * (WEIGHT_INPUT * one.input + WEIGHT_CACHE_CREATION * one.five + WEIGHT_CACHE_CREATION_1H * one.hour) / UNIT_SCALE;
            if one.cold > 0.0 && gap.is_some_and(|gap| gap < HOUR_SECONDS) {
                estimate.saved += one.cold * (written - one.weight * WEIGHT_CACHE_READ * tokens / UNIT_SCALE);
                estimate.rewrites += 1;
            } else {
                estimate.extra += one.weight * (WEIGHT_CACHE_CREATION_1H - WEIGHT_CACHE_CREATION) * one.five / UNIT_SCALE;
            }
        }
        Ok(seen.then_some(estimate))
    }

    /// What compacting the main thread at `point` tokens of context is worth over the last `window` up to `now`.
    /// `None` for a window that is not one of the three, and where nothing was compacted in it, so there is no
    /// cost of a compaction to price the extra ones from.
    ///
    /// **A request past the point gives the part of its burn past it**, its burn times the context past the
    /// point over its context, the way `main_context` counts past 300k. **A stretch between compactions** that
    /// held such a request is compacted once more, and each costs what the window's compactions cost on average:
    /// the compaction's own request, and the start after it as `fresh_starts` counts it.
    pub fn compaction_estimate(&self, window: &str, now: &str, point: u64) -> Result<Option<CompactionEstimate>, WriteError> {
        let conn = &self.conn;
        let Some((since, until)) = bounds(conn, window, now)? else {
            return Ok(None);
        };
        let sql = format!(
            "SELECT session_id, background, input_tokens + cache_creation_tokens + cache_read_tokens, {units}
               FROM usage_turns WHERE NOT subagent AND at >= ?1 AND at < ?2 ORDER BY session_id, at, background",
            units = units()
        );
        let mut statement = conn.prepare(&sql).map_err(reading)?;
        let rows = statement
            .query_map([&since, &until], |row| Ok((row.get::<_, String>(0)?, row.get::<_, bool>(1)?, row.get::<_, i64>(2)?.max(0) as u64, row.get::<_, f64>(3)?)))
            .map_err(reading)?;
        let (mut past, mut stretches, mut compactions, mut compacting) = (0.0, 0u32, 0u32, 0.0);
        let (mut session, mut over) = (String::new(), false);
        for row in rows {
            let (id, background, context, burn) = row.map_err(reading)?;
            if id != session {
                session = id;
                over = false;
            }
            if background {
                compactions += 1;
                compacting += burn;
                over = false;
            } else if context > point {
                past += burn * (context - point) as f64 / context as f64;
                if !over {
                    stretches += 1;
                    over = true;
                }
            }
        }
        if compactions == 0 {
            return Ok(None);
        }
        // The start after a compaction, as the waste's walk counts it, averaged over the starts it found.
        let walked = requests(conn, &since, &until)?;
        let fresh = WasteKind::ALL.iter().position(|kind| *kind == WasteKind::FreshStarts).unwrap_or_default();
        let (start_units, starts) = walked
            .counted
            .iter()
            .filter(|one| one.taken.units[fresh] > 0.0)
            .fold((0.0, 0u32), |(units, starts), one| (units + one.taken.units[fresh], starts + 1));
        let start = if starts > 0 { start_units / f64::from(starts) } else { 0.0 };
        Ok(Some(CompactionEstimate { past, stretches, each: compacting / f64::from(compactions) + start }))
    }

    /// The changes to settings kept in the last thirty days up to `now`, newest first and no more than six,
    /// each with the share of the burn the kind it cuts was before and since. `cuts` names the kind a key cuts,
    /// and a key it names none for is left out.
    pub fn settings_changes_read(&self, now: &str, cuts: impl Fn(&str) -> Option<WasteKind>) -> Result<Vec<ChangeRead>, WriteError> {
        let conn = &self.conn;
        let Some(until) = text(conn, &format!("SELECT strftime({STAMP}, ?1)"), &[&now])? else {
            return Ok(Vec::new());
        };
        let from = text(conn, &format!("SELECT strftime({STAMP}, ?1, '-{CHANGES_FOR_DAYS} days')"), &[&until])?.unwrap_or_default();
        let mut statement = conn
            .prepare(
                "SELECT key, at, from_value, to_value, julianday(?2) - julianday(at) FROM settings_changes
                  WHERE at >= ?1 AND at <= ?2 ORDER BY at DESC",
            )
            .map_err(reading)?;
        let kept = statement
            .query_map([&from, &until], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, f64>(4)?))
            })
            .map_err(reading)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(reading)?;
        let mut read = Vec::new();
        for (key, at, from_value, to_value, days) in kept {
            let Some(kind) = cuts(&key) else { continue };
            if read.len() == MOST_CHANGES {
                break;
            }
            let back = text(conn, &format!("SELECT strftime({STAMP}, ?1, '-{BEFORE_DAYS} days')"), &[&at])?.unwrap_or_default();
            let since_share = match days >= SINCE_AFTER_DAYS {
                true => Some(share(conn, kind, &at, &until)?),
                false => None,
            };
            read.push(ChangeRead {
                key,
                at: at.clone(),
                from: from_value,
                to: to_value,
                kind,
                before_share: share(conn, kind, &back, &at)?,
                since_share,
                since_days: days,
            });
        }
        Ok(read)
    }

    /// The window's whole burn, as the waste reads it.
    pub fn burn_of(&self, window: &str, now: &str) -> Result<Option<f64>, WriteError> {
        let Some((since, until)) = bounds(&self.conn, window, now)? else {
            return Ok(None);
        };
        total(&self.conn, &since, &until).map(Some)
    }
}

/// The span of `window` ending at `now`, as stamps.
fn bounds(conn: &Connection, window: &str, now: &str) -> Result<Option<(String, String)>, WriteError> {
    let Some((_, days)) = SPENT_WINDOWS.iter().find(|(id, _)| *id == window) else {
        return Ok(None);
    };
    let Some(until) = text(conn, &format!("SELECT strftime({STAMP}, ?1)"), &[&now])? else {
        return Ok(None);
    };
    let since = text(conn, &format!("SELECT strftime({STAMP}, ?1, '-{days} days')"), &[&until])?.unwrap_or_default();
    Ok(Some((since, until)))
}

fn total(conn: &Connection, since: &str, until: &str) -> Result<f64, WriteError> {
    conn.query_row(&format!("SELECT COALESCE(SUM({}), 0) FROM usage_turns WHERE at >= ?1 AND at < ?2", units()), [since, until], |row| {
        row.get::<_, f64>(0)
    })
    .map_err(reading)
}

/// `kind`'s units over `[since, until)` as a share of that span's whole burn, 0 where there was none.
fn share(conn: &Connection, kind: WasteKind, since: &str, until: &str) -> Result<f64, WriteError> {
    let whole = total(conn, since, until)?;
    if whole <= 0.0 {
        return Ok(0.0);
    }
    let at = WasteKind::ALL.iter().position(|one| *one == kind).unwrap_or_default();
    let units: f64 = requests(conn, since, until)?.counted.iter().map(|one| one.taken.units[at]).sum();
    Ok((units / whole).min(1.0))
}
