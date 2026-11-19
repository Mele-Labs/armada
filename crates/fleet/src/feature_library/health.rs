//! A feature's health: a mark and the reasons behind it. `docs/concepts/feature-catalog.md`, *Health*.
//!
//! **Read from what Fleet already keeps, never collected and never guessed.** Each source is read once for a repository and
//! then asked of every feature of it. A source Fleet holds nothing for adds no reason and no mark.
//!
//! | Source | Where Fleet keeps it |
//! |---|---|
//! | Flaky tests that reach it | the `test` rows a fixing Job holds, and the test map's edges for that test |
//! | No test reaches it | the test map, where it holds any edge for the repository |
//! | Stale sections | the written sections and the merges after them (`staleness`) |
//! | Open bugs | the feature's `bug` links not closed |
//! | Failed Checks and analyzers | the Check runs of the live Jobs working in its files |
//!
//! **Left out, because Fleet keeps nothing per file for them**: a pull request's forge checks, and main's CI state.

use std::collections::{BTreeMap, BTreeSet};

use ipc::{FeatureHealth, FeatureJob, HealthMark, HealthReason, HealthReasonKind};
use store::{feature_link_kind, KeptFeature, KeptSection, Store};

use super::reading::Loaded;

/// Flaky tests reaching a feature from which it reads `poor`. Chosen, not measured: one is a nuisance and two a coincidence.
const FLAKY_POOR: usize = 3;
/// Open bugs from which a feature reads `poor`. Chosen, not measured: the same count as the flaky tests.
const BUGS_POOR: usize = 3;
/// How many of a repository's newest Check runs are read for the last run of each Check. Chosen, not measured: it
/// reaches back past every run of the Jobs now at work.
const CHECK_RUNS_READ: u32 = 400;
/// A Check whose name holds one of these is an analyzer: it reads the code and runs no test.
const ANALYZERS: [&str; 4] = ["clippy", "lint", "typecheck", "analy"];

/// One Check that failed on its last run in a Job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FailedRun {
    pub(crate) session: String,
    pub(crate) title: String,
    pub(crate) check: String,
}

/// What Fleet kept about one feature, each source empty where Fleet holds nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Evidence {
    /// The test map holds edges for the repository and none reaches the feature's files.
    pub(crate) no_test_reaches: bool,
    /// The flaky tests that reach its files.
    pub(crate) flaky: Vec<String>,
    /// A stale section's name and the pull request it is stale since.
    pub(crate) stale: Vec<(String, u64)>,
    /// An open bug's target and title.
    pub(crate) bugs: Vec<(String, String)>,
    pub(crate) failed: Vec<FailedRun>,
}

fn leaf(test: &str) -> &str {
    test.rsplit("::").next().unwrap_or(test)
}

fn is_analyzer(check: &str) -> bool {
    let check = check.to_ascii_lowercase();
    ANALYZERS.iter().any(|name| check.contains(name))
}

/// The mark and reasons `evidence` gives. **Poor** where a Check failed in its files, or flaky tests or open bugs reach the
/// count named above; **watch** where any other reason stands; **good**, with no reasons, where none does.
pub(crate) fn health_of(evidence: &Evidence) -> FeatureHealth {
    let mut reasons = Vec::new();
    for run in &evidence.failed {
        let kind = if is_analyzer(&run.check) { HealthReasonKind::AnalyzerFailed } else { HealthReasonKind::CheckFailed };
        reasons.push(HealthReason {
            kind,
            fact: format!("{} failed on the last run of \"{}\" in its files", run.check, run.title),
            target: Some(run.session.clone()),
        });
    }
    for test in &evidence.flaky {
        reasons.push(HealthReason { kind: HealthReasonKind::FlakyTest, fact: format!("flaky test {} reaches it", leaf(test)), target: Some(test.clone()) });
    }
    for (title, number) in &evidence.stale {
        reasons.push(HealthReason { kind: HealthReasonKind::StaleSection, fact: format!("{title} section stale since #{number}"), target: Some(number.to_string()) });
    }
    if evidence.no_test_reaches {
        reasons.push(HealthReason { kind: HealthReasonKind::NoTestReaches, fact: "no test reaches its files".to_string(), target: None });
    }
    for (target, title) in &evidence.bugs {
        let shown = if title.is_empty() { target } else { title };
        reasons.push(HealthReason { kind: HealthReasonKind::OpenBug, fact: format!("open bug: {shown}"), target: Some(target.clone()) });
    }
    let poor = !evidence.failed.is_empty() || evidence.flaky.len() >= FLAKY_POOR || evidence.bugs.len() >= BUGS_POOR;
    let mark = match (poor, reasons.is_empty()) {
        (true, _) => HealthMark::Poor,
        (false, false) => HealthMark::Watch,
        (false, true) => HealthMark::Good,
    };
    FeatureHealth { mark, reasons }
}

