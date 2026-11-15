//! Work on his other Macs, as the Operator reads it: the Job rows each Mac shares with this one, kept by
//! `other_macs`, and nothing else. `armada-hu76.2`.
//!
//! **It tells and holds nothing.** No Job here can wait on a Job another Mac holds, so work found near a start
//! is named to the new Job and in the Log, and the Job starts as it would have. A plan's paths meeting another
//! Mac's plan ride on the plan as `other_macs`, for Bridge to draw.
//!
//! **A Mac asleep past [`ASLEEP_PAST`] is left out**, and the Log says which and for how long: its rows are the
//! last it sent, and may name work long finished.

use std::collections::BTreeSet;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use core_model::Actor;
use ipc::{Notice, PlanOverlap, SessionId, SessionRecord, SessionState};
use store::{FromMac, Store};

use super::{Logged, Outcome};
use crate::daemon::Fleet;

/// How long a Mac may be asleep, in seconds, before its rows are left out.
pub(crate) const ASLEEP_PAST: i64 = 2 * 60 * 60;

/// A live Job another Mac shares, as a start is weighed against it.
pub(crate) struct ElsewhereJob {
    mac: String,
    mac_name: String,
    job: SharedJob,
}

/// A Job on another Mac: its plan's paths, the issues it holds and its words.
struct SharedJob {
    session_id: String,
    title: String,
    manifest_id: String,
    words: String,
    paths: Vec<String>,
}

/// What a Job on another Mac shares with a start.
struct Near {
    paths: Vec<String>,
    words: Vec<String>,
}

impl Near {
    fn said(&self) -> String {
        let mut parts = Vec::new();
        if !self.paths.is_empty() {
            parts.push(format!("paths {}", self.paths.join(", ")));
        }
        if !self.words.is_empty() {
            parts.push(format!("words {}", self.words.join(", ")));
        }
        parts.join("; ")
    }
}

/// Words this short, or this common in a request, say nothing about what the work is.
const SHORTEST_WORD: usize = 5;
const COMMON: &[&str] = &[
    "about", "after", "again", "agent", "before", "being", "build", "change", "could", "every", "fixed", "first", "issue", "should",
    "their", "there", "these", "thing", "think", "those", "where", "which", "while", "would", "write", "armada",
];
/// Shared words that make two Jobs near, where nothing else does.
const WORDS_NEAR: usize = 2;
/// The most shared words shown for one pair.
const WORDS_SHOWN: usize = 5;

/// What `job` shares with a start's `paths` and words, where it shares anything.
fn near(paths: &[String], subject: &str, words: &str, job: &SharedJob) -> Option<Near> {
    let ours = uncommon(&format!("{subject} {words}"));
    let paths = shared_paths(paths, &job.paths);
    let theirs = uncommon(&format!("{} {}", job.title, job.words));
    let mut shared: Vec<String> = ours.intersection(&theirs).cloned().collect();
    if shared.len() < WORDS_NEAR {
        shared.clear();
    }
    shared.truncate(WORDS_SHOWN);
    (!paths.is_empty() || !shared.is_empty()).then_some(Near { paths, words: shared })
}

/// The paths `text` names: words with a `/` in them that end in a file's extension or a folder's slash.
fn paths_in(text: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for word in text.split(|one: char| one.is_whitespace() || matches!(one, ',' | ';' | ':' | '(' | ')' | '"' | '\'' | '`' | '<' | '>' | '[' | ']')) {
        let path = word.trim_end_matches('.').trim_start_matches("./");
        let last = path.rsplit('/').next().unwrap_or_default();
        if path.contains('/') && !path.contains("//") && (path.ends_with('/') || last.contains('.')) && !found.iter().any(|one| one == path) {
            found.push(path.to_string());
        }
    }
    found
}

/// Each of `ours` that meets one of `theirs`, the more exact of the two named.
pub(crate) fn shared_paths(ours: &[String], theirs: &[String]) -> Vec<String> {
    let mut shared = BTreeSet::new();
    for one in ours {
        for other in theirs {
            if let Some(met) = met(one, other) {
                shared.insert(met.to_string());
            }
        }
    }
    shared.into_iter().collect()
}

