//! What the kept turns, builds and splits add up to, summed in SQL so a page gets figures and not
//! rows, as `usage_summary.rs` does for burn. `docs/concepts/session.md`, *Where a Job's time went*.
//!
//! **Medians are taken in SQL** with window functions, so an hour or a build costs one pass over the
//! turns in it. Seconds throughout the figures; milliseconds in a split, as they are kept. A figure
//! the hour or build did not produce is `None`, so a line breaks there and does not fall to zero.
//! An hour is counted from `since`, which the caller makes the start of one.

use rusqlite::{params, Connection};

use crate::error::{fault, WriteError};
use crate::fleet_builds::{KeptBuildSplit, KeptFleetBuild};
use crate::open::Store;
use crate::usage_summary::{text, Named, STAMP};

/// The calls that return when the owner answers. Their span is his wait, so the fold does not count
/// it as a tool's and the median of a tool's time leaves it out.
pub const OWNER_ASKING_TOOLS: &[&str] = &["AskUserQuestion"];
/// How many hours a read is made of when the caller names none.
pub const DEFAULT_HOURS: u32 = 48;
/// How many days of Jobs a read is made of when the caller names none.
const DEFAULT_DAYS: u32 = 7;
/// How many of the slowest Jobs are listed when the caller names none.
pub const DEFAULT_SLOWEST: usize = 20;

/// The four medians, in seconds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Medians {
    /// The wait on the model per turn of the main thread.
    pub model: Option<f64>,
    /// The time the tools of a turn ran, per turn that called any (not a question to the owner).
    pub tools: Option<f64>,
    /// A Check run's time, queued or running.
    pub checks: Option<f64>,
    /// A pull request's wait in CI and the merge queue with nothing else running.
    pub pull_request: Option<f64>,
}

/// One hour. An hour in which nothing ran is left out of a read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HourFigures {
    /// Hours from `since`.
    pub hour: u32,
    /// The hour's start.
    pub at: String,
    /// Cost units by model, most first.
    pub burn: Named,
    pub medians: Medians,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HoursRead {
    pub since: String,
    pub until: String,
    /// Oldest first.
    pub hours: Vec<HourFigures>,
}

/// Milliseconds in each bucket, summed over Jobs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SplitMs {
    pub model: i64,
    pub tools: i64,
    pub checks: i64,
    pub subagents: i64,
    pub slot: i64,
    pub you: i64,
    pub pull_request: i64,
    pub stopped: i64,
    pub idle: i64,
}

impl SplitMs {
    pub fn total(&self) -> i64 {
        self.model + self.tools + self.checks + self.subagents + self.slot + self.you + self.pull_request + self.stopped + self.idle
    }
}

impl From<KeptBuildSplit> for SplitMs {
    fn from(one: KeptBuildSplit) -> Self {
        SplitMs {
            model: one.model_ms,
            tools: one.tools_ms,
            checks: one.checks_ms,
            subagents: one.subagents_ms,
            slot: one.slot_ms,
            you: one.you_ms,
            pull_request: one.pull_request_ms,
            stopped: one.stopped_ms,
            idle: one.idle_ms,
        }
    }
}

/// One build and what ran on it.
#[derive(Clone, Debug, PartialEq)]
pub struct BuildFigures {
    pub build: KeptFleetBuild,
    /// When the next build began, if one has: the end of this one's run.
    pub until: Option<String>,
    /// Every turn kept while it ran, subagents' included.
    pub turns: u64,
    /// Cost units over those turns.
    pub units: f64,
    pub medians: Medians,
    /// The Jobs that took a turn on it.
    pub jobs: Vec<String>,
    /// The time of every Job in the stretch it ran, cut at its start.
    pub split: SplitMs,
}

/// A Job's split, for the list of the slowest.
#[derive(Clone, Debug, PartialEq)]
pub struct SlowJob {
    pub session_id: String,
    pub title: Option<String>,
    pub workflow: String,
    pub began_at: String,
    pub ended_at: String,
    pub split: SplitMs,
}

/// The usual Job of a workflow: the median milliseconds in each bucket over every kept Job of it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Usual {
    pub jobs: u32,
    pub split: SplitMs,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimeRead {
    pub since: String,
    pub until: String,
    /// Jobs that ended in the window.
    pub jobs: u32,
    pub total: SplitMs,
    /// Longest first.
    pub slowest: Vec<SlowJob>,
    /// By workflow, alphabetically. A Job with no workflow is in none.
    pub usual: Vec<(String, Usual)>,
}

