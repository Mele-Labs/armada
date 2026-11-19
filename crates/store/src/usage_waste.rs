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
//!
//! **And by every request after it in the same thread, until the thread's next compaction**, which read it from
//! the cache again: its tokens, the request's model weight and the cache-read weight. That is taken from the
//! later request's own burn, after its own results and before the context kinds, and no more than it read.

use std::collections::{BTreeMap, HashMap, HashSet};

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
    /// The thread that read it and the compactions that thread had had, which name the context it stays in.
    thread: String,
    epoch: i64,
}

/// The calls the window's requests carried results of that are waste, with the window on the request that
/// carried them. A repeat is a Read of lines of a file the same thread, since its last compaction, had read
/// before, whose result had reached it, with no edit of the file by the thread between. A Read with no lines
/// is the whole file. Takes the window as `?1` and `?2`.
///
/// **Each table is named its index.** A store with no `sqlite_stat1`, which is every store, plans these lookups by
/// the primary key's session alone, and one session holds a third of all calls, so every repeat check read that
/// session's calls again: 88 s for a week on a copy of a live store, 0.09 s with the indexes.
const FLAGGED: &str = "SELECT session_id, message_id, tool, path, bytes, repeated, thread, epoch FROM (
        SELECT c.session_id AS session_id, c.carried_by AS message_id, c.tool AS tool, c.path AS path, c.result_bytes AS bytes,
               c.thread AS thread, c.epoch AS epoch,
               (c.path IS NOT NULL AND EXISTS (
                   SELECT 1 FROM session_tool_calls e INDEXED BY session_tool_calls_by_path
                    WHERE e.session_id = c.session_id AND e.thread = c.thread AND e.epoch = c.epoch AND e.path = c.path
                      AND e.result_bytes IS NOT NULL
                      AND COALESCE(e.from_line, 1) <= COALESCE(c.to_line, 9223372036854775807)
                      AND COALESCE(c.from_line, 1) <= COALESCE(e.to_line, 9223372036854775807)
                      AND (e.at < c.at OR (e.at = c.at AND e.tool_use_id < c.tool_use_id))
                      AND NOT EXISTS (
                          SELECT 1 FROM session_tool_calls x INDEXED BY session_tool_calls_by_edit
                           WHERE x.session_id = c.session_id AND x.thread = c.thread AND x.edited = c.path
                             AND x.at > e.at AND x.at < c.at))) AS repeated
          FROM session_turn_usage t JOIN session_tool_calls c INDEXED BY session_tool_calls_by_carrier ON c.session_id = t.session_id AND c.carried_by = t.message_id
         WHERE t.at >= ?1 AND t.at < ?2 AND c.result_bytes > 0)
     WHERE repeated OR bytes > ";

fn flagged_sql() -> String {
    format!("{FLAGGED}{BIG_RESULT_BYTES}")
}

/// One request, as far as the kinds read it.
#[derive(Default)]
struct Request {
    subagent: bool,
    /// The model's weight, which a token it reads again is weighed by.
    model: f64,
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
    /// Of the result kinds, what each file or tool took of the request's own writes.
    causes: Vec<(WasteKind, String, f64)>,
    /// What reading the earlier results again took of `repeat_reads` and of `big_results`, which is in `units` too.
    again: [f64; 2],
}

/// The tokens of the waste results earlier in a request's thread and context, which it read from the cache again.
#[derive(Clone, Copy, Default)]
struct Earlier {
    repeats: f64,
    bigs: f64,
}

/// A result's tokens: all of a repeat's, and a big result's past [`BIG_RESULT_BYTES`].
fn result_tokens(one: &Flagged) -> f64 {
    one.bytes.saturating_sub(if one.repeated { 0 } else { BIG_RESULT_BYTES }).div_ceil(4) as f64
}