/// Where two paths meet: the same file, a file under a folder the other names, or under a pattern's
/// fixed start. The longer of the two, which names the place more exactly.
fn met<'a>(one: &'a str, other: &'a str) -> Option<&'a str> {
    let (a, b) = (fixed(one), fixed(other));
    if a.0.is_empty() || b.0.is_empty() {
        return None;
    }
    let under = |path: (&str, bool), place: (&str, bool)| {
        path.0 == place.0 || (place.1 && path.0.starts_with(place.0)) || (path.0.starts_with(place.0) && path.0[place.0.len()..].starts_with('/'))
    };
    if under(a, b) {
        Some(one)
    } else if under(b, a) {
        Some(other)
    } else {
        None
    }
}

/// A path up to its first pattern character, without a leading `./` or a trailing `/`, and whether it was a pattern.
fn fixed(path: &str) -> (&str, bool) {
    let path = path.trim().trim_start_matches("./");
    match path.find(['*', '?', '[', '{']) {
        Some(at) => (&path[..at], true),
        None => (path.trim_end_matches('/'), false),
    }
}

/// The uncommon words of `text`, lowercased, with a plural's `s` dropped.
fn uncommon(text: &str) -> BTreeSet<String> {
    text.split(|one: char| !one.is_alphanumeric())
        .filter(|word| word.chars().count() >= SHORTEST_WORD)
        .map(|word| {
            let word = word.to_lowercase();
            match word.strip_suffix('s') {
                Some(single) if single.chars().count() >= SHORTEST_WORD && !single.ends_with('s') => single.to_string(),
                _ => word,
            }
        })
        .filter(|word| !COMMON.contains(&word.as_str()) && !word.chars().all(|one| one.is_ascii_digit()))
        .collect()
}

/// A paired Mac whose rows were left out, asleep for `for_secs`.
pub(crate) struct Asleep {
    name: String,
    for_secs: i64,
}

/// What his other Macs share that is still counted, and the Macs left out as asleep.
#[derive(Default)]
pub(crate) struct Elsewhere {
    jobs: Vec<ElsewhereJob>,
    asleep: Vec<Asleep>,
}

/// What a start found on his other Macs: the Jobs near it, each with what they share, and the Macs left out.
#[derive(Default)]
pub(crate) struct FoundElsewhere {
    near: Vec<(ElsewhereJob, Near)>,
    asleep: Vec<Asleep>,
}

impl FoundElsewhere {
    pub(crate) fn is_empty(&self) -> bool {
        self.near.is_empty() && self.asleep.is_empty()
    }

    /// What the new Job is told, where another Mac has work near it.
    pub(crate) fn told(&self) -> Option<String> {
        if self.near.is_empty() {
            return None;
        }
        let mut text = String::from(
            "Work near this is under way on another Mac. Nothing here waits on it. Read it before you plan, and say in your plan how yours differs.\n",
        );
        for (one, near) in &self.near {
            text.push_str(&format!("\n- On {}: Job \"{}\" ({}), sharing {}", one.mac_name, one.job.title, one.job.session_id, near.said()));
        }
        Some(text)
    }

    /// The Log's line for the start: the Macs and Jobs near it, and the Macs left out.
    fn because(&self) -> String {
        let mut said = vec!["Started it.".to_string()];
        for (one, near) in &self.near {
            said.push(format!("Job \"{}\" on {} is near: {}.", one.job.title, one.mac_name, near.said()));
        }
        for one in &self.asleep {
            said.push(format!("Left out {}'s Jobs: asleep for {}.", one.name, hours(one.for_secs)));
        }
        said.join(" ")
    }
}

/// `secs` as whole hours, or minutes under one.
fn hours(secs: i64) -> String {
    match secs / 3600 {
        0 => format!("{} minutes", secs / 60),
        1 => "1 hour".to_string(),
        many => format!("{many} hours"),
    }
}