fn reading(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("summing where jobs' time went")(why))
}

/// `(key, median of d)` over the rows `source` selects as `(k, d)`.
fn medians_of(conn: &Connection, source: &str, args: &[&dyn rusqlite::ToSql]) -> Result<Vec<(i64, f64)>, WriteError> {
    let sql = format!(
        "WITH s AS ({source}), r AS (
            SELECT k, d, ROW_NUMBER() OVER (PARTITION BY k ORDER BY d) AS rn, COUNT(*) OVER (PARTITION BY k) AS n FROM s WHERE d IS NOT NULL
         ) SELECT k, AVG(d) FROM r WHERE rn IN ((n + 1) / 2, (n + 2) / 2) GROUP BY k"
    );
    let mut statement = conn.prepare(&sql).map_err(reading)?;
    let rows = statement.query_map(args, |row| Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))).map_err(reading)?;
    rows.collect::<Result<_, _>>().map_err(reading)
}

/// The tools column does not name a call that waits on the owner.
fn not_asking() -> String {
    OWNER_ASKING_TOOLS.iter().map(|name| format!("AND tools NOT LIKE '%\"{name}\"%'")).collect::<Vec<_>>().join(" ")
}

/// A turn's wait on the model, in seconds.
const MODEL_WAIT: &str = "ROUND(unixepoch(t.at, 'subsec') - unixepoch(t.model_began, 'subsec'), 3)";
const TOOLS_RAN: &str = "ROUND(unixepoch(t.tools_ended, 'subsec') - unixepoch(t.tools_began, 'subsec'), 3)";
const CHECK_RAN: &str = "ROUND(unixepoch(r.ended_at, 'subsec') - unixepoch(r.started_at, 'subsec'), 3)";

/// The turns a read is made of, copied once into `read_turns` as `(k, model, session_id, units, md, td)`:
/// the key a figure is grouped by, the burn, and the seconds of the model's wait and the tools' run
/// (null where the turn is a subagent's, has no such span, or its tools wait on the owner). The
/// medians and the burn then come from this one pass, which is most of what a read costs.
/// `prefix` is a `WITH` the key may read from, and `from_where` is `FROM … t WHERE …`.
fn take_turns(conn: &Connection, prefix: &str, key: &str, from_where: &str, args: &[&dyn rusqlite::ToSql]) -> Result<(), WriteError> {
    conn.execute_batch("DROP TABLE IF EXISTS temp.read_turns").map_err(reading)?;
    let asking = not_asking();
    let sql = format!(
        "CREATE TEMP TABLE read_turns AS {prefix} SELECT {key} AS k, t.model AS model, t.session_id AS session_id, {units} AS units,
                CASE WHEN t.subagent = 0 AND t.model_began IS NOT NULL THEN {MODEL_WAIT} END AS md,
                CASE WHEN t.subagent = 0 AND t.tools_began IS NOT NULL AND t.tools_ended IS NOT NULL {asking} THEN {TOOLS_RAN} END AS td
         {from_where}",
        units = crate::usage_summary::units(),
    );
    conn.execute(&sql, args).map_err(reading)?;
    Ok(())
}

/// The four sources of medians: each selects `(k, d)`, the key a figure is grouped by and its seconds.
struct Sources {
    model: String,
    tools: String,
    checks: String,
    pull_requests: String,
}

impl Sources {
    /// The turns' two come from `read_turns`; `runs` and `waits` are `FROM … WHERE …` over Check runs
    /// as `r` and pull request waits as `p`, each keyed by the expression given, after `prefix`.
    fn of(prefix: &str, runs: &str, run_key: &str, waits: &str, wait_key: &str) -> Sources {
        Sources {
            model: "SELECT k, md AS d FROM temp.read_turns".to_string(),
            tools: "SELECT k, td AS d FROM temp.read_turns".to_string(),
            checks: format!("{prefix} SELECT {run_key} AS k, {CHECK_RAN} AS d {runs}"),
            pull_requests: format!("{prefix} SELECT {wait_key} AS k, p.wait_ms / 1000.0 AS d {waits} AND p.wait_ms > 0"),
        }
    }
}