impl Request {
    fn taken(&self, earlier: Earlier) -> Taken {
        let cold = (self.written * self.cold).min(self.burn);
        let start = if self.first_after_compaction {
            (self.read * self.start_read + self.written * self.start_written).min(self.burn - cold)
        } else {
            0.0
        };
        let left = self.burn - cold - start;

        // A repeat's whole result is waste; a big result that is not a repeat is waste past the limit.
        let repeats: Vec<(&Flagged, f64)> = self.results.iter().filter(|one| one.repeated).map(|one| (one, result_tokens(one))).collect();
        let bigs: Vec<(&Flagged, f64)> =
            self.results.iter().filter(|one| !one.repeated && one.bytes > BIG_RESULT_BYTES).map(|one| (one, result_tokens(one))).collect();
        let (repeat_tokens, big_tokens) = (repeats.iter().map(|one| one.1).sum::<f64>(), bigs.iter().map(|one| one.1).sum::<f64>());
        let write_tokens = self.write_tokens as f64;
        let repeat_part = if write_tokens > 0.0 { (repeat_tokens / write_tokens).min(1.0) } else { 0.0 };
        let big_part = if write_tokens > 0.0 { (big_tokens / write_tokens).min(1.0 - repeat_part) } else { 0.0 };
        let repeat = (self.written * repeat_part).min(left);
        let big = (self.written * big_part).min(left - repeat);

        // The results earlier in the thread's context were read from the cache again, at a tenth. What that took is
        // out of the request's cache read less the start-up's part of it, and out of what the two kinds above left.
        let left = left - repeat - big;
        let read_left = (self.read - if self.first_after_compaction { self.read * self.start_read } else { 0.0 }).max(0.0);
        let per_token = self.model * WEIGHT_CACHE_READ / UNIT_SCALE;
        let again_repeat = (earlier.repeats * per_token).min(read_left).min(left);
        let again_big = (earlier.bigs * per_token).min(read_left - again_repeat).min(left - again_repeat);

        let rest = left - again_repeat - again_big;
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
            units: [cold, start, repeat + again_repeat, big + again_big, sub, main],
            occurrences: [1, 1, repeats.len() as u32, bigs.len() as u32, 1, 1],
            causes,
            again: [again_repeat, again_big],
        }
    }
}

