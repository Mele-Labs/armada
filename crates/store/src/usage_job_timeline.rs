//! One Job's burn over its life: bars by bucket of time, the main thread's context, the compactions, and
//! the pauses between its requests. `docs/concepts/session/where-the-tokens-went.md`, *A Job's burn over time*.
//!
//! **Burn is weighed as `usage_summary` weighs it** ([`units`]), over `usage_turns`, so a compaction's own
//! request is burn of the main thread and `total_units` is the summary's for the Job. The rows are read in one
//! pass in time order and everything else is made from them here, so no unit is counted twice.
//!
//! **A pause is a gap of more than [`PAUSE_AFTER_SECONDS`] between two main-thread requests.** What it waited on is
//! the last call the request before it made, which `session_turn_usage.tools` holds in the order they were made.
//! It is cold where the request after it found the cache cold, which is the rule `usage_waste` takes `cold_cache` by.

use rusqlite::{params, Connection};

use crate::error::{fault, WriteError};
use crate::open::Store;
use crate::usage_summary::{units, STAMP};

/// A gap between two main-thread requests past this, in seconds, is a pause. The cache holds five minutes.
pub const PAUSE_AFTER_SECONDS: f64 = 300.0;
/// The most points the context line carries.
pub const MOST_CONTEXT_POINTS: usize = 400;
/// The sizes a bucket is, in seconds: 1, 5, 10, 15, 30 and 60 minutes.
const BUCKET_SIZES: [u32; 6] = [60, 300, 600, 900, 1800, 3600];
/// The most buckets a Job's span is cut into, where a size in [`BUCKET_SIZES`] allows it.
const MOST_BUCKETS: f64 = 120.0;
/// The days the week's burn reaches back.
const WEEK_SECONDS: f64 = 7.0 * 86_400.0;

/// What a pause waited on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaitedOn {
    /// The turn ended with no call pending: the Job was waiting for a person.
    You,
    /// The last call was a shell command.
    Command,
    /// The last call was a subagent.
    Subagent,
    Other,
}