/// The live Jobs his other Macs share, read from what this Mac keeps of them at `now`, in seconds. A row this
/// build cannot read, a Mac no longer paired, and an ended Job are left out.
pub(crate) fn elsewhere(store: &Store, now: i64) -> Elsewhere {
    let held = store.held_from_macs(FromMac::Sessions).unwrap_or_default();
    if held.is_empty() {
        return Elsewhere::default();
    }
    let macs = store.paired_macs().unwrap_or_default();
    let mut found = Elsewhere::default();
    let mut left_out = BTreeSet::new();
    for one in held {
        let Some(mac) = macs.iter().find(|mac| mac.id == one.mac) else { continue };
        if let Some(since) = mac.asleep_since.filter(|since| now - since > ASLEEP_PAST) {
            if left_out.insert(mac.id.clone()) {
                found.asleep.push(Asleep { name: mac.name.clone(), for_secs: now - since });
            }
            continue;
        }
        let Ok(record) = ipc::decode::<SessionRecord>("a Job row from another Mac", one.body.as_bytes()) else { continue };
        if record.state == SessionState::Ended {
            continue;
        }
        found.jobs.push(ElsewhereJob { mac: mac.id.clone(), mac_name: mac.name.clone(), job: weighed(&record) });
    }
    found
}

/// A row from another Mac as a Job is weighed: its plan's paths and its words, which are its plan's summary and group titles.
fn weighed(record: &SessionRecord) -> SharedJob {
    let mut paths = BTreeSet::new();
    let mut words = Vec::new();
    if let Some(plan) = &record.plan {
        words.push(plan.summary.clone());
        for group in &plan.groups {
            paths.extend(group.paths.iter().cloned());
            words.push(group.title.clone());
        }
    }
    SharedJob {
        session_id: record.id.as_str().to_string(),
        title: record.title.clone().unwrap_or_else(|| record.id.as_str().to_string()),
        manifest_id: record.manifest_id.as_ref().map(|id| id.as_str().to_string()).unwrap_or_default(),
        words: words.join(". "),
        paths: paths.into_iter().collect(),
    }
}

/// The Jobs on his other Macs, in `manifest`, whose plans meet `paths`, a plan's own, at `now` in seconds.
pub(crate) fn plan_overlap(store: &Store, manifest: &str, paths: &[String], now: i64) -> Vec<PlanOverlap> {
    if paths.is_empty() {
        return Vec::new();
    }
    elsewhere(store, now)
        .jobs
        .into_iter()
        .filter(|one| one.job.manifest_id == manifest)
        .filter_map(|one| {
            let met = shared_paths(paths, &one.job.paths);
            (!met.is_empty()).then(|| PlanOverlap { mac: one.mac, session_id: SessionId::carried(&one.job.session_id), title: one.job.title, paths: met })
        })
        .collect()
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// The work his other Macs share near a start titled `subject` in `manifest` on `words`. Nothing while the
    /// Operator is off.
    pub(crate) async fn found_elsewhere(&self, manifest: &str, subject: &str, words: &str) -> FoundElsewhere {
        if !self.operator_on() {
            return FoundElsewhere::default();
        }
        let Elsewhere { jobs, asleep } = elsewhere(&*self.store().lock().await, self.unix_seconds());
        let paths = paths_in(words);
        let near = jobs
            .into_iter()
            .filter(|one| one.job.manifest_id == manifest)
            .filter_map(|one| {
                let found = near(&paths, subject, words, &one.job)?;
                Some((one, found))
            })
            .collect();
        FoundElsewhere { near, asleep }
    }

    /// What a start found elsewhere, told to its Job `job` as a notice of the Operator's and kept in the Log.
    pub(crate) async fn elsewhere_told(self: &Arc<Self>, job: &str, manifest: &str, subject: &str, found: &FoundElsewhere) {
        if let Some(text) = found.told() {
            if !matches!(self.told_by_fleet(job, &text, Notice::Operator).await, Ok(true)) {
                self.operator_warned("a Job was not told of work near it on another Mac", job);
            }
        }
        self.elsewhere_logged(job, manifest, subject, found).await;
    }

    /// The Log's row for a start that found work elsewhere or left a Mac out. Nothing where it did neither.
    pub(crate) async fn elsewhere_logged(&self, job: &str, manifest: &str, subject: &str, found: &FoundElsewhere) {
        if found.is_empty() {
            return;
        }
        let mut row = Logged::about(job, manifest, subject, Outcome::Started, Actor::Operator, self.now());
        row.arrival = format!("elsewhere:{job}");
        row.shared = found.near.iter().flat_map(|(_, near)| near.paths.iter().cloned()).collect();
        row.because = found.because();
        self.operator_logged(row).await;
    }
}