fn flagged(conn: &Connection, since: &str, until: &str) -> Result<HashMap<(String, String), Vec<Flagged>>, WriteError> {
    let mut statement = conn.prepare(&flagged_sql()).map_err(reading)?;
    let rows = statement
        .query_map([since, until], |row| {
            Ok((
                (row.get::<_, String>(0)?, row.get::<_, String>(1)?),
                Flagged {
                    tool: row.get(2)?,
                    path: row.get(3)?,
                    bytes: row.get::<_, i64>(4)?.max(0) as u64,
                    repeated: row.get(5)?,
                    thread: row.get(6)?,
                    epoch: row.get(7)?,
                },
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

/// One row of a window's stream: a main or subagent request, or a compaction's own request.
struct Row {
    session_id: String,
    message_id: String,
    at: String,
    background: bool,
    subagent: bool,
    spent: bool,
    request: Request,
}

/// A request that took part of a kind, with what it took.
struct Counted {
    session_id: String,
    context: u64,
    taken: Taken,
}

/// The requests the kinds took from, and what reading results again took, by file or tool.
struct Walk {
    counted: Vec<Counted>,
    /// A Job, a kind and a file or tool, with the units later requests spent reading what it put in the context.
    again: Vec<(WasteKind, String, String, f64)>,
}

/// The waste results in one thread's context, and what reading them again has cost so far.
///
/// A result's cost is the sum, over the requests after the one that carried it, of its tokens at the cache-read
/// weight. `paid_*` is the cost of one token of all the results held so far, summed over the requests since, so each
/// result is settled by the part of it that came after the result was added, with no list walked for any request.
#[derive(Default)]
struct Held {
    repeats: f64,
    bigs: f64,
    paid_repeats: f64,
    paid_bigs: f64,
    /// Each result, its tokens and `paid_*` when it was added.
    results: Vec<(WasteKind, String, f64, f64)>,
}

impl Held {
    fn earlier(&self) -> Earlier {
        Earlier { repeats: self.repeats, bigs: self.bigs }
    }

    /// A request read the held results again and `taken` them.
    fn pay(&mut self, again: [f64; 2]) {
        if self.repeats > 0.0 {
            self.paid_repeats += again[0] / self.repeats;
        }
        if self.bigs > 0.0 {
            self.paid_bigs += again[1] / self.bigs;
        }
    }

    fn hold(&mut self, one: &Flagged) {
        let tokens = result_tokens(one);
        if one.repeated {
            self.results.push((WasteKind::RepeatReads, one.path.clone().unwrap_or_default(), tokens, self.paid_repeats));
            self.repeats += tokens;
        } else {
            self.results.push((WasteKind::BigResults, one.tool.clone(), tokens, self.paid_bigs));
            self.bigs += tokens;
        }
    }

    /// The context ended, at a compaction or the end of the window: what each result cost is final.
    fn settle(self, session: &str, into: &mut Vec<(WasteKind, String, String, f64)>) {
        for (kind, name, tokens, paid) in self.results {
            let now = if kind == WasteKind::RepeatReads { self.paid_repeats } else { self.paid_bigs };
            if tokens * (now - paid) > 0.0 {
                into.push((kind, session.to_string(), name, tokens * (now - paid)));
            }
        }
    }
}

/// The Jobs whose last main-thread request before `since` was followed by a compaction, so the first main-thread
/// request after `since` is a fresh start.
fn compacted_before(conn: &Connection, since: &str) -> Result<HashSet<String>, WriteError> {
    let mut statement = conn
        .prepare(
            "SELECT DISTINCT b.session_id FROM session_background_usage b
              WHERE b.at < ?1
                AND b.at >= COALESCE((SELECT max(m.at) FROM session_turn_usage m
                                       WHERE m.session_id = b.session_id AND NOT m.subagent AND m.at < ?1), '')",
        )
        .map_err(reading)?;
    let rows = statement.query_map([since], |row| row.get::<_, String>(0)).map_err(reading)?;
    rows.collect::<Result<_, _>>().map_err(reading)
}

/// The thread and compactions of each subagent request that made a call in the window. A subagent's requests are
/// not otherwise told apart, and one that made no call (its last, as a rule) is left out.
fn subagent_threads(conn: &Connection, since: &str, until: &str) -> Result<HashMap<(String, String), (String, i64)>, WriteError> {
    let mut statement = conn
        .prepare("SELECT DISTINCT session_id, message_id, thread, epoch FROM session_tool_calls WHERE thread <> '' AND at >= ?1 AND at < ?2")
        .map_err(reading)?;
    let rows = statement
        .query_map([since, until], |row| Ok(((row.get::<_, String>(0)?, row.get::<_, String>(1)?), (row.get::<_, String>(2)?, row.get::<_, i64>(3)?))))
        .map_err(reading)?;
    rows.collect::<Result<_, _>>().map_err(reading)
}

/// Each request that could be waste in `[since, until)`: past the context limit, with cold writes, the
/// first main-thread one after a compaction, one that carried a result that was waste, or one that read an
/// earlier such result again. A compaction's own request is the fix, not the waste.
///
/// **The window's requests are read once, in time order a Job at a time**, and a fresh start is found by walking
/// them: a main-thread request with shares kept is a start if a compaction came after the Job's last main-thread
/// request before it. Requests at one instant are read together, so none of them is the start of another.
/// The same walk holds the waste results of each thread's context, and a compaction empties the main thread's.
fn requests(conn: &Connection, since: &str, until: &str) -> Result<Walk, WriteError> {
    let model = model_weight();
    let mut carried = flagged(conn, since, until)?;
    let compacted = compacted_before(conn, since)?;
    let threads = subagent_threads(conn, since, until)?;
    let sql = format!(
        "SELECT session_id, message_id, at, subagent, background, u, ctx, wt, w, r, cold, start_r, start_w, spent, mw FROM (
            SELECT session_id, message_id, at, subagent, 0 AS background, {units} AS u,
                   (input_tokens + cache_creation_tokens + cache_read_tokens) AS ctx,
                   (input_tokens + cache_creation_tokens) AS wt,
                   {model} * {written} / {UNIT_SCALE:?} AS w,
                   {model} * {WEIGHT_CACHE_READ:?} * cache_read_tokens / {UNIT_SCALE:?} AS r,
                   COALESCE(json_extract(spent_on, '$.write.cold_cache'), 0) AS cold,
                   COALESCE(json_extract(spent_on, '$.read.start_up'), 0) AS start_r,
                   COALESCE(json_extract(spent_on, '$.write.start_up'), 0) AS start_w,
                   spent_on IS NOT NULL AS spent, {model} AS mw
              FROM session_turn_usage WHERE at >= ?1 AND at < ?2
            UNION ALL
            SELECT session_id, '', at, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
              FROM session_background_usage WHERE at >= ?1 AND at < ?2)
          ORDER BY session_id, at, background, message_id",
        units = units(),
        written = written(),
    );
    let mut statement = conn.prepare(&sql).map_err(reading)?;
    let rows = statement
        .query_map([since, until], |row| {
            Ok(Row {
                session_id: row.get(0)?,
                message_id: row.get(1)?,
                at: row.get(2)?,
                background: row.get(4)?,
                subagent: row.get(3)?,
                spent: row.get(13)?,
                request: Request {
                    subagent: row.get(3)?,
                    model: row.get(14)?,
                    burn: row.get(5)?,
                    context: row.get::<_, i64>(6)?.max(0) as u64,
                    write_tokens: row.get::<_, i64>(7)?.max(0) as u64,
                    written: row.get(8)?,
                    read: row.get(9)?,
                    cold: row.get(10)?,
                    start_read: row.get(11)?,
                    start_written: row.get(12)?,
                    first_after_compaction: false,
                    results: Vec::new(),
                },
            })
        })
        .map_err(reading)?;
    let mut all: Vec<Row> = rows.collect::<Result<_, _>>().map_err(reading)?;

    let mut walk = Walk { counted: Vec::new(), again: Vec::new() };
    let mut at = 0;
    while at < all.len() {
        let session = all[at].session_id.clone();
        let end = all[at..].iter().position(|one| one.session_id != session).map_or(all.len(), |n| at + n);
        walk_session(&mut all[at..end], compacted.contains(&session), &mut carried, &threads, &mut walk);
        at = end;
    }
    Ok(walk)
}

/// One Job's requests in time order.
fn walk_session(
    rows: &mut [Row],
    mut compacted: bool,
    carried: &mut HashMap<(String, String), Vec<Flagged>>,
    threads: &HashMap<(String, String), (String, i64)>,
    walk: &mut Walk,
) {
    let session = rows[0].session_id.clone();
    let mut main = Held::default();
    let mut subagents: HashMap<(String, i64), Held> = HashMap::new();
    let mut at = 0;
    while at < rows.len() {
        let end = rows[at..].iter().position(|one| one.at != rows[at].at).map_or(rows.len(), |n| at + n);
        let started = compacted;
        for one in &mut rows[at..end] {
            if one.background {
                continue;
            }
            let mut request = std::mem::take(&mut one.request);
            request.first_after_compaction = started && !request.subagent && one.spent;
            request.results = carried.remove(&(one.session_id.clone(), one.message_id.clone())).unwrap_or_default();
            let key = if request.subagent { threads.get(&(one.session_id.clone(), one.message_id.clone())).cloned() } else { None };
            let held = if request.subagent { key.as_ref().and_then(|key| subagents.get_mut(key)) } else { Some(&mut main) };
            let earlier = held.as_ref().map_or_else(Earlier::default, |held| held.earlier());
            if request.context > LONG_CONTEXT
                || request.cold > 0.0
                || request.first_after_compaction
                || !request.results.is_empty()
                || earlier.repeats > 0.0
                || earlier.bigs > 0.0
            {
                let taken = request.taken(earlier);
                if let Some(held) = held {
                    held.pay(taken.again);
                }
                walk.counted.push(Counted { session_id: session.clone(), context: request.context, taken });
            }
            // What it carried joins the context it was read in, for the requests after it.
            for result in &request.results {
                let held = if result.thread.is_empty() { &mut main } else { subagents.entry((result.thread.clone(), result.epoch)).or_default() };
                held.hold(result);
            }
        }
        if rows[at..end].iter().any(|one| !one.background && !one.subagent) {
            compacted = false;
        }
        if rows[at..end].iter().any(|one| one.background) {
            compacted = true;
            std::mem::take(&mut main).settle(&session, &mut walk.again);
        }
        at = end;
    }
    main.settle(&session, &mut walk.again);
    for (_, held) in subagents {
        held.settle(&session, &mut walk.again);
    }
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
        let Walk { counted, again } = requests(conn, &since, &until)?;
        for request in counted {
            let taken = request.taken;
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
        for (kind, session_id, name, units) in again {
            *causes.entry((kind, session_id)).or_default().entry(name).or_default() += units;
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
