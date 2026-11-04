//! What the burn was spent on, and who spent it, over the last 24 hours, 7 days and 14 days.
//! `docs/concepts/session/where-the-tokens-went.md`, *What the burn was spent on*.
//!
//! **One pass over the 14 days**, grouped by Job and by which of the three stretches a turn fell in,
//! summed in SQL; the windows are those stretches added up. Every figure is weighed as
//! `usage_summary` weighs it, so a window's whole is the summary's for the same span.
//!
//! **Each split adds up to the whole.** A turn's cache read and its writes are shared by the shares
//! kept on its row in `spent_on`. A turn with none has its input shared as the rest of its Job's
//! turns in the window are, or as the window's where the Job has none, and the parts are then scaled
//! to the input they share, so no float is lost.

use std::collections::BTreeMap;

use rusqlite::Connection;

use crate::error::{fault, WriteError};
use crate::open::Store;
use crate::usage_summary::{model_weight, text, units, written, STAMP, TOP_SESSIONS, UNIT_SCALE, WEIGHT_CACHE_READ, WEIGHT_OUTPUT};

/// The nine parts, in the order [`SpentSplit::on`] holds them.
pub const SPENT_ON: [&str; 9] =
    ["start_up", "reads", "own_words", "output", "cold_cache", "announced", "other_results", "messages", "background"];
/// The parts a turn's input is shared among, by its row's `spent_on`, and where each sits in [`SPENT_ON`].
const SHARED: [(&str, usize); 7] =
    [("start_up", 0), ("reads", 1), ("own_words", 2), ("cold_cache", 4), ("announced", 5), ("other_results", 6), ("messages", 7)];
const OUTPUT: usize = 3;
const BACKGROUND: usize = 8;
/// The three, in the order [`SpentSplit::by`] holds them.
pub const SPENT_BY: [&str; 3] = ["main", "subagents", "background"];
/// The windows, shortest first, by id and days back from the end.
pub const SPENT_WINDOWS: [(&str, u32); 3] = [("day", 1), ("week", 7), ("fortnight", 14)];

/// Units twice over: by what they went on, and by who spent them. Each adds up to the whole.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpentSplit {
    pub on: [f64; 9],
    pub by: [f64; 3],
}

impl SpentSplit {
    pub fn units(&self) -> f64 {
        self.on.iter().sum()
    }