impl WaitedOn {
    pub fn id(self) -> &'static str {
        match self {
            WaitedOn::You => "you",
            WaitedOn::Command => "command",
            WaitedOn::Subagent => "subagent",
            WaitedOn::Other => "other",
        }
    }

    /// What a request's last call, if it made one, says the Job waited on.
    fn of(last_call: Option<&str>) -> WaitedOn {
        match last_call {
            None => WaitedOn::You,
            Some("Bash") => WaitedOn::Command,
            Some("Agent" | "Task") => WaitedOn::Subagent,
            Some(_) => WaitedOn::Other,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BucketRead {
    /// Where the bucket begins.
    pub at: String,
    pub main_units: f64,
    pub subagent_units: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PauseRead {
    pub from: String,
    pub to: String,
    pub waited_on: WaitedOn,
    pub cold: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct JobTimelineRead {
    pub started: String,
    pub ended: Option<String>,
    pub bucket_seconds: u32,
    pub total_units: f64,
    /// Every Job's burn over the seven days ending where this Job's span does.
    pub week_units: f64,
    /// Every bucket from `started` to the end of the span, one with no burn included.
    pub buckets: Vec<BucketRead>,
    /// The main thread's context at each request, thinned to [`MOST_CONTEXT_POINTS`].
    pub context: Vec<(String, u64)>,
    pub compactions: Vec<String>,
    pub pauses: Vec<PauseRead>,
    pub peak_context: u64,
    pub cold_wakes: u32,
    /// The part of `total_units` subagents burned.
    pub subagent_share: f64,
}

fn reading(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("reading a Job's burn over time")(why))
}

/// One request of the Job, or a compaction's own.
struct Row {
    seconds: f64,
    at: String,
    subagent: bool,
    background: bool,
    units: f64,
    context: u64,
    last_call: Option<String>,
    cold: bool,
}

/// The size of a bucket for a span of `seconds`: the smallest of [`BUCKET_SIZES`] that keeps it to
/// [`MOST_BUCKETS`], and the largest where none does.
pub(crate) fn bucket_seconds(seconds: f64) -> u32 {
    BUCKET_SIZES.into_iter().find(|size| seconds / f64::from(*size) <= MOST_BUCKETS).unwrap_or(BUCKET_SIZES[BUCKET_SIZES.len() - 1])
}

/// At most [`MOST_CONTEXT_POINTS`] of `points`, the largest of each equal share of them, so the peak is always kept.
pub(crate) fn thinned(points: Vec<(String, u64)>) -> Vec<(String, u64)> {
    let all = points.len();
    if all <= MOST_CONTEXT_POINTS {
        return points;
    }
    (0..MOST_CONTEXT_POINTS)
        .filter_map(|share| points[share * all / MOST_CONTEXT_POINTS..(share + 1) * all / MOST_CONTEXT_POINTS].iter().max_by_key(|one| one.1).cloned())
        .collect()
}

/// The gaps past [`PAUSE_AFTER_SECONDS`] between consecutive main-thread requests.
fn pauses(rows: &[Row]) -> Vec<PauseRead> {
    let mut main = rows.iter().filter(|one| !one.subagent && !one.background);
    let Some(mut before) = main.next() else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for one in main {
        if one.seconds - before.seconds > PAUSE_AFTER_SECONDS {
            found.push(PauseRead {
                from: before.at.clone(),
                to: one.at.clone(),
                waited_on: WaitedOn::of(before.last_call.as_deref()),
                cold: one.cold,
            });
        }
        before = one;
    }
    found
}

fn rows(conn: &Connection, session_id: &str) -> Result<Vec<Row>, WriteError> {
    let sql = format!(
        "SELECT at, unixepoch(at, 'subsec'), subagent, background, {units},
                input_tokens + cache_creation_tokens + cache_read_tokens,
                json_extract(tools, '$[#-1]'),
                COALESCE(json_extract(spent_on, '$.write.cold_cache'), 0) > 0
           FROM usage_turns WHERE session_id = ?1 ORDER BY at, background",
        units = units()
    );
    let mut statement = conn.prepare(&sql).map_err(reading)?;
    let found = statement
        .query_map([session_id], |row| {
            Ok(Row {
                at: row.get(0)?,
                seconds: row.get::<_, Option<f64>>(1)?.unwrap_or(0.0),
                subagent: row.get(2)?,
                background: row.get(3)?,
                units: row.get(4)?,
                context: row.get::<_, i64>(5)?.max(0) as u64,
                last_call: row.get(6)?,
                cold: row.get(7)?,
            })
        })
        .map_err(reading)?;
    found.collect::<Result<_, _>>().map_err(reading)
}

fn seconds_of(conn: &Connection, stamp: &str) -> Result<Option<f64>, WriteError> {
    conn.query_row("SELECT unixepoch(?1, 'subsec')", [stamp], |row| row.get(0)).map_err(reading)
}

fn stamp_of(conn: &Connection, seconds: f64) -> Result<String, WriteError> {
    conn.query_row(&format!("SELECT strftime({STAMP}, ?1, 'unixepoch')"), [seconds], |row| row.get(0)).map_err(reading)
}

impl Store {
    /// The burn of the Job `session_id`, begun at `started_at` and ended at `ended_at` where it has, over its life.
    /// The span runs to `ended_at`, and to `now` where the Job has not ended, and reaches the last request if that is later.
    pub fn usage_job_timeline(
        &self,
        session_id: &str,
        started_at: &str,
        ended_at: Option<&str>,
        now: &str,
    ) -> Result<JobTimelineRead, WriteError> {
        let conn = &self.conn;
        let rows = rows(conn, session_id)?;

        // The span begins at the Job's start, or at its first request where that came first or the start is no time.
        let first = rows.first();
        let begun = seconds_of(conn, started_at)?;
        let (started, from) = match (begun, first) {
            (Some(begun), Some(first)) if first.seconds < begun => (first.at.clone(), first.seconds),
            (Some(begun), _) => (started_at.to_string(), begun),
            (None, Some(first)) => (first.at.clone(), first.seconds),
            (None, None) => (now.to_string(), seconds_of(conn, now)?.unwrap_or(0.0)),
        };
        let from = from.floor();
        let last = rows.last().map_or(from, |one| one.seconds);
        let to = seconds_of(conn, ended_at.unwrap_or(now))?.unwrap_or(last).max(last).max(from);

        let bucket_seconds = bucket_seconds(to - from);
        let size = f64::from(bucket_seconds);
        let count = (((to - from) / size).ceil() as usize).max(1);
        let mut buckets: Vec<BucketRead> = Vec::with_capacity(count);
        for at in 0..count {
            buckets.push(BucketRead { at: stamp_of(conn, from + at as f64 * size)?, main_units: 0.0, subagent_units: 0.0 });
        }

        let (mut total_units, mut subagent_units, mut cold_wakes, mut peak_context) = (0.0, 0.0, 0, 0);
        let (mut context, mut compactions) = (Vec::new(), Vec::new());
        for one in &rows {
            let at = (((one.seconds - from) / size).floor().max(0.0) as usize).min(count - 1);
            total_units += one.units;
            if one.subagent {
                subagent_units += one.units;
                buckets[at].subagent_units += one.units;
                continue;
            }
            buckets[at].main_units += one.units;
            if one.background {
                compactions.push(one.at.clone());
                continue;
            }
            context.push((one.at.clone(), one.context));
            peak_context = peak_context.max(one.context);
            cold_wakes += u32::from(one.cold);
        }

        let week_units = conn
            .query_row(
                &format!("SELECT COALESCE(SUM({}), 0) FROM usage_turns WHERE at >= ?1 AND at <= ?2", units()),
                params![stamp_of(conn, to - WEEK_SECONDS)?, stamp_of(conn, to + 0.001)?],
                |row| row.get::<_, f64>(0),
            )
            .map_err(reading)?;

        Ok(JobTimelineRead {
            started,
            ended: ended_at.map(str::to_string),
            bucket_seconds,
            total_units,
            week_units,
            buckets,
            context: thinned(context),
            compactions,
            pauses: pauses(&rows),
            peak_context,
            cold_wakes,
            subagent_share: if total_units > 0.0 { subagent_units / total_units } else { 0.0 },
        })
    }
}
