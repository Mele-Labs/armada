//! The token waste the kept turns can name, with its cost and the Jobs it hit.
//! `docs/concepts/session/where-the-tokens-went.md`, *Waste*.
//!
//! **Each request is read once and its burn handed out in a fixed order**, so no unit is in two
//! kinds and the kinds add up to at most the window's burn: the cold cache's writes first
//! ([`WasteKind::ColdCache`]), then what a fresh start held as start-up ([`WasteKind::FreshStarts`]),
//! then, of what is left, the results a request carried that were no use ([`WasteKind::RepeatReads`], then
//! [`WasteKind::BigResults`]), and last, of what is left after them, the part of the context past
//! [`LONG_CONTEXT`]. A kind is added by a variant here, a place in [`WasteKind::ALL`] and the sentences
//! Fleet gives it.
//!
//! A request with no shares kept on its row (heard only from the mod, or kept before shares existed)
//! has no cold part and no start-up part, and counts only for the context kinds.
//!
//! **A result is paid for by the request that first carried it**, which `session_tool_calls.carried_by`
//! names. Its tokens (a fourth of its bytes) are that request's write share: the request's weighed input
//! and writes times the part of its input and writes the result was.

use std::collections::{BTreeMap, HashMap};

use rusqlite::Connection;

use crate::error::{fault, WriteError};
use crate::open::Store;
use crate::usage_spent::SPENT_WINDOWS;
use crate::usage_summary::{model_weight, text, units, written, LONG_CONTEXT, STAMP, TOP_SESSIONS, UNIT_SCALE, WEIGHT_CACHE_READ};

/// A result larger than this, in bytes, is a big result: 20 KB. The part past it is the waste.
pub const BIG_RESULT_BYTES: u64 = 20 * 1024;
/// The most causes a Job's share of a kind names.
const TOP_CAUSES: usize = 3;

/// A kind of waste, in the order its burn is handed out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WasteKind {
    ColdCache,
    FreshStarts,
    RepeatReads,
    BigResults,
    SubagentContext,
    MainContext,
}

impl WasteKind {
    pub const ALL: [WasteKind; 6] = [
        WasteKind::ColdCache,
        WasteKind::FreshStarts,
        WasteKind::RepeatReads,
        WasteKind::BigResults,
        WasteKind::SubagentContext,
        WasteKind::MainContext,
    ];

