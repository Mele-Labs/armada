//! Every call Fleet makes to GitHub's API, counted by the line that asked, the API that answered and
//! how it answered, so what spends the limit every caller on this login shares is read, not guessed.
//!
//! **This crate reads no clock.** Fleet turns the minute ([`tick`]) on its own clock's word, and a
//! call is counted into whichever minute is open. Two hours are kept, in memory, lost with the process.
//!
//! **A call is a request that reached GitHub.** One `gh` would not give a token for never left, and a
//! log's download goes to GitHub's storage, which no API limit counts; neither is counted.

use std::cell::Cell;
use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::panic::Location;
use std::sync::{Mutex, PoisonError};

use crate::github_api::RateLimit;

/// Finished minutes kept for the report.
const MINUTES_KEPT: usize = 120;
/// The minutes "the last hour" reads.
const HOUR: usize = 60;
/// Callers a minute's log line names.
const NAMED_IN_A_LINE: usize = 5;
/// Callers the report names.
const NAMED_IN_THE_REPORT: usize = 12;

/// Which API a call went to: each has its own limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Api {
    Rest,
    GraphQl,
}

impl Api {
    fn name(self) -> &'static str {
        match self {
            Api::Rest => "rest",
            Api::GraphQl => "graphql",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Outcome {
    Answered,
    RateLimited,
    Failed,
}

/// A line of source, as `Location` gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Place {
    file: &'static str,
    line: u32,
}

impl Place {
    fn of(at: &'static Location<'static>) -> Place {
        Place { file: at.file(), line: at.line() }
    }
}

/// `crates/fleet/src/main_hub.rs` as `fleet/main_hub.rs`.
impl fmt::Display for Place {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file = self.file.strip_prefix("crates/").unwrap_or(self.file).replacen("/src/", "/", 1);
        write!(out, "{file}:{}", self.line)
    }
}

/// One call's counting. `asked_from` is Fleet's line where it named one ([`asked_from`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    asked_from: Option<Place>,
    call: Place,
    api: Api,
    outcome: Outcome,
}

impl Key {
    fn caller(&self) -> String {
        match self.asked_from {
            Some(from) => format!("{from} > {}", self.call),
            None => self.call.to_string(),
        }
    }
}

/// A refusal for the rate limit, with GitHub's own words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Limited {
    /// Fleet's instant at the tick after it; empty until then.
    pub at: String,
    pub caller: String,
    pub said: String,
    pub limit: Option<RateLimit>,
}

impl Limited {
    /// When, where and the limit's numbers, without GitHub's words: the debug report prints names
    /// and states only.
    fn bare(&self) -> String {
        format!("{} {}{}", self.at, self.caller, limit_of(self.limit))
    }
}

/// The log's line: GitHub's words as well.
impl fmt::Display for Limited {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "{} {}: {}{}", self.at, self.caller, self.said, limit_of(self.limit))
    }
}

/// ` (retry after 60 s)` or ` (remaining 0, resets at 1795369200)`, where GitHub said.
fn limit_of(limit: Option<RateLimit>) -> String {
    match limit {
        Some(RateLimit { retry_after: Some(seconds), .. }) => format!(" (retry after {seconds} s)"),
        Some(RateLimit { remaining, reset_at: Some(at), .. }) => {
            format!(" (remaining {}, resets at {at})", remaining.map_or("?".to_string(), |it| it.to_string()))
        }
        _ => String::new(),
    }
}

/// The latest call GitHub did not answer, for what Fleet says of a quiet forge.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Failed {
    pub rate_limited: bool,
    /// Seconds GitHub said to wait: `retry-after`.
    pub retry_after: Option<u64>,
    /// When a spent window resets, in seconds since the epoch: `x-ratelimit-reset` with none remaining.
    pub spent_until: Option<u64>,
    /// Fleet's instant at the tick after it; empty until then.
    pub at: String,
}

thread_local! {
    static ASKED_FROM: Cell<Option<Place>> = const { Cell::new(None) };
}

/// Puts the thread's caller back, a panic in the work included: blocking threads are reused.
struct Restored(Option<Place>);

