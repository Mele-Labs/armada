//! What a Job's usage has to look like for Fleet to tell it so, and whether it has been told.
//! `docs/concepts/session.md`, *Where the tokens went*.
//!
//! **Values in, values out.** [`tripped`] reads a Job's kept turns and names the rules they cross;
//! [`Following`] decides which of those are news. Nothing here reads the store or the clock, so a case
//! is a list of turns and a list of stamps. Fleet's part, reading the turns just ingested and
//! sending the words, is `usage_following.rs`.
//!
//! **Fleet only ever says it.** No rule starts, forks, moves or ends a Job, and none changes its
//! model: each text is a fact and one action the Job may take or leave. The one exception is the
//! owner's own `plan.contextHandoff` set to carry on automatically, which `usage_following` acts on.
//!
//! **The main thread is the Job's.** A subagent's turn counts toward the hour's burn and toward
//! nothing else: its context is not the Job's.

use std::collections::BTreeSet;
use std::sync::Mutex;

use config::settings::{self as keys, Key, Resolved};
use core_model::Timestamp;
pub use store::{KeptTrip, KeptTurnUsage};
use store::turn_units;

const HOUR_MILLIS: i64 = 3_600_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    Context,
}

impl Rule {
    pub const ALL: [Rule; 1] = [Rule::Context];

    /// The name a trip row keeps.
    pub fn name(self) -> &'static str {
        match self {
            Rule::Context => "context",
        }
    }
}

/// Each rule's limit, or `None` where the rule is switched off.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    /// Tokens of context in the last turn.
    pub context_tokens: Option<u64>,
}

impl Limits {
    /// What ships.
    pub fn shipped() -> Limits {
        Limits { context_tokens: Some(300_000) }
    }

    /// The limits settings.json holds now.
    pub fn of(settings: &Resolved) -> Limits {
        Limits::from_dials(&dials_of(settings))
    }

    /// The limits the dials give, a rule that is off having none.
    pub fn from_dials(dials: &[RuleDial; 1]) -> Limits {
        let limit = |one: &RuleDial| one.on.then_some(one.limit);
        Limits {
            context_tokens: limit(&dials[0]).map(|k| u64::try_from(k).unwrap_or(0) * 1_000),
        }
    }

    pub fn any(&self) -> bool {
        self.context_tokens.is_some()
    }

    /// How many main-thread turns a reading needs to have, however long ago they were.
    pub fn main_turns_needed(&self) -> usize {
        1
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

/// The Job's context in its newest main-thread turn of `turns`, oldest first. `None` with no such turn.
pub fn context_now(turns: &[KeptTurnUsage]) -> Option<u64> {
    turns.iter().rev().find(|turn| !turn.subagent).map(context)
}

fn millis(turn: &KeptTurnUsage) -> Option<i64> {
    Timestamp::from_rfc3339(turn.at.clone()).epoch_millis()
}

/// The turns of the 60 minutes up to the newest, which `turns` ends with.
fn last_hour(turns: &[KeptTurnUsage]) -> Vec<&KeptTurnUsage> {
    let Some(newest) = turns.last().and_then(millis) else { return Vec::new() };
    turns.iter().filter(|turn| millis(turn).is_some_and(|at| at >= newest - HOUR_MILLIS)).collect()
}

/// Cost units per hour of active time over `turns`, oldest first. **Active time is the sum of the
/// gaps between consecutive turns, each counted as at most [`ACTIVE_GAP_CAP_MINUTES`]**, so a Job
/// that stopped for forty minutes and came back has not burned less per hour. `None` where no time
/// passed between the turns.
fn burn_rate(turns: &[&KeptTurnUsage]) -> Option<f64> {
    let stamps: Vec<i64> = turns.iter().filter_map(|turn| millis(turn)).collect();
    let active: i64 = stamps.windows(2).map(|pair| (pair[1] - pair[0]).clamp(0, ACTIVE_GAP_CAP_MINUTES * 60_000)).sum();
    let units: f64 = turns.iter().map(|turn| turn_units(turn)).sum();
    (active > 0).then(|| units / (active as f64 / 3_600_000.0))
}

/// The Job's burn rate over the 60 minutes up to the newest turn, which `turns` ends with, by
/// [`burn_rate`]. It is stored whatever the number of turns, the trip having fired on the newest;
/// 0 where one turn is all there is.
pub fn hour_burn(turns: &[KeptTurnUsage]) -> f64 {
    burn_rate(&last_hour(turns)).unwrap_or(0.0)
}

/// The rules `turns` cross. `turns` are one Job's, oldest first, and reach back over the last hour
/// and its last [`Limits::main_turns_needed`] main-thread turns.
pub fn tripped(limits: &Limits, turns: &[KeptTurnUsage]) -> Vec<Trip> {
    let main: Vec<&KeptTurnUsage> = turns.iter().filter(|turn| !turn.subagent).collect();
    let mut found = Vec::new();

    if let (Some(limit), Some(last)) = (limits.context_tokens, main.last()) {
        let held = context(last);
        if held > limit {
            found.push(Trip {
                rule: Rule::Context,
                figure: held as f64,
                text: format!(
                    "Context is {}k tokens, past the {} limit. Write only a brief of where the work is, what is open and what you hold, and send it with handoff_brief. The owner answers on a card. Ask him nothing and start nothing yourself. Once it is sent, end your turn.",
                    (held as f64 / 1_000.0).round(),
                    ipc::handoff_figure(limit),
                ),
            });
        }
    }
    found
}

/// Which Jobs have been told which rules. **A Job is told a rule once**, however its figure moves
/// after. Never lost by a restart: the trip rows say a Job was told ([`Following::remembered`]).
/// A telling due while the person has something to decide waits in `held_for_quiet` until a quiet
/// moment.
#[derive(Default)]
pub struct Following {
    told: Mutex<BTreeSet<(String, Rule)>>,
    held_for_quiet: Mutex<BTreeSet<String>>,
}

impl Following {
    fn told_set(&self) -> std::sync::MutexGuard<'_, BTreeSet<(String, Rule)>> {
        self.told.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Whether this Job's standing for `rule` is known, in this process or from a row.
    pub fn knows(&self, session: &str, rule: Rule) -> bool {
        self.told_set().contains(&(session.to_string(), rule))
    }

    /// The Job was told of `rule`, which a trip row says.
    pub fn remembered(&self, session: &str, rule: Rule) {
        self.told_set().insert((session.to_string(), rule));
    }

    /// The Job was sent the words for `rule`.
    pub fn told(&self, session: &str, rule: Rule) {
        self.told_set().insert((session.to_string(), rule));
        self.held_for_quiet.lock().unwrap_or_else(std::sync::PoisonError::into_inner).remove(session);
    }

    /// The Job has a telling due that the person's own decision holds back; it is read again.
    pub fn held(&self, session: &str) {
        self.held_for_quiet.lock().unwrap_or_else(std::sync::PoisonError::into_inner).insert(session.to_string());
    }

    /// The Jobs whose telling waits for a quiet moment.
    pub fn waiting_for_quiet(&self) -> Vec<String> {
        self.held_for_quiet.lock().unwrap_or_else(std::sync::PoisonError::into_inner).iter().cloned().collect()
    }

    /// Of the rules `over` the Job is over, those it has not been told.
    pub fn due(&self, session: &str, over: &[Trip]) -> Vec<Trip> {
        over.iter().filter(|trip| !self.knows(session, trip.rule)).cloned().collect()
    }
}

/// A rule's on-off and limit as settings.json holds them, in [`Rule::ALL`] order: thousand tokens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleDial {
    pub on: bool,
    pub limit: i64,
}

/// The dials settings.json holds now.
pub fn dials_of(settings: &Resolved) -> [RuleDial; 1] {
    let dial = |on: keys::Flag, limit: keys::Integer| RuleDial { on: settings.get(on), limit: settings.get(limit) };
    [
        dial(keys::CONTEXT_WATCH, keys::CONTEXT_WATCH_KTOKENS),
    ]
}

/// What a Job's hour after a trip came to. The hour is from the trip's instant, exclusive, to 60
/// minutes on, inclusive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Measured {
    /// How many turns the Job made in the hour.
    pub turns: u32,
    /// Cost units per hour of the Job's **active time**, or `None` where the Job made fewer than
    /// [`MEASURE_AT_LEAST`] turns (a trip with no reading) or no time passed between them.
    pub burn_per_hour: Option<f64>,
}

