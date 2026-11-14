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

use super::arriving::{Arrival, Arrived, LiveJob, Source};
use super::candidates::{near, shared_paths, Near};
use super::ruling::{Outcome, Way};
use super::thening::paths_in;
use super::Logged;
use crate::daemon::Fleet;
use crate::session_issues::ISSUE;

/// How long a Mac may be asleep, in seconds, before its rows are left out.
pub(crate) const ASLEEP_PAST: i64 = 2 * 60 * 60;

/// A live Job another Mac shares, as the Operator weighs it.
pub(crate) struct ElsewhereJob {
    mac: String,
    mac_name: String,
    job: LiveJob,
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

/// A row from another Mac as a live Job is weighed: its plan's paths, the issues it holds or its plan works, and
/// its words, which are its plan's summary and group titles.
fn weighed(record: &SessionRecord) -> LiveJob {
    let mut paths = BTreeSet::new();
    let mut issues = BTreeSet::new();
    let mut words = Vec::new();
    if let Some(plan) = &record.plan {
        words.push(plan.summary.clone());
        for group in &plan.groups {
            paths.extend(group.paths.iter().cloned());
            issues.extend(group.issue.iter().cloned());
            words.push(group.title.clone());
        }
    }
    issues.extend(record.attachments.iter().filter(|one| one.kind == ISSUE).map(|one| one.target.clone()));
    LiveJob {
        session_id: record.id.as_str().to_string(),
        title: record.title.clone().unwrap_or_else(|| record.id.as_str().to_string()),
        manifest_id: record.manifest_id.as_ref().map(|id| id.as_str().to_string()).unwrap_or_default(),
        words: words.join(". "),
        paths: paths.into_iter().collect(),
        issues: issues.into_iter().collect(),
        running: Vec::new(),
        started_by: Vec::new(),
        // Another Mac sends only what it shares.
        shared: true,
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
        let arrival = Arrival {
            id: String::new(),
            arrived: Arrived::Start,
            source: Source::Fleet,
            session_id: String::new(),
            manifest_id: manifest.to_string(),
            subject: subject.to_string(),
            words: words.to_string(),
            paths: paths_in(words),
            issues: Vec::new(),
            ways: Vec::new(),
            open: Vec::new(),
        };
        let near = jobs
            .into_iter()
            .filter_map(|one| {
                let found = near(&arrival, std::slice::from_ref(&one.job)).into_iter().next()?;
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
        let mut row = Logged::about(job, manifest, subject, Outcome::Ruled(Way::NewJob), Actor::Operator, self.now());
        row.arrival = format!("elsewhere:{job}");
        row.shared = found.near.iter().flat_map(|(_, near)| near.paths.iter().cloned()).collect();
        row.because = found.because();
        self.operator_logged(row).await;
        self.operator_board_changed();
    }
}