impl Drop for Restored {
    fn drop(&mut self) {
        ASKED_FROM.with(|it| it.set(self.0));
    }
}

/// `work`, with every call it makes to GitHub on this thread counted as asked from `at`.
pub fn asked_from<T>(at: &'static Location<'static>, work: impl FnOnce() -> T) -> T {
    let _restored = Restored(ASKED_FROM.with(|it| it.replace(Some(Place::of(at)))));
    work()
}

/// `work` for another thread, its calls counted as asked from this thread's caller: a spawned
/// thread starts with none.
pub(crate) fn carried<T>(work: impl FnOnce() -> T) -> impl FnOnce() -> T {
    let from = ASKED_FROM.with(Cell::get);
    move || {
        let _restored = Restored(ASKED_FROM.with(|it| it.replace(from)));
        work()
    }
}

/// One call that reached GitHub, and the refusal's words and limit where it was rate-limited.
pub(crate) fn counted(at: &'static Location<'static>, api: Api, outcome: Outcome, limited: Option<(String, Option<RateLimit>)>) {
    let key = Key { asked_from: ASKED_FROM.with(Cell::get), call: Place::of(at), api, outcome };
    held().count(key, limited);
}

/// What GitHub said is left of a limit, `x-ratelimit-resource` and `x-ratelimit-remaining`.
pub(crate) fn left(resource: &str, remaining: u64) {
    held().left.insert(resource.to_string(), remaining);
}

/// The latest call GitHub did not answer, if any has failed since Fleet started.
pub fn latest_failure() -> Option<Failed> {
    held().latest_failure.clone()
}

/// Fleet's tick: `minute` since the epoch and `now` as Fleet's clock wrote it. The lines to log: the
/// minute just finished, where any call was made in it, and the first rate-limit refusal, once.
pub fn tick(minute: i64, now: &str) -> Vec<String> {
    held().tick(minute, now)
}

/// The report's rows: the last hour, the callers that spent it, and the rate-limit refusals.
pub fn report_rows() -> Vec<(String, String)> {
    held().rows()
}

static TALLY: Mutex<Tally> = Mutex::new(Tally::new());

fn held() -> std::sync::MutexGuard<'static, Tally> {
    TALLY.lock().unwrap_or_else(PoisonError::into_inner)
}

/// One minute's calls, under `HH:MMZ`.
#[derive(Debug, Default)]
struct Minute {
    label: String,
    counts: BTreeMap<Key, u32>,
}

#[derive(Debug)]
pub(crate) struct Tally {
    /// The open minute, since the epoch, and its label.
    open: Option<(i64, String)>,
    counts: BTreeMap<Key, u32>,
    /// Finished minutes, oldest first.
    kept: VecDeque<Minute>,
    totals: BTreeMap<Key, u64>,
    since: Option<String>,
    first_limited: Option<Limited>,
    latest_limited: Option<Limited>,
    first_told: bool,
    left: BTreeMap<String, u64>,
    latest_failure: Option<Failed>,
}

impl Tally {
    pub(crate) const fn new() -> Tally {
        Tally {
            open: None,
            counts: BTreeMap::new(),
            kept: VecDeque::new(),
            totals: BTreeMap::new(),
            since: None,
            first_limited: None,
            latest_limited: None,
            first_told: false,
            left: BTreeMap::new(),
            latest_failure: None,
        }
    }

    fn count(&mut self, key: Key, limited: Option<(String, Option<RateLimit>)>) {
        *self.counts.entry(key).or_default() += 1;
        *self.totals.entry(key).or_default() += 1;
        if key.outcome == Outcome::Answered {
            return;
        }
        let (said, limit) = limited.unwrap_or_default();
        self.latest_failure = Some(Failed {
            rate_limited: key.outcome == Outcome::RateLimited,
            retry_after: limit.and_then(|it| it.retry_after),
            spent_until: limit.filter(|it| it.remaining == Some(0)).and_then(|it| it.reset_at),
            at: String::new(),
        });
        if key.outcome != Outcome::RateLimited {
            return;
        }
        let limited = Limited { at: String::new(), caller: key.caller(), said, limit };
        self.first_limited.get_or_insert_with(|| limited.clone());
        self.latest_limited = Some(limited);
    }

