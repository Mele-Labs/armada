//! What a Job's usage has to look like for Fleet to tell it so, and whether it has been told.
//! `docs/concepts/session.md`, *Where the tokens went*.
//!
//! **Values in, values out.** [`tripped`] reads a Job's kept turns and names the rules they cross;
//! [`Watch`] decides which of those are news. Nothing here reads the store or the clock, so a case
//! is a list of turns and a list of stamps. Fleet's part, reading the turns just ingested and
//! sending the words, is `usage_watch.rs`.
//!
//! **Fleet only ever says it.** No rule starts, forks, moves or ends a Job, and none changes its
//! model: each text is a fact and one action the Job may take or leave.
//!
//! **The main thread is the Job's.** A subagent's turn counts toward the hour's burn and its share,
//! and toward nothing else: its context, its cache and its model are not the Job's.

use std::collections::BTreeMap;
use std::sync::Mutex;

use config::settings::{self as keys, Resolved};
use core_model::Timestamp;
pub use store::KeptTurnUsage;
use store::turn_units;

/// How long after a telling the same rule may speak again, once the figure has fallen under.
pub const COOLDOWN_MINUTES: i64 = 30;
/// An hour's burn has to pass this many cost units before the subagents' share of it means anything.
/// A unit is a million weighted tokens, `store::turn_units`.
const MIN_HOUR_UNITS: f64 = 1.0;
/// Above half the context limit counts while the Job does more than this many turns in an hour.
const BUSY_TURNS: usize = 20;
/// The turns the cache hit rate is read over.
const CACHE_TURNS: usize = 10;
const HOUR_MILLIS: i64 = 3_600_000;
const EDITS: [&str; 4] = ["Edit", "Write", "MultiEdit", "NotebookEdit"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    Context,
    Subagents,
    CacheHit,
    OpusReading,
}

impl Rule {
    pub const ALL: [Rule; 4] = [Rule::Context, Rule::Subagents, Rule::CacheHit, Rule::OpusReading];

    /// The name a trip row keeps.
    pub fn name(self) -> &'static str {
        match self {
            Rule::Context => "context",
            Rule::Subagents => "subagents",
            Rule::CacheHit => "cache_hit",
            Rule::OpusReading => "opus_reading",
        }
    }
}

/// Each rule's limit, or `None` where the rule is switched off.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    /// Tokens of context in the last turn.
    pub context_tokens: Option<u64>,
    /// The subagents' percent of the last hour's burn.
    pub subagent_percent: Option<u32>,
    /// The cache hit percent the last 10 turns must not fall under.
    pub cache_percent: Option<u32>,
    /// Turns of reading on opus.
    pub opus_turns: Option<u32>,
}

impl Limits {
    /// What ships.
    pub fn shipped() -> Limits {
        Limits { context_tokens: Some(300_000), subagent_percent: Some(50), cache_percent: Some(70), opus_turns: Some(20) }
    }

    /// The limits settings.json holds now.
    pub fn of(settings: &Resolved) -> Limits {
        let on = |flag: keys::Flag, limit: keys::Integer| {
            settings.get(flag).then(|| u32::try_from(settings.get(limit)).unwrap_or(0))
        };
        Limits {
            context_tokens: on(keys::CONTEXT_WATCH, keys::CONTEXT_WATCH_KTOKENS).map(|k| u64::from(k) * 1_000),
            subagent_percent: on(keys::SUBAGENT_WATCH, keys::SUBAGENT_WATCH_PERCENT),
            cache_percent: on(keys::CACHE_WATCH, keys::CACHE_WATCH_PERCENT),
            opus_turns: on(keys::OPUS_READING_WATCH, keys::OPUS_READING_WATCH_TURNS),
        }
    }

    pub fn any(&self) -> bool {
        self.context_tokens.is_some() || self.subagent_percent.is_some() || self.cache_percent.is_some() || self.opus_turns.is_some()
    }

    /// How many main-thread turns a reading needs to have, however long ago they were.
    pub fn main_turns_needed(&self) -> usize {
        (self.opus_turns.unwrap_or(0) as usize).max(CACHE_TURNS)
    }
}

/// A rule a Job's turns cross.
#[derive(Clone, Debug, PartialEq)]
pub struct Trip {
    pub rule: Rule,
    /// Tokens, a percent or a count, by rule.
    pub figure: f64,
    /// What the Job is sent.
    pub text: String,
}

fn context(turn: &KeptTurnUsage) -> u64 {
    turn.input_tokens + turn.cache_creation_tokens + turn.cache_read_tokens
}

fn millis(turn: &KeptTurnUsage) -> Option<i64> {
    Timestamp::from_rfc3339(turn.at.clone()).epoch_millis()
}

/// Whether the turn's first call was a read or a search.
fn began_reading(turn: &KeptTurnUsage) -> bool {
    match turn.tools.first().map(String::as_str) {
        Some("Read") => true,
        Some("Bash") => turn.bash_use.as_deref() == Some("read_search"),
        _ => false,
    }
}

/// The turns of the 60 minutes up to the newest, which `turns` ends with.
fn last_hour(turns: &[KeptTurnUsage]) -> Vec<&KeptTurnUsage> {
    let Some(newest) = turns.last().and_then(millis) else { return Vec::new() };
    turns.iter().filter(|turn| millis(turn).is_some_and(|at| at >= newest - HOUR_MILLIS)).collect()
}

/// The burn of the 60 minutes up to the newest turn, in cost units.
pub fn hour_burn(turns: &[KeptTurnUsage]) -> f64 {
    last_hour(turns).into_iter().map(turn_units).sum()
}