fn all_medians(conn: &Connection, sources: &Sources, args: &[&dyn rusqlite::ToSql]) -> Result<Vec<(i64, Medians)>, WriteError> {
    let mut by_key: std::collections::BTreeMap<i64, Medians> = std::collections::BTreeMap::new();
    for (k, d) in medians_of(conn, &sources.model, &[])? {
        by_key.entry(k).or_default().model = Some(d);
    }
    for (k, d) in medians_of(conn, &sources.tools, &[])? {
        by_key.entry(k).or_default().tools = Some(d);
    }
    for (k, d) in medians_of(conn, &sources.checks, args)? {
        by_key.entry(k).or_default().checks = Some(d);
    }
    for (k, d) in medians_of(conn, &sources.pull_requests, args)? {
        by_key.entry(k).or_default().pull_request = Some(d);
    }
    Ok(by_key.into_iter().collect())
}

fn stamp(conn: &Connection, instant: &str, shift: &str) -> Result<Option<String>, WriteError> {
    text(conn, &format!("SELECT strftime({STAMP}, ?1, ?2)"), &[&instant, &shift])
}

/// The builds that ran at any time after `?1`, each with the start of the next as `stop`.
const CHOSEN: &str = "WITH b AS (
        SELECT id, started_at, LEAD(started_at) OVER (ORDER BY started_at, id) AS until FROM fleet_builds
     ), sel AS (
        SELECT id, started_at, until, COALESCE(until, '9999-12-31T00:00:00.000Z') AS stop FROM b
        WHERE COALESCE(until, '9999-12-31') > ?1
     )";

impl Store {
    /// Where a window of `hours` hours begins: `since` as given, or the start of the hour that makes
    /// the window end with the hour `now` is in.
    fn window_start(&self, since: Option<&str>, hours: u32, now: &str) -> Result<Option<String>, WriteError> {
        match since {
            Some(since) => stamp(&self.conn, since, "+0 seconds"),
            None => text(
                &self.conn,
                &format!("SELECT strftime('%Y-%m-%dT%H:00:00.000Z', ?1, ?2)"),
                &[&now, &format!("-{} hours", hours.saturating_sub(1))],
            ),
        }
    }

