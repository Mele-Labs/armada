//! What the fake forge says about the base branch's newest commit, scripted
//! per commit, with every ask counted so a test can prove one was not made.

use std::collections::BTreeMap;
use std::sync::Mutex;

use adapter_traits::{CiRun, CiRuns, FromOutside, MergedPull};

#[derive(Debug, Default)]
struct Script {
    head: Option<String>,
    runs: BTreeMap<String, CiRuns>,
    logs: BTreeMap<String, String>,
    merged: BTreeMap<String, MergedPull>,
    asked_head: usize,
    asked_runs: usize,
    asked_logs: usize,
    asked_merged: usize,
}

/// Unscripted is the forge's silence: no head, no runs, no log, no pull request.
#[derive(Debug, Default)]
pub struct MainCiScript(Mutex<Script>);

impl MainCiScript {
    fn with<T>(&self, then: impl FnOnce(&mut Script) -> T) -> T {
        then(&mut self.0.lock().expect("not poisoned"))
    }

    /// Main's head on the forge now; `None` is a forge that would not answer.
    pub fn head_is(&self, head: Option<&str>) {
        self.with(|it| it.head = head.map(str::to_string));
    }

    /// The CI jobs the forge ran on `commit`. An empty list is a commit nothing ran on.
    pub fn runs_on(&self, commit: &str, runs: Vec<CiRun>) {
        self.with(|it| it.runs.insert(commit.to_string(), runs));
    }

    /// The log the forge gives for the job with this handle.
    pub fn log_of(&self, handle: &str, text: &str) {
        self.with(|it| it.logs.insert(handle.to_string(), text.to_string()));
    }

    /// The pull request the forge maps `commit` to.
    pub fn merged_by(&self, commit: &str, pull: MergedPull) {
        self.with(|it| it.merged.insert(commit.to_string(), pull));
    }

    pub fn times_asked_for_the_head(&self) -> usize {
        self.with(|it| it.asked_head)
    }

    pub fn times_asked_for_runs(&self) -> usize {
        self.with(|it| it.asked_runs)
    }

    pub fn times_asked_for_a_log(&self) -> usize {
        self.with(|it| it.asked_logs)
    }

    pub fn times_asked_for_the_merge(&self) -> usize {
        self.with(|it| it.asked_merged)
    }

    pub(super) fn head(&self) -> Option<String> {
        self.with(|it| {
            it.asked_head += 1;
            it.head.clone()
        })
    }

    pub(super) fn runs(&self, commit: &str) -> Option<CiRuns> {
        self.with(|it| {
            it.asked_runs += 1;
            it.runs.get(commit).cloned()
        })
    }

    pub(super) fn log(&self, run: &CiRun) -> Option<FromOutside> {
        self.with(|it| {
            it.asked_logs += 1;
            it.logs
                .get(run.handle.as_written())
                .map(|text| FromOutside::verbatim(text.as_str()))
        })
    }

    pub(super) fn merged(&self, commit: &str) -> Option<MergedPull> {
        self.with(|it| {
            it.asked_merged += 1;
            it.merged.get(commit).cloned()
        })
    }
}