    fn add(&mut self, other: &SpentSplit) {
        for (held, more) in self.on.iter_mut().zip(other.on) {
            *held += more;
        }
        for (held, more) in self.by.iter_mut().zip(other.by) {
            *held += more;
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpentWindowRead {
    pub id: String,
    pub since: String,
    pub until: String,
    pub total: SpentSplit,
    /// The thirty largest Jobs, largest first.
    pub jobs: Vec<(String, SpentSplit)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UsageSpentRead {
    pub until: String,
    /// Shortest first.
    pub windows: Vec<SpentWindowRead>,
}

/// One Job's sums over one stretch, before its unshared input is shared out.
#[derive(Clone, Copy, Debug, Default)]
struct Sums {
    units: f64,
    subagents: f64,
    background: f64,
    output: f64,
    unshared: f64,
    /// By [`SHARED`].
    shared: [f64; 7],
}

impl Sums {
    fn add(&mut self, other: &Sums) {
        self.units += other.units;
        self.subagents += other.subagents;
        self.background += other.background;
        self.output += other.output;
        self.unshared += other.unshared;
        for (held, more) in self.shared.iter_mut().zip(other.shared) {
            *held += more;
        }
    }

    /// The split, the unshared input shared as `mix` is where this Job shared none of its own.
    fn split(&self, mix: &[f64; 7]) -> SpentSplit {
        let input = (self.units - self.output - self.background).max(0.0);
        let own: f64 = self.shared.iter().sum();
        let by = if own > 0.0 { &self.shared } else { mix };
        let whole: f64 = by.iter().sum();
        let mut parts = self.shared;
        for (part, weight) in parts.iter_mut().zip(by) {
            *part += if whole > 0.0 { self.unshared * weight / whole } else { 0.0 };
        }
        let sum: f64 = parts.iter().sum();
        if sum > 0.0 {
            parts.iter_mut().for_each(|part| *part *= input / sum);
        } else {
            parts[0] = input;
        }
        let mut on = [0.0; 9];
        for ((_, at), part) in SHARED.iter().zip(parts) {
            on[*at] = part;
        }
        on[OUTPUT] = self.output;
        on[BACKGROUND] = self.background;
        SpentSplit { on, by: [self.units - self.subagents - self.background, self.subagents, self.background] }
    }
}

fn reading(why: rusqlite::Error) -> WriteError {
    WriteError::Database(fault("summing what the burn was spent on")(why))
}

/// Each Job's sums in each stretch: 0 is the last day, 1 the six before it, 2 the week before that.
fn sums(conn: &Connection, bounds: &[String; 4]) -> Result<BTreeMap<String, [Sums; 3]>, WriteError> {
    let share = |side: &str, part: &str| format!("COALESCE(json_extract(spent_on, '$.{side}.{part}'), 0)");
    let shared: Vec<String> = SHARED
        .iter()
        .map(|(part, _)| {
            format!("COALESCE(SUM(CASE WHEN NOT background THEN r * {} + w * {} END), 0)", share("read", part), share("write", part))
        })
        .collect();
    let model = model_weight();
    let sql = format!(
        "SELECT session_id, CASE WHEN at >= ?3 THEN 0 WHEN at >= ?2 THEN 1 ELSE 2 END AS b,
                COALESCE(SUM(u), 0), COALESCE(SUM(CASE WHEN subagent AND NOT background THEN u END), 0),
                COALESCE(SUM(CASE WHEN background THEN u END), 0), COALESCE(SUM(CASE WHEN NOT background THEN o END), 0),
                COALESCE(SUM(CASE WHEN NOT background AND spent_on IS NULL THEN r + w END), 0), {}
           FROM (SELECT session_id, at, subagent, background, spent_on, {units} AS u,
                        {model} * {written} / {UNIT_SCALE:?} AS w,
                        {model} * {WEIGHT_CACHE_READ:?} * cache_read_tokens / {UNIT_SCALE:?} AS r,
                        {model} * {WEIGHT_OUTPUT:?} * output_tokens / {UNIT_SCALE:?} AS o
                   FROM usage_turns WHERE at >= ?1 AND at < ?4)
          GROUP BY session_id, b",
        shared.join(", "),
        units = units(),
        written = written(),
    );
    let mut statement = conn.prepare(&sql).map_err(reading)?;
    let rows = statement
        .query_map(rusqlite::params![bounds[0], bounds[1], bounds[2], bounds[3]], |row| {
            let mut one = Sums {
                units: row.get(2)?,
                subagents: row.get(3)?,
                background: row.get(4)?,
                output: row.get(5)?,
                unshared: row.get(6)?,
                ..Sums::default()
            };
            for (at, part) in one.shared.iter_mut().enumerate() {
                *part = row.get(7 + at)?;
            }
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, one))
        })
        .map_err(reading)?;
    let mut by_job: BTreeMap<String, [Sums; 3]> = BTreeMap::new();
    for row in rows {
        let (id, stretch, one) = row.map_err(reading)?;
        if let Some(held) = by_job.entry(id).or_default().get_mut(stretch as usize) {
            held.add(&one);
        }
    }
    Ok(by_job)
}

impl Store {
    /// What the burn of the last 24 hours, 7 days and 14 days up to `until` (`now` where the caller
    /// names none) was spent on, and who spent it. `None` where `until` is not a time.
    pub fn usage_spent(&self, until: Option<&str>, now: &str) -> Result<Option<UsageSpentRead>, WriteError> {
        let conn = &self.conn;
        let Some(until) = text(conn, &format!("SELECT strftime({STAMP}, ?1)"), &[&until.unwrap_or(now)])? else {
            return Ok(None);
        };
        let back = |days: u32| -> Result<String, WriteError> {
            Ok(text(conn, &format!("SELECT strftime({STAMP}, ?1, '-{days} days')"), &[&until])?.unwrap_or_default())
        };
        let sinces: Vec<String> = SPENT_WINDOWS.iter().map(|(_, days)| back(*days)).collect::<Result<_, _>>()?;
        let by_job = sums(conn, &[sinces[2].clone(), sinces[1].clone(), sinces[0].clone(), until.clone()])?;

        let mut windows = Vec::new();
        for (at, ((id, _), since)) in SPENT_WINDOWS.iter().zip(&sinces).enumerate() {
            let jobs: Vec<(&String, Sums)> = by_job
                .iter()
                .map(|(job, stretches)| {
                    let mut one = Sums::default();
                    stretches[..=at].iter().for_each(|stretch| one.add(stretch));
                    (job, one)
                })
                .filter(|(_, one)| one.units > 0.0)
                .collect();
            let mut mix = [0.0; 7];
            for (_, one) in &jobs {
                for (held, more) in mix.iter_mut().zip(one.shared) {
                    *held += more;
                }
            }
            let mut splits: Vec<(String, SpentSplit)> = jobs.iter().map(|(job, one)| ((*job).clone(), one.split(&mix))).collect();
            let mut total = SpentSplit::default();
            splits.iter().for_each(|(_, one)| total.add(one));
            splits.sort_by(|a, b| b.1.units().total_cmp(&a.1.units()).then_with(|| a.0.cmp(&b.0)));
            splits.truncate(TOP_SESSIONS);
            windows.push(SpentWindowRead { id: (*id).to_string(), since: since.clone(), until: until.clone(), total, jobs: splits });
        }
        Ok(Some(UsageSpentRead { until, windows }))
    }
}