/// The rules `turns` cross. `turns` are one Job's, oldest first, and reach back over the last hour
/// and its last [`Limits::main_turns_needed`] main-thread turns.
pub fn tripped(limits: &Limits, turns: &[KeptTurnUsage]) -> Vec<Trip> {
    let main: Vec<&KeptTurnUsage> = turns.iter().filter(|turn| !turn.subagent).collect();
    let hour = last_hour(turns);
    let mut found = Vec::new();

    if let (Some(limit), Some(last)) = (limits.context_tokens, main.last()) {
        let held = context(last);
        let busy = hour.iter().filter(|turn| !turn.subagent).count() > BUSY_TURNS;
        if held > limit || (busy && held > limit / 2) {
            found.push(Trip {
                rule: Rule::Context,
                figure: held as f64,
                text: format!(
                    "Context is {}k tokens. End this thread at the next boundary and carry on in a new one.",
                    (held as f64 / 1_000.0).round()
                ),
            });
        }
    }

    if let Some(limit) = limits.subagent_percent {
        let whole: f64 = hour.iter().map(|turn| turn_units(turn)).sum();
        let subagents: f64 = hour.iter().filter(|turn| turn.subagent).map(|turn| turn_units(turn)).sum();
        if whole > MIN_HOUR_UNITS && subagents / whole * 100.0 > f64::from(limit) {
            let share = subagents / whole * 100.0;
            found.push(Trip {
                rule: Rule::Subagents,
                figure: share,
                text: format!(
                    "Subagents ran {}% of your last hour. Pass model sonnet on reading and searching agents and keep at most two running.",
                    share.round()
                ),
            });
        }
    }

    if let Some(limit) = limits.cache_percent {
        if main.len() >= CACHE_TURNS {
            let recent = &main[main.len() - CACHE_TURNS..];
            let whole: u64 = recent.iter().map(|turn| context(turn)).sum();
            let read: u64 = recent.iter().map(|turn| turn.cache_read_tokens).sum();
            if whole > 0 && (read as f64 / whole as f64 * 100.0) < f64::from(limit) {
                let hit = read as f64 / whole as f64 * 100.0;
                found.push(Trip {
                    rule: Rule::CacheHit,
                    figure: hit,
                    text: format!(
                        "Cache hit rate is {}% over your last {CACHE_TURNS} turns. Something in the prompt or tools changed each turn; check hooks and MCP servers.",
                        hit.round()
                    ),
                });
            }
        }
    }

    if let Some(count) = limits.opus_turns.map(|count| count as usize).filter(|count| *count > 0) {
        if main.len() >= count {
            let recent = &main[main.len() - count..];
            let reading = recent.iter().all(|turn| {
                turn.model.to_lowercase().contains("opus")
                    && began_reading(turn)
                    && !turn.tools.iter().any(|tool| EDITS.contains(&tool.as_str()))
            });
            if reading {
                found.push(Trip {
                    rule: Rule::OpusReading,
                    figure: count as f64,
                    text: format!(
                        "Your last {count} turns on opus only read and searched. Subagents for reading can run on sonnet."
                    ),
                });
            }
        }
    }
    found
}

/// Where a rule stands for one Job.
#[derive(Clone, Copy)]
enum Held {
    /// Told at this instant, and not seen under the limit since.
    Told(i64),
    /// Seen under the limit since being told at this instant.
    Cleared(i64),
}

/// Which Jobs have been told which rules. **Never lost by a restart in the way that matters**: the
/// trip rows say when a Job was last told ([`Watch::remembered`]), and a Job remembered is treated
/// as still over until it is seen under.
#[derive(Default)]
pub struct Watch(Mutex<BTreeMap<(String, Rule), Held>>);

impl Watch {
    fn held(&self) -> std::sync::MutexGuard<'_, BTreeMap<(String, Rule), Held>> {
        self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Whether this Job's standing for `rule` is known, in this process or from a row.
    pub fn knows(&self, session: &str, rule: Rule) -> bool {
        self.held().contains_key(&(session.to_string(), rule))
    }

    /// The Job was told of `rule` at `at`, which a trip row says. Nothing changes where it is known.
    pub fn remembered(&self, session: &str, rule: Rule, at: &Timestamp) {
        self.held().entry((session.to_string(), rule)).or_insert(Held::Told(at.epoch_millis().unwrap_or(0)));
    }

    /// The Job was sent the words for `rule` at `at`.
    pub fn told(&self, session: &str, rule: Rule, at: &Timestamp) {
        self.held().insert((session.to_string(), rule), Held::Told(at.epoch_millis().unwrap_or(0)));
    }

    /// Of the rules `over` the Job is over at `now`, those it has not been told: not yet, or fell under
    /// since and the cooldown has passed. A rule the Job is not over is seen under here.
    pub fn due(&self, session: &str, over: &[Trip], now: &Timestamp) -> Vec<Trip> {
        let now = now.epoch_millis().unwrap_or(0);
        let mut held = self.held();
        let mut due = Vec::new();
        for rule in Rule::ALL {
            let key = (session.to_string(), rule);
            match (over.iter().find(|trip| trip.rule == rule), held.get(&key).copied()) {
                (None, Some(Held::Told(at))) => {
                    held.insert(key, Held::Cleared(at));
                }
                (None, _) => {}
                (Some(trip), None) => due.push(trip.clone()),
                (Some(_), Some(Held::Told(_))) => {}
                (Some(trip), Some(Held::Cleared(at))) => {
                    if now - at >= COOLDOWN_MINUTES * 60_000 {
                        due.push(trip.clone());
                    }
                }
            }
        }
        due
    }
}