    pub fn id(self) -> &'static str {
        match self {
            WasteKind::ColdCache => "cold_cache",
            WasteKind::FreshStarts => "fresh_starts",
            WasteKind::RepeatReads => "repeat_reads",
            WasteKind::BigResults => "big_results",
            WasteKind::SubagentContext => "subagent_context",
            WasteKind::MainContext => "main_context",
        }
    }

    /// The two kinds that read a request's context, and so carry a peak.
    fn is_context(self) -> bool {
        matches!(self, WasteKind::SubagentContext | WasteKind::MainContext)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WasteJobRead {
    pub session_id: String,
    pub units: f64,
    pub occurrences: u32,
    /// The most context one of the counted requests held; only for the context kinds.
    pub peak_context: Option<u64>,
    /// The three largest causes, largest first: files for `repeat_reads`, tools for `big_results`, and
    /// none for any other kind.
    pub causes: Vec<(String, f64)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WasteKindRead {
    pub kind: WasteKind,
    pub units: f64,
    pub occurrences: u32,
    /// The thirty largest Jobs, largest first.
    pub jobs: Vec<WasteJobRead>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UsageWasteRead {
    pub until: String,
    pub window: String,
    pub since: String,
    /// The window's whole burn, which the usage summary reports for the same span.
    pub total_units: f64,
    /// Largest first; a kind with nothing is left out.
    pub kinds: Vec<WasteKindRead>,
}

fn reading(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("summing the waste in the kept turns")(why))
}

/// A result a request carried that is waste: a read of a file the thread already held, or one over [`BIG_RESULT_BYTES`].
struct Flagged {
    tool: String,
    path: Option<String>,
    bytes: u64,
    repeated: bool,
}

/// The calls the window's requests carried results of that are waste, with the window on the request that
/// carried them. A repeat is a Read of lines of a file the same thread, since its last compaction, had read
/// before, whose result had reached it, with no edit of the file by the thread between. A Read with no lines
/// is the whole file. Takes the window as `?1` and `?2`.
const FLAGGED: &str = "SELECT session_id, message_id, tool, path, bytes, repeated FROM (
        SELECT c.session_id AS session_id, c.carried_by AS message_id, c.tool AS tool, c.path AS path, c.result_bytes AS bytes,
               (c.path IS NOT NULL AND EXISTS (
                   SELECT 1 FROM session_tool_calls e
                    WHERE e.session_id = c.session_id AND e.thread = c.thread AND e.epoch = c.epoch AND e.path = c.path
                      AND e.result_bytes IS NOT NULL
                      AND COALESCE(e.from_line, 1) <= COALESCE(c.to_line, 9223372036854775807)
                      AND COALESCE(c.from_line, 1) <= COALESCE(e.to_line, 9223372036854775807)
                      AND (e.at < c.at OR (e.at = c.at AND e.tool_use_id < c.tool_use_id))
                      AND NOT EXISTS (
                          SELECT 1 FROM session_tool_calls x
                           WHERE x.session_id = c.session_id AND x.thread = c.thread AND x.edited = c.path
                             AND x.at > e.at AND x.at < c.at))) AS repeated
          FROM session_turn_usage t JOIN session_tool_calls c ON c.session_id = t.session_id AND c.carried_by = t.message_id
         WHERE t.at >= ?1 AND t.at < ?2 AND c.result_bytes > 0)
     WHERE repeated OR bytes > ";

fn flagged_sql() -> String {
    format!("{FLAGGED}{BIG_RESULT_BYTES}")
}

/// One request, as far as the kinds read it.
struct Request {
    session_id: String,
    subagent: bool,
    burn: f64,
    context: u64,
    /// The weighed input and writes, and the weighed cache read.
    written: f64,
    read: f64,
    /// The input and cache-write tokens, which the results it carried were part of.
    write_tokens: u64,
    cold: f64,
    start_read: f64,
    start_written: f64,
    first_after_compaction: bool,
    results: Vec<Flagged>,
}

/// What each kind takes of one request, in [`WasteKind::ALL`] order.
struct Taken {
    units: [f64; 6],
    occurrences: [u32; 6],
    /// Of the result kinds, what each file or tool took.
    causes: Vec<(WasteKind, String, f64)>,
}

impl Request {
    fn taken(&self) -> Taken {
        let cold = (self.written * self.cold).min(self.burn);
        let start = if self.first_after_compaction {
            (self.read * self.start_read + self.written * self.start_written).min(self.burn - cold)
        } else {
            0.0
        };
        let left = self.burn - cold - start;

        // A repeat's whole result is waste; a big result that is not a repeat is waste past the limit.
        let tokens = |bytes: u64| bytes.div_ceil(4) as f64;
        let repeats: Vec<(&Flagged, f64)> = self.results.iter().filter(|one| one.repeated).map(|one| (one, tokens(one.bytes))).collect();
        let bigs: Vec<(&Flagged, f64)> = self
            .results
            .iter()
            .filter(|one| !one.repeated && one.bytes > BIG_RESULT_BYTES)
            .map(|one| (one, tokens(one.bytes - BIG_RESULT_BYTES)))
            .collect();
        let (repeat_tokens, big_tokens) = (repeats.iter().map(|one| one.1).sum::<f64>(), bigs.iter().map(|one| one.1).sum::<f64>());
        let write_tokens = self.write_tokens as f64;
        let repeat_part = if write_tokens > 0.0 { (repeat_tokens / write_tokens).min(1.0) } else { 0.0 };
        let big_part = if write_tokens > 0.0 { (big_tokens / write_tokens).min(1.0 - repeat_part) } else { 0.0 };
        let repeat = (self.written * repeat_part).min(left);
        let big = (self.written * big_part).min(left - repeat);

        let rest = left - repeat - big;
        let past = if self.context > LONG_CONTEXT { rest * (self.context - LONG_CONTEXT) as f64 / self.context as f64 } else { 0.0 };
        let (sub, main) = if self.subagent { (past, 0.0) } else { (0.0, past) };

        let mut causes = Vec::new();
        if repeat > 0.0 {
            causes.extend(repeats.iter().map(|(one, tokens)| (WasteKind::RepeatReads, one.path.clone().unwrap_or_default(), repeat * tokens / repeat_tokens)));
        }
        if big > 0.0 {
            causes.extend(bigs.iter().map(|(one, tokens)| (WasteKind::BigResults, one.tool.clone(), big * tokens / big_tokens)));
        }
        Taken {
            units: [cold, start, repeat, big, sub, main],
            occurrences: [1, 1, repeats.len() as u32, bigs.len() as u32, 1, 1],
            causes,
        }
    }
}

fn flagged(conn: &Connection, since: &str, until: &str) -> Result<HashMap<(String, String), Vec<Flagged>>, WriteError> {
    let mut statement = conn.prepare(&flagged_sql()).map_err(reading)?;
    let rows = statement
        .query_map([since, until], |row| {
            Ok((
                (row.get::<_, String>(0)?, row.get::<_, String>(1)?),
                Flagged { tool: row.get(2)?, path: row.get(3)?, bytes: row.get::<_, i64>(4)?.max(0) as u64, repeated: row.get(5)? },
            ))
        })
        .map_err(reading)?;
    let mut by_request: HashMap<(String, String), Vec<Flagged>> = HashMap::new();
    for row in rows {
        let (key, one) = row.map_err(reading)?;
        by_request.entry(key).or_default().push(one);
    }
    Ok(by_request)
}

/// Each request that could be waste in `[since, until)`: past the context limit, with cold writes, the
/// first main-thread one after a compaction, or one that carried a result that was waste. A compaction's
/// own request is the fix, not the waste.
fn requests(conn: &Connection, since: &str, until: &str) -> Result<Vec<Request>, WriteError> {
    let model = model_weight();
    let mut carried = flagged(conn, since, until)?;
    let sql = format!(
        "WITH turns AS (
            SELECT session_id, message_id, at, model, input_tokens, cache_creation_tokens, cache_creation_1h_tokens,
                   cache_read_tokens, output_tokens, subagent, spent_on, 0 AS background
              FROM session_turn_usage
            UNION ALL
            SELECT session_id, NULL, at, model, 0, 0, 0, cache_read_tokens, output_tokens, 0, NULL, 1
              FROM session_background_usage),
          flagged AS ({flagged})
        SELECT session_id, message_id, subagent, u, ctx, wt, w, r, cold, start_r, start_w, started FROM (
            SELECT t.session_id AS session_id, t.message_id AS message_id, t.subagent AS subagent, {units} AS u,
                   (input_tokens + cache_creation_tokens + cache_read_tokens) AS ctx,
                   (input_tokens + cache_creation_tokens) AS wt,
                   {model} * {written} / {UNIT_SCALE:?} AS w,
                   {model} * {WEIGHT_CACHE_READ:?} * cache_read_tokens / {UNIT_SCALE:?} AS r,
                   COALESCE(json_extract(spent_on, '$.write.cold_cache'), 0) AS cold,
                   COALESCE(json_extract(spent_on, '$.read.start_up'), 0) AS start_r,
                   COALESCE(json_extract(spent_on, '$.write.start_up'), 0) AS start_w,
                   (NOT t.subagent AND spent_on IS NOT NULL AND EXISTS (
                       SELECT 1 FROM session_background_usage b
                        WHERE b.session_id = t.session_id AND b.at < t.at
                          AND NOT EXISTS (SELECT 1 FROM session_turn_usage m
                                           WHERE m.session_id = t.session_id AND NOT m.subagent
                                             AND m.at > b.at AND m.at < t.at))) AS started
              FROM turns t WHERE t.at >= ?1 AND t.at < ?2 AND NOT t.background)
          WHERE ctx > {LONG_CONTEXT} OR cold > 0 OR started
             OR (session_id, message_id) IN (SELECT session_id, message_id FROM flagged)",
        flagged = flagged_sql(),
        units = units(),
        written = written(),
    );
    let mut statement = conn.prepare(&sql).map_err(reading)?;
    let rows = statement
        .query_map([since, until], |row| {
            let (session_id, message_id): (String, String) = (row.get(0)?, row.get(1)?);
            Ok(Request {
                results: carried.remove(&(session_id.clone(), message_id)).unwrap_or_default(),
                session_id,
                subagent: row.get(2)?,
                burn: row.get(3)?,
                context: row.get::<_, i64>(4)?.max(0) as u64,
                write_tokens: row.get::<_, i64>(5)?.max(0) as u64,
                written: row.get(6)?,
                read: row.get(7)?,
                cold: row.get(8)?,
                start_read: row.get(9)?,
                start_written: row.get(10)?,
                first_after_compaction: row.get(11)?,
            })
        })
        .map_err(reading)?;
    rows.collect::<Result<_, _>>().map_err(reading)
}