/// Fewer turns than this in the hour after a trip is no reading.
pub const MEASURE_AT_LEAST: usize = 5;
/// A gap between two turns counts as at most this long, so a Job that sat idle does not look cheap.
pub const ACTIVE_GAP_CAP_MINUTES: i64 = 5;
const MEASURE_MILLIS: i64 = 60 * 60_000;

/// Whether `newest`, the Job's latest turn, is 60 minutes or more past `trip`, so the hour after it
/// has been covered.
pub fn covered(trip: &Timestamp, newest: &Timestamp) -> bool {
    match (trip.epoch_millis(), newest.epoch_millis()) {
        (Some(trip), Some(newest)) => newest - trip >= MEASURE_MILLIS,
        _ => false,
    }
}

/// The hour after a trip, from the Job's turns in it, oldest first, by [`burn_rate`]. Fewer than
/// [`MEASURE_AT_LEAST`] turns is no reading.
pub fn measured_after(turns: &[KeptTurnUsage]) -> Measured {
    let count = u32::try_from(turns.len()).unwrap_or(u32::MAX);
    if turns.len() < MEASURE_AT_LEAST {
        return Measured { turns: count, burn_per_hour: None };
    }
    let all: Vec<&KeptTurnUsage> = turns.iter().collect();
    Measured { turns: count, burn_per_hour: burn_rate(&all) }
}

fn unit_of(key: &str) -> String {
    match keys::entry(key).map(|entry| entry.kind) {
        Some(keys::Kind::Integer { unit, .. }) => unit.to_string(),
        _ => String::new(),
    }
}

/// The wire form of the rules over `[since, until)`: each rule with its dial and its trips, newest
/// first. `trips` may be in any order and hold any rule.
pub fn report(since: &str, until: &str, dials: &[RuleDial; 1], trips: &[KeptTrip]) -> ipc::UsageRules {
    let keys_of = [keys::CONTEXT_WATCH_KTOKENS.name()];
    let rules = Rule::ALL
        .into_iter()
        .zip(dials)
        .zip(keys_of)
        .map(|((rule, dial), key)| {
            let mut held: Vec<&KeptTrip> = trips.iter().filter(|trip| trip.rule == rule.name()).collect();
            held.sort_by(|a, b| b.at.cmp(&a.at).then_with(|| a.session_id.cmp(&b.session_id)));
            ipc::UsageRule {
                rule: rule.name().to_string(),
                on: dial.on,
                limit: ipc::UsageRuleLimit { value: dial.limit, unit: unit_of(key) },
                trips: held
                    .into_iter()
                    .map(|trip| ipc::UsageRuleTrip {
                        session_id: trip.session_id.clone(),
                        at: trip.at.clone(),
                        figure: trip.figure,
                        burn_per_hour_before: trip.burn_per_hour_before,
                        burn_per_hour_after: trip.burn_per_hour_after,
                    })
                    .collect(),
            }
        })
        .collect();
    ipc::UsageRules { since: since.to_string(), until: until.to_string(), rules }
}