    /// The `hours` hours from `since` (which the caller makes the start of one): burn by model and the
    /// four medians in each, the hours that produced nothing left out. Where `since` is not named the
    /// window ends with the hour `now` is in. `None` where `since` is not a time.
    pub fn usage_hours(&self, since: Option<&str>, hours: u32, now: &str) -> Result<Option<HoursRead>, WriteError> {
        let conn = &self.conn;
        let Some(since) = self.window_start(since, hours, now)? else {
            return Ok(None);
        };
        let Some(until) = stamp(conn, &since, &format!("+{hours} hours"))? else {
            return Ok(None);
        };
        let hour_of = |column: &str| format!("CAST((unixepoch({column}) - unixepoch(?1)) / 3600 AS INTEGER)");
        let args: [&dyn rusqlite::ToSql; 2] = [&since, &until];
        take_turns(conn, "", &hour_of("t.at"), "FROM session_turn_usage t WHERE t.at >= ?1 AND t.at < ?2", &args)?;
        let sources = Sources::of(
            "",
            "FROM session_check_runs r WHERE r.ended_at >= ?1 AND r.ended_at < ?2",
            &hour_of("r.ended_at"),
            "FROM job_time_pull_requests p WHERE p.ended_at >= ?1 AND p.ended_at < ?2",
            &hour_of("p.ended_at"),
        );
        let mut found: std::collections::BTreeMap<u32, HourFigures> = std::collections::BTreeMap::new();
        let mut statement = conn
            .prepare("SELECT k, model, SUM(units) AS u FROM temp.read_turns GROUP BY k, model ORDER BY k, u DESC")
            .map_err(reading)?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, f64>(2)?)))
            .map_err(reading)?;
        for row in rows {
            let (hour, model, units) = row.map_err(reading)?;
            if let Ok(hour) = u32::try_from(hour) {
                found.entry(hour).or_default().burn.push((model, units));
            }
        }
        drop(statement);
        for (hour, medians) in all_medians(conn, &sources, &args)? {
            if let Ok(hour) = u32::try_from(hour) {
                found.entry(hour).or_default().medians = medians;
            }
        }
        conn.execute_batch("DROP TABLE IF EXISTS temp.read_turns").map_err(reading)?;
        let mut out = Vec::with_capacity(found.len());
        for (hour, mut one) in found {
            one.hour = hour;
            one.at = stamp(conn, &since, &format!("+{hour} hours"))?.unwrap_or_default();
            out.push(one);
        }
        Ok(Some(HoursRead { since, until, hours: out }))
    }

    /// The builds that ran at any time since `since` (the one running at `since` among them), oldest
    /// first, each with what its whole run took: turns, burn, medians, the Jobs on it and their time
    /// cut at its start. A turn is a build's when it was kept between the build's start and the next's.
    /// `None` where `since` is not a time.
    pub fn usage_builds(&self, since: Option<&str>, now: &str) -> Result<Option<Vec<BuildFigures>>, WriteError> {
        let conn = &self.conn;
        let Some(since) = self.window_start(since, DEFAULT_HOURS, now)? else {
            return Ok(None);
        };
        let args: [&dyn rusqlite::ToSql; 1] = [&since];
        take_turns(conn, CHOSEN, "b.id", "FROM sel b JOIN session_turn_usage t ON t.at >= b.started_at AND t.at < b.stop", &args)?;
        let sources = Sources::of(
            CHOSEN,
            "FROM sel b JOIN session_check_runs r ON r.ended_at >= b.started_at AND r.ended_at < b.stop",
            "b.id",
            "FROM sel b JOIN job_time_pull_requests p ON p.ended_at >= b.started_at AND p.ended_at < b.stop WHERE 1 = 1",
            "b.id",
        );
        let medians: std::collections::BTreeMap<i64, Medians> = all_medians(conn, &sources, &args)?.into_iter().collect();

        let mut figures = Vec::new();
        let sql = format!(
            "{CHOSEN} SELECT b.id, b.until, COALESCE(a.c, 0), COALESCE(a.u, 0), COALESCE(a.j, '')
             FROM sel b LEFT JOIN (
                 SELECT k, COUNT(*) AS c, SUM(units) AS u, GROUP_CONCAT(DISTINCT session_id) AS j FROM temp.read_turns GROUP BY k
             ) a ON a.k = b.id ORDER BY b.started_at, b.id"
        );
        let mut statement = conn.prepare(&sql).map_err(reading)?;
        let rows = statement
            .query_map(args, |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, i64>(2)?, row.get::<_, f64>(3)?, row.get::<_, String>(4)?))
            })
            .map_err(reading)?;
        let mut rest = Vec::new();
        for row in rows {
            rest.push(row.map_err(reading)?);
        }
        drop(statement);
        conn.execute_batch("DROP TABLE IF EXISTS temp.read_turns").map_err(reading)?;
        let kept = self.builds()?;
        for (id, until, turns, units, jobs) in rest {
            let Some(build) = kept.iter().find(|one| one.id == id).cloned() else { continue };
            figures.push(BuildFigures {
                build,
                until,
                turns: turns.max(0) as u64,
                units,
                medians: medians.get(&id).copied().unwrap_or_default(),
                jobs: jobs.split(',').filter(|one| !one.is_empty()).map(str::to_string).collect(),
                split: self.build_split(id)?,
            });
        }
        Ok(Some(figures))
    }

    /// Every Job's time in one build, summed.
    fn build_split(&self, build_id: i64) -> Result<SplitMs, WriteError> {
        self.conn
            .query_row(
                "SELECT COALESCE(SUM(model_ms), 0), COALESCE(SUM(tools_ms), 0), COALESCE(SUM(checks_ms), 0), COALESCE(SUM(subagents_ms), 0),
                        COALESCE(SUM(slot_ms), 0), COALESCE(SUM(you_ms), 0), COALESCE(SUM(pull_request_ms), 0), COALESCE(SUM(stopped_ms), 0),
                        COALESCE(SUM(idle_ms), 0) FROM job_time_builds WHERE build_id = ?1",
                [build_id],
                split_of,
            )
            .map_err(reading)
    }

    /// Where the time of the Jobs that ended in `[since, until)` went (a week back from `until` where
    /// `since` is not named, and `until` is `now` where it is not), the `slowest` of them by length, and
    /// the usual Job of each workflow. `None` where a bound is not a time.
    pub fn usage_time(&self, since: Option<&str>, until: Option<&str>, now: &str, slowest: usize) -> Result<Option<TimeRead>, WriteError> {
        let conn = &self.conn;
        let Some(until) = stamp(conn, until.unwrap_or(now), "+0 seconds")? else {
            return Ok(None);
        };
        let since = match since {
            Some(since) => stamp(conn, since, "+0 seconds")?,
            None => stamp(conn, &until, &format!("-{DEFAULT_DAYS} days"))?,
        };
        let Some(since) = since else {
            return Ok(None);
        };
        let (jobs, total) = conn
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(model_ms), 0), COALESCE(SUM(tools_ms), 0), COALESCE(SUM(checks_ms), 0), COALESCE(SUM(subagents_ms), 0),
                        COALESCE(SUM(slot_ms), 0), COALESCE(SUM(you_ms), 0), COALESCE(SUM(pull_request_ms), 0), COALESCE(SUM(stopped_ms), 0),
                        COALESCE(SUM(idle_ms), 0) FROM job_times WHERE ended_at >= ?1 AND ended_at < ?2",
                params![since, until],
                |row| Ok((row.get::<_, u32>(0)?, split_at(row, 1)?)),
            )
            .map_err(reading)?;
        let mut statement = conn
            .prepare(
                "SELECT j.session_id, s.title, j.workflow, j.began_at, j.ended_at, j.model_ms, j.tools_ms, j.checks_ms, j.subagents_ms, j.slot_ms,
                        j.you_ms, j.pull_request_ms, j.stopped_ms, j.idle_ms
                 FROM job_times j LEFT JOIN sessions s ON s.id = j.session_id WHERE j.ended_at >= ?1 AND j.ended_at < ?2
                 ORDER BY j.model_ms + j.tools_ms + j.checks_ms + j.subagents_ms + j.slot_ms + j.you_ms + j.pull_request_ms + j.stopped_ms + j.idle_ms DESC,
                          j.session_id LIMIT ?3",
            )
            .map_err(reading)?;
        let rows = statement
            .query_map(params![since, until, slowest as i64], |row| {
                Ok(SlowJob {
                    session_id: row.get(0)?,
                    title: row.get(1)?,
                    workflow: row.get(2)?,
                    began_at: row.get(3)?,
                    ended_at: row.get(4)?,
                    split: split_at(row, 5)?,
                })
            })
            .map_err(reading)?;
        let slowest = rows.collect::<Result<Vec<_>, _>>().map_err(reading)?;
        drop(statement);
        Ok(Some(TimeRead { since, until, jobs, total, slowest, usual: self.usual_splits()? }))
    }

    /// The usual Job of each workflow that has a name, from every Job kept of it.
    pub fn usual_splits(&self) -> Result<Vec<(String, Usual)>, WriteError> {
        let conn = &self.conn;
        let unpivot = ["model", "tools", "checks", "subagents", "slot", "you", "pull_request", "stopped", "idle"]
            .iter()
            .enumerate()
            .map(|(at, name)| format!("SELECT workflow, {at} AS bucket, {name}_ms AS d FROM job_times WHERE workflow <> ''"))
            .collect::<Vec<_>>()
            .join(" UNION ALL ");
        let sql = format!(
            "WITH s AS ({unpivot}), r AS (
                SELECT workflow, bucket, d, ROW_NUMBER() OVER (PARTITION BY workflow, bucket ORDER BY d) AS rn,
                       COUNT(*) OVER (PARTITION BY workflow, bucket) AS n FROM s
             ) SELECT workflow, bucket, AVG(d), MAX(n) FROM r WHERE rn IN ((n + 1) / 2, (n + 2) / 2) GROUP BY workflow, bucket ORDER BY workflow, bucket"
        );
        let mut statement = conn.prepare(&sql).map_err(reading)?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, f64>(2)?, row.get::<_, u32>(3)?)))
            .map_err(reading)?;
        let mut usual: Vec<(String, Usual)> = Vec::new();
        for row in rows {
            let (workflow, bucket, median, jobs) = row.map_err(reading)?;
            if usual.last().is_none_or(|(last, _)| *last != workflow) {
                usual.push((workflow.clone(), Usual { jobs, split: SplitMs::default() }));
            }
            let split = &mut usual.last_mut().expect("pushed above").1.split;
            let median = median.round() as i64;
            match bucket {
                0 => split.model = median,
                1 => split.tools = median,
                2 => split.checks = median,
                3 => split.subagents = median,
                4 => split.slot = median,
                5 => split.you = median,
                6 => split.pull_request = median,
                7 => split.stopped = median,
                _ => split.idle = median,
            }
        }
        Ok(usual)
    }
}

/// Nine consecutive columns from `at`, in the order of a split.
fn split_at(row: &rusqlite::Row<'_>, at: usize) -> rusqlite::Result<SplitMs> {
    Ok(SplitMs {
        model: row.get(at)?,
        tools: row.get(at + 1)?,
        checks: row.get(at + 2)?,
        subagents: row.get(at + 3)?,
        slot: row.get(at + 4)?,
        you: row.get(at + 5)?,
        pull_request: row.get(at + 6)?,
        stopped: row.get(at + 7)?,
        idle: row.get(at + 8)?,
    })
}

fn split_of(row: &rusqlite::Row<'_>) -> rusqlite::Result<SplitMs> {
    split_at(row, 0)
}