impl Store {
    /// The waste in the last `window` (`day`, `week` or `fortnight`) up to `now`. `None` where the
    /// window is not one of those, or `now` is not a time.
    pub fn usage_waste(&self, window: &str, now: &str) -> Result<Option<UsageWasteRead>, WriteError> {
        let conn = &self.conn;
        let Some((_, days)) = SPENT_WINDOWS.iter().find(|(id, _)| *id == window) else {
            return Ok(None);
        };
        let Some(until) = text(conn, &format!("SELECT strftime({STAMP}, ?1)"), &[&now])? else {
            return Ok(None);
        };
        let since = text(conn, &format!("SELECT strftime({STAMP}, ?1, '-{days} days')"), &[&until])?.unwrap_or_default();
        let total_units = conn
            .query_row(&format!("SELECT COALESCE(SUM({}), 0) FROM usage_turns WHERE at >= ?1 AND at < ?2", units()), [&since, &until], |row| {
                row.get::<_, f64>(0)
            })
            .map_err(reading)?;

        let mut by_job: BTreeMap<(WasteKind, String), WasteJobRead> = BTreeMap::new();
        let mut causes: BTreeMap<(WasteKind, String), BTreeMap<String, f64>> = BTreeMap::new();
        for request in requests(conn, &since, &until)? {
            let taken = request.taken();
            for (at, kind) in WasteKind::ALL.into_iter().enumerate() {
                if taken.units[at] <= 0.0 {
                    continue;
                }
                let one = by_job.entry((kind, request.session_id.clone())).or_insert_with(|| WasteJobRead {
                    session_id: request.session_id.clone(),
                    ..WasteJobRead::default()
                });
                one.units += taken.units[at];
                one.occurrences += taken.occurrences[at];
                if kind.is_context() {
                    one.peak_context = Some(one.peak_context.unwrap_or(0).max(request.context));
                }
            }
            for (kind, name, units) in taken.causes {
                *causes.entry((kind, request.session_id.clone())).or_default().entry(name).or_default() += units;
            }
        }
        for (key, named) in causes {
            let mut named: Vec<(String, f64)> = named.into_iter().collect();
            named.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            named.truncate(TOP_CAUSES);
            if let Some(job) = by_job.get_mut(&key) {
                job.causes = named;
            }
        }

        let mut kinds = Vec::new();
        for kind in WasteKind::ALL {
            let mut jobs: Vec<WasteJobRead> =
                by_job.iter().filter(|((one, _), _)| *one == kind).map(|(_, job)| job.clone()).collect();
            if jobs.is_empty() {
                continue;
            }
            let units = jobs.iter().map(|job| job.units).sum();
            let occurrences = jobs.iter().map(|job| job.occurrences).sum();
            jobs.sort_by(|a, b| b.units.total_cmp(&a.units).then_with(|| a.session_id.cmp(&b.session_id)));
            jobs.truncate(TOP_SESSIONS);
            kinds.push(WasteKindRead { kind, units, occurrences, jobs });
        }
        kinds.sort_by(|a, b| b.units.total_cmp(&a.units).then_with(|| a.kind.cmp(&b.kind)));
        Ok(Some(UsageWasteRead { until, window: window.to_string(), since, total_units, kinds }))
    }
}