    pub(crate) fn tick(&mut self, minute: i64, now: &str) -> Vec<String> {
        for limited in [&mut self.first_limited, &mut self.latest_limited].into_iter().flatten() {
            if limited.at.is_empty() {
                limited.at = now.to_string();
            }
        }
        if let Some(failed) = self.latest_failure.as_mut().filter(|it| it.at.is_empty()) {
            failed.at = now.to_string();
        }
        let mut lines = Vec::new();
        match &self.open {
            Some((open, _)) if *open == minute => {}
            Some(_) => {
                let (_, label) = self.open.take().unwrap_or_default();
                let finished = Minute { label, counts: std::mem::take(&mut self.counts) };
                if !finished.counts.is_empty() {
                    lines.push(line_of(&finished, &self.left));
                }
                self.kept.push_back(finished);
                if self.kept.len() > MINUTES_KEPT {
                    self.kept.pop_front();
                }
                self.open = Some((minute, label_of(now)));
            }
            None => {
                self.since = Some(now.to_string());
                self.open = Some((minute, label_of(now)));
            }
        }
        if let (Some(first), false) = (&self.first_limited, self.first_told) {
            lines.push(format!("github: first rate-limit refusal {first}"));
            self.first_told = true;
        }
        lines
    }

    pub(crate) fn rows(&self) -> Vec<(String, String)> {
        let Some(since) = &self.since else { return Vec::new() };
        let mut rows = vec![("counted since".to_string(), since.clone())];
        let hour: Vec<&BTreeMap<Key, u32>> =
            self.kept.iter().rev().take(HOUR).map(|minute| &minute.counts).chain([&self.counts]).collect();
        let per_minute: Vec<(String, u32)> = self
            .kept
            .iter()
            .rev()
            .take(HOUR)
            .map(|minute| (minute.label.clone(), minute.counts.values().sum()))
            .collect();
        let in_hour: u64 = hour.iter().flat_map(|counts| counts.values()).map(|it| u64::from(*it)).sum();
        let peak = per_minute.iter().max_by_key(|(_, calls)| *calls);
        let mut last_hour = format!("{in_hour} calls in {} finished minutes", per_minute.len());
        if let Some((label, calls)) = peak {
            last_hour.push_str(&format!(", peak {calls}/min at {label}"));
        }
        rows.push(("last hour".to_string(), last_hour));
        rows.push(("since start".to_string(), summary_of(self.totals.iter().map(|(key, calls)| (*key, *calls)))));
        let mut callers: BTreeMap<(String, Api), (u64, u64)> = BTreeMap::new();
        for (key, calls) in hour.iter().flat_map(|counts| counts.iter()) {
            let it = callers.entry((key.caller(), key.api)).or_default();
            it.0 += u64::from(*calls);
            if key.outcome == Outcome::RateLimited {
                it.1 += u64::from(*calls);
            }
        }
        let mut callers: Vec<_> = callers.into_iter().collect();
        callers.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then_with(|| a.0.cmp(&b.0)));
        let named: Vec<String> = callers
            .iter()
            .take(NAMED_IN_THE_REPORT)
            .map(|((caller, api), (calls, limited))| match limited {
                0 => format!("{calls:>5}  {} {caller}", api.name()),
                _ => format!("{calls:>5}  {} {caller} ({limited} rate-limited)", api.name()),
            })
            .collect();
        if !named.is_empty() {
            rows.push(("top, last hour".to_string(), named.join("\n")));
        }
        if let Some(first) = &self.first_limited {
            rows.push(("first rate limit".to_string(), first.bare()));
        }
        if let Some(latest) = &self.latest_limited {
            rows.push(("latest rate limit".to_string(), latest.bare()));
        }
        if !self.left.is_empty() {
            rows.push(("limit left".to_string(), left_of(&self.left)));
        }
        rows
    }
}