/// What the repository's own sources say, read once and asked of each of its features.
#[derive(Default)]
struct Reach {
    /// The ids of the features some test reaches. `None` where the test map holds nothing for the repository.
    reached: Option<BTreeSet<String>>,
    /// Per feature id, the flaky tests that reach it.
    flaky: BTreeMap<String, Vec<String>>,
    /// The Checks whose last run in a Job failed.
    failed: Vec<FailedRun>,
}

impl Reach {
    fn read(store: &Store, loaded: &Loaded, manifest: &str) -> Reach {
        let repository = ipc::ManifestId::carried(manifest).to_domain();
        let mut reach = Reach::default();
        let files: BTreeSet<String> = store.files_tests_reach(&repository).unwrap_or_default().into_iter().collect();
        if !files.is_empty() {
            reach.reached = Some(loaded.map.owning_any(Some(manifest), &files));
        }
        for row in store.standing_in_order(crate::flaky_tests::TEST, manifest, None).unwrap_or_default() {
            let files: BTreeSet<String> = store.files_of_test_named(&repository, &row.target).unwrap_or_default().into_iter().collect();
            for feature in loaded.map.owning_any(Some(manifest), &files) {
                let tests = reach.flaky.entry(feature).or_default();
                if !tests.contains(&row.target) {
                    tests.push(row.target.clone());
                }
            }
        }
        let mut seen = BTreeSet::new();
        for run in store.session_checks(Some(manifest), CHECK_RUNS_READ).unwrap_or_default() {
            // Newest first, so the first of a Job's Check is its last run.
            if run.state != "running" && seen.insert((run.session_id.clone(), run.name.clone())) && run.state == "failed" {
                reach.failed.push(FailedRun { session: run.session_id, title: String::new(), check: run.name });
            }
        }
        reach
    }
}

/// Reads the health of a project's features: each repository's sources once, then each feature's links.
pub(super) struct Health<'a> {
    store: &'a Store,
    loaded: &'a Loaded,
    /// The project's repositories, which a feature that names none is read in.
    repositories: Vec<String>,
    by_manifest: BTreeMap<String, Reach>,
}

impl<'a> Health<'a> {
    pub(super) fn new(store: &'a Store, loaded: &'a Loaded, repositories: Vec<String>) -> Health<'a> {
        Health { store, loaded, repositories, by_manifest: BTreeMap::new() }
    }

    /// `feature`'s health, given its sections with what each is stale since and the live Jobs on it.
    pub(super) fn of(&mut self, feature: &KeptFeature, marked: &[(KeptSection, Option<u64>)], jobs_now: &[FeatureJob]) -> FeatureHealth {
        let (store, loaded) = (self.store, self.loaded);
        let manifests = match feature.manifest_id.is_empty() {
            true => self.repositories.clone(),
            false => vec![feature.manifest_id.clone()],
        };
        let mut evidence = Evidence::default();
        let mut fed = false;
        for manifest in &manifests {
            let reach = self.by_manifest.entry(manifest.clone()).or_insert_with(|| Reach::read(store, loaded, manifest));
            fed |= reach.reached.is_some();
            for test in reach.flaky.get(&feature.id).into_iter().flatten() {
                if !evidence.flaky.contains(test) {
                    evidence.flaky.push(test.clone());
                }
            }
            for run in &reach.failed {
                if let Some(job) = jobs_now.iter().find(|job| job.session_id.as_str() == run.session) {
                    evidence.failed.push(FailedRun { title: job.title.clone(), ..run.clone() });
                }
            }
        }
        // Reached in any repository the feature is in is reached, and a map that holds nothing says nothing.
        let reached = manifests.iter().any(|manifest| self.by_manifest.get(manifest).and_then(|reach| reach.reached.as_ref()).is_some_and(|reached| reached.contains(&feature.id)));
        evidence.no_test_reaches = fed && !reached;
        evidence.stale = marked.iter().filter_map(|(section, since)| Some((section.section.clone(), (*since)?))).collect();
        evidence.bugs = store
            .feature_links(&feature.id)
            .unwrap_or_default()
            .into_iter()
            .filter(|link| link.kind == feature_link_kind::BUG && link.state != "closed")
            .rev()
            .map(|link| (link.target, link.title))
            .collect();
        health_of(&evidence)
    }
}
