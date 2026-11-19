//! The token waste the kept turns can name, with its cost and the Jobs it hit.
//! `docs/concepts/session/where-the-tokens-went.md`, *Waste*.
//!
//! **Each request is read once and its burn handed out in a fixed order**, so no unit is in two
//! kinds and the kinds add up to at most the window's burn: the cold cache's writes first
//! ([`WasteKind::ColdCache`]), then what a fresh start held as start-up ([`WasteKind::FreshStarts`]),
//! then, of what is left, the part of the context past [`LONG_CONTEXT`]. A kind is added by a variant
//! here, a place in [`WasteKind::ALL`] and the sentences Fleet gives it.
//!
//! A request with no shares kept on its row (heard only from the mod, or kept before shares existed)
//! has no cold part and no start-up part, and counts only for the context kinds.

use std::collections::BTreeMap;

use rusqlite::Connection;

use crate::error::{fault, WriteError};
use crate::open::Store;
use crate::usage_spent::SPENT_WINDOWS;
use crate::usage_summary::{model_weight, text, units, written, LONG_CONTEXT, STAMP, TOP_SESSIONS, UNIT_SCALE, WEIGHT_CACHE_READ};

/// A kind of waste, in the order its burn is handed out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WasteKind {
    ColdCache,
    FreshStarts,
    SubagentContext,
    MainContext,
}

impl WasteKind {
    pub const ALL: [WasteKind; 4] = [WasteKind::ColdCache, WasteKind::FreshStarts, WasteKind::SubagentContext, WasteKind::MainContext];

    pub fn id(self) -> &'static str {
        match self {
            WasteKind::ColdCache => "cold_cache",
            WasteKind::FreshStarts => "fresh_starts",
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

/// One request, as far as the kinds read it.
struct Request {
    session_id: String,
    subagent: bool,
    burn: f64,
    context: u64,
    /// The weighed input and writes, and the weighed cache read.
    written: f64,
    read: f64,
    cold: f64,
    start_read: f64,
    start_written: f64,
    first_after_compaction: bool,
}

impl Request {
    /// What each kind takes of this request's burn, in [`WasteKind::ALL`] order.
    fn taken(&self) -> [f64; 4] {
        let cold = (self.written * self.cold).min(self.burn);
        let start = if self.first_after_compaction {
            (self.read * self.start_read + self.written * self.start_written).min(self.burn - cold)
        } else {
            0.0
        };
        let rest = self.burn - cold - start;
        let past = if self.context > LONG_CONTEXT { rest * (self.context - LONG_CONTEXT) as f64 / self.context as f64 } else { 0.0 };
        let (sub, main) = if self.subagent { (past, 0.0) } else { (0.0, past) };
        [cold, start, sub, main]
    }
}

/// Each request that could be waste in `[since, until)`: past the context limit, with cold writes, or
/// the first main-thread one after a compaction. A compaction's own request is the fix, not the waste.
fn requests(conn: &Connection, since: &str, until: &str) -> Result<Vec<Request>, WriteError> {
    let model = model_weight();
    let sql = format!(
        "SELECT session_id, subagent, u, ctx, w, r, cold, start_r, start_w, started FROM (
            SELECT t.session_id AS session_id, t.subagent AS subagent, {units} AS u,
                   (input_tokens + cache_creation_tokens + cache_read_tokens) AS ctx,
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
              FROM usage_turns t WHERE t.at >= ?1 AND t.at < ?2 AND NOT t.background)
          WHERE ctx > {LONG_CONTEXT} OR cold > 0 OR started",
        units = units(),
        written = written(),
    );
    let mut statement = conn.prepare(&sql).map_err(reading)?;
    let rows = statement
        .query_map([since, until], |row| {
            Ok(Request {
                session_id: row.get(0)?,
                subagent: row.get(1)?,
                burn: row.get(2)?,
                context: row.get::<_, i64>(3)?.max(0) as u64,
                written: row.get(4)?,
                read: row.get(5)?,
                cold: row.get(6)?,
                start_read: row.get(7)?,
                start_written: row.get(8)?,
                first_after_compaction: row.get(9)?,
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
        for request in requests(conn, &since, &until)? {
            for (kind, taken) in WasteKind::ALL.into_iter().zip(request.taken()) {
                if taken <= 0.0 {
                    continue;
                }
                let one = by_job.entry((kind, request.session_id.clone())).or_insert_with(|| WasteJobRead {
                    session_id: request.session_id.clone(),
                    ..WasteJobRead::default()
                });
                one.units += taken;
                one.occurrences += 1;
                if kind.is_context() {
                    one.peak_context = Some(one.peak_context.unwrap_or(0).max(request.context));
                }
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