/// `17:53Z` of `2026-11-16T17:53:02.000Z`.
fn label_of(now: &str) -> String {
    match now.get(11..16) {
        Some(clock) => format!("{clock}Z"),
        None => now.to_string(),
    }
}

/// `42 (rest 30, graphql 12; 3 rate-limited, 1 failed)`.
fn summary_of(counts: impl Iterator<Item = (Key, u64)>) -> String {
    let (mut all, mut rest, mut graphql, mut limited, mut failed) = (0, 0, 0, 0, 0);
    for (key, calls) in counts {
        all += calls;
        match key.api {
            Api::Rest => rest += calls,
            Api::GraphQl => graphql += calls,
        }
        match key.outcome {
            Outcome::Answered => {}
            Outcome::RateLimited => limited += calls,
            Outcome::Failed => failed += calls,
        }
    }
    format!("{all} (rest {rest}, graphql {graphql}; {limited} rate-limited, {failed} failed)")
}

fn left_of(left: &BTreeMap<String, u64>) -> String {
    left.iter().map(|(resource, remaining)| format!("{resource} {remaining}")).collect::<Vec<_>>().join(", ")
}

/// `github calls 17:53Z: 42 (…); <caller> graphql 12, …; left core 3451, graphql 4800`.
fn line_of(minute: &Minute, left: &BTreeMap<String, u64>) -> String {
    let mut callers: BTreeMap<(String, Api), u32> = BTreeMap::new();
    for (key, calls) in &minute.counts {
        *callers.entry((key.caller(), key.api)).or_default() += calls;
    }
    let mut callers: Vec<_> = callers.into_iter().collect();
    callers.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let named: Vec<String> = callers
        .iter()
        .take(NAMED_IN_A_LINE)
        .map(|((caller, api), calls)| format!("{caller} {} {calls}", api.name()))
        .collect();
    let summary = summary_of(minute.counts.iter().map(|(key, calls)| (*key, u64::from(*calls))));
    let mut line = format!("github calls {}: {summary}; {}", minute.label, named.join(", "));
    if !left.is_empty() {
        line.push_str(&format!("; left {}", left_of(left)));
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn here() -> &'static Location<'static> {
        Location::caller()
    }

    fn key(at: &'static Location<'static>, api: Api, outcome: Outcome) -> Key {
        Key { asked_from: None, call: Place::of(at), api, outcome }
    }

    #[test]
    fn a_finished_minute_is_one_line_naming_its_busiest_caller_first() {
        let busy = here();
        let quiet = here();
        let mut tally = Tally::new();
        assert!(tally.tick(100, "2026-11-16T17:52:59.000Z").is_empty());
        for _ in 0..3 {
            tally.count(key(busy, Api::GraphQl, Outcome::Answered), None);
        }
        tally.count(key(quiet, Api::Rest, Outcome::Failed), None);
        tally.left.insert("graphql".to_string(), 4800);
        assert!(tally.tick(100, "2026-11-16T17:52:59.500Z").is_empty(), "the minute is still open");
        let lines = tally.tick(101, "2026-11-16T17:53:00.000Z");
        assert_eq!(lines.len(), 1);
        let line = &lines[0];
        assert!(line.starts_with("github calls 17:52Z: 4 (rest 1, graphql 3; 0 rate-limited, 1 failed); "), "{line}");
        let (first, second) = (line.find(&Place::of(busy).to_string()), line.find(&Place::of(quiet).to_string()));
        assert!(first.is_some() && first < second, "{line}");
        assert!(line.ends_with("; left graphql 4800"), "{line}");
    }

    #[test]
    fn a_thread_spawned_for_the_work_counts_its_calls_against_the_caller_that_spawned_it() {
        let at = here();
        let (bare, carried) = asked_from(at, || {
            let bare = std::thread::spawn(|| ASKED_FROM.with(Cell::get)).join().unwrap();
            (bare, std::thread::spawn(carried(|| ASKED_FROM.with(Cell::get))).join().unwrap())
        });
        assert_eq!(bare, None);
        assert_eq!(carried, Some(Place::of(at)));
    }

    #[test]
    fn a_minute_with_no_calls_logs_nothing() {
        let mut tally = Tally::new();
        tally.tick(100, "2026-11-16T17:52:00.000Z");
        assert!(tally.tick(101, "2026-11-16T17:53:00.000Z").is_empty());
    }

    #[test]
    fn the_first_rate_limit_refusal_is_logged_once_with_its_words_and_the_caller_that_met_it() {
        let at = here();
        let mut tally = Tally::new();
        tally.tick(100, "2026-11-16T17:52:00.000Z");
        let said = "API rate limit already exceeded for user ID 1".to_string();
        tally.count(key(at, Api::GraphQl, Outcome::RateLimited), Some((said.clone(), None)));
        let lines = tally.tick(100, "2026-11-16T17:52:05.000Z");
        assert_eq!(lines, vec![format!("github: first rate-limit refusal 2026-11-16T17:52:05.000Z {}: {said}", Place::of(at))]);
        tally.count(key(at, Api::GraphQl, Outcome::RateLimited), Some(("again".to_string(), None)));
        assert!(tally.tick(100, "2026-11-16T17:52:06.000Z").is_empty(), "told once");
        let rows = tally.rows();
        let row = |label: &str| rows.iter().find(|(it, _)| it == label).map(|(_, value)| value.clone());
        assert_eq!(row("first rate limit"), Some(format!("2026-11-16T17:52:05.000Z {}", Place::of(at))));
        assert!(rows.iter().all(|(_, value)| !value.contains("exceeded") && !value.contains("again")), "GitHub's words stay in the log");
    }

    #[test]
    fn the_report_names_the_hours_peak_minute_and_its_callers() {
        let at = here();
        let mut tally = Tally::new();
        tally.tick(100, "2026-11-16T17:00:00.000Z");
        tally.count(key(at, Api::Rest, Outcome::Answered), None);
        tally.tick(101, "2026-11-16T17:01:00.000Z");
        for _ in 0..5 {
            tally.count(key(at, Api::Rest, Outcome::Answered), None);
        }
        tally.tick(102, "2026-11-16T17:02:00.000Z");
        let rows = tally.rows();
        let row = |label: &str| rows.iter().find(|(it, _)| it == label).map(|(_, value)| value.clone()).unwrap_or_default();
        assert_eq!(row("last hour"), "6 calls in 2 finished minutes, peak 5/min at 17:01Z");
        assert_eq!(row("top, last hour"), format!("    6  rest {}", Place::of(at)));
    }

    #[test]
    fn the_latest_failure_says_whether_it_was_the_rate_limit_and_when_github_said_to_come_back() {
        let at = here();
        let mut tally = Tally::new();
        let spent = RateLimit { remaining: Some(0), reset_at: Some(1_795_369_200), retry_after: None };
        tally.count(key(at, Api::GraphQl, Outcome::RateLimited), Some(("spent".to_string(), Some(spent))));
        tally.tick(100, "2026-11-16T17:52:05.000Z");
        let failed = tally.latest_failure.clone().expect("a failure is kept");
        assert!(failed.rate_limited);
        assert_eq!((failed.spent_until, failed.retry_after), (Some(1_795_369_200), None));
        assert_eq!(failed.at, "2026-11-16T17:52:05.000Z");
        tally.count(key(at, Api::Rest, Outcome::Failed), None);
        assert!(!tally.latest_failure.clone().expect("kept").rate_limited, "the latest failure wins");
        tally.count(key(at, Api::Rest, Outcome::Answered), None);
        assert!(tally.latest_failure.is_some(), "an answer does not clear it: Fleet's visit says when the forge answers");
    }

    #[test]
    fn a_caller_fleet_named_heads_the_line_that_made_the_call() {
        let from = here();
        let call = here();
        asked_from(from, || counted(call, Api::Rest, Outcome::Answered, None));
        let named = Key { asked_from: Some(Place::of(from)), call: Place::of(call), api: Api::Rest, outcome: Outcome::Answered };
        assert!(held().totals.contains_key(&named));
        assert_eq!(ASKED_FROM.with(Cell::get), None, "the thread's caller is put back");
    }
}
