//! What the fake forge says about the base branch's newest commit, scripted
//! per commit, with every ask counted so a test can prove one was not made.

use std::collections::BTreeMap;
use std::sync::Mutex;

use adapter_traits::{
    CiRun, CiRuns, FromOutside, MergeQueue, MergedPull, OpenPulls,
    OpenPull, RecentlyMergedPulls, FollowedPull, FollowedPulls,
};

#[derive(Debug, Default)]
struct Script {
    head: Option<String>,
    runs: BTreeMap<String, CiRuns>,
    queue_commits: BTreeMap<u64, String>,
    logs: BTreeMap<String, String>,
    merged: BTreeMap<String, MergedPull>,
    pulls: Option<OpenPulls>,
    queue: Option<MergeQueue>,
    followed: Option<FollowedPulls>,
    by_number: Vec<FollowedPull>,
    open_by_number: Vec<OpenPull>,
    asked_by_number: Vec<u64>,
    open_by_author: Option<OpenPulls>,
    asked_by_author: Vec<String>,
    recent: Option<RecentlyMergedPulls>,
    asked_recent: usize,
    asked_head: usize,
    asked_runs: usize,
    asked_logs: usize,
    asked_merged: usize,
    asked_pulls: usize,
    gates: Vec<String>,
    login: Option<String>,
}

/// Unscripted is the forge's silence: no head, no runs, no log, no pull request, no listing.
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

    /// The commit the merge queue last tested pull request `number` on.
    pub fn queue_commit_is(&self, number: u64, commit: &str) {
        self.with(|it| it.queue_commits.insert(number, commit.to_string()));
    }

    /// The log the forge gives for the job with this handle.
    pub fn log_of(&self, handle: &str, text: &str) {
        self.with(|it| it.logs.insert(handle.to_string(), text.to_string()));
    }

    /// The pull request the forge maps `commit` to.
    pub fn merged_by(&self, commit: &str, pull: MergedPull) {
        self.with(|it| it.merged.insert(commit.to_string(), pull));
    }

    /// The merge queue the forge reports; `None` is a forge that would not answer.
    pub fn queue_is(&self, queue: Option<MergeQueue>) {
        self.with(|it| it.queue = queue);
    }

    /// The open pull requests the forge lists; `None` is a forge that would not answer.
    pub fn pulls_are(&self, pulls: Option<OpenPulls>) {
        self.with(|it| it.pulls = pulls);
    }

    /// The open pull requests as a pull reading reads them; `None` is a forge that would not answer.
    pub fn followed_are(&self, followed: Option<FollowedPulls>) {
        self.with(|it| it.followed = followed);
    }

    /// The pull requests the forge answers for by number and not in its lists, as a pull reading reads
    /// them: a pull request the lists' cap left out. An unscripted number is the forge's silence.
    pub fn followed_by_number_are(&self, followed: Vec<FollowedPull>) {
        self.with(|it| it.by_number = followed);
    }

    /// The same, as the forge's listing reads them.
    pub fn open_by_number_are(&self, pulls: Vec<OpenPull>) {
        self.with(|it| it.open_by_number = pulls);
    }

    /// The open pull requests the forge answers for when asked for one author's, each answered to
    /// its own author in any case: a pull request the listing's cap left out. `None`, the default,
    /// is a forge that would not answer.
    pub fn open_by_author_are(&self, pulls: Option<OpenPulls>) {
        self.with(|it| it.open_by_author = pulls);
    }

    /// Every author open pull requests were asked for by, in order.
    pub fn authors_asked(&self) -> Vec<String> {
        self.with(|it| it.asked_by_author.clone())
    }

    /// Every number a pull request was asked for by, in order, whichever reading asked.
    pub fn numbers_asked(&self) -> Vec<u64> {
        self.with(|it| it.asked_by_number.clone())
    }

    /// The recently merged pull requests the forge lists, newest first.
    pub fn recently_merged_are(&self, recent: Option<RecentlyMergedPulls>) {
        self.with(|it| it.recent = recent);
    }

    /// The gate every pull request reading was asked with, in order: which check a repository
    /// says its `ci` is read from.
    pub fn gates_asked(&self) -> Vec<String> {
        self.with(|it| it.gates.clone())
    }

    /// The login the forge's command line is signed in as on this machine; `None`, the default, is
    /// one that would not say.
    pub fn login_is(&self, login: Option<&str>) {
        self.with(|it| it.login = login.map(str::to_string));
    }

    pub fn times_asked_for_the_recently_merged(&self) -> usize {
        self.with(|it| it.asked_recent)
    }

    pub fn times_asked_for_the_pulls(&self) -> usize {
        self.with(|it| it.asked_pulls)
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

    pub(super) fn queue_commit(&self, number: u64) -> Option<String> {
        self.with(|it| it.queue_commits.get(&number).cloned())
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

    pub(super) fn recently_merged(&self, limit: usize) -> Option<RecentlyMergedPulls> {
        self.with(|it| {
            it.asked_recent += 1;
            it.recent
                .clone()
                .map(|list| list.into_iter().take(limit).collect())
        })
    }

    pub(super) fn merge_queue(&self) -> Option<MergeQueue> {
        self.with(|it| it.queue.clone())
    }

    pub(super) fn login(&self) -> Option<String> {
        self.with(|it| it.login.clone())
    }

    pub(super) fn gated_by(&self, gate: &str) {
        self.with(|it| it.gates.push(gate.to_string()));
    }

    pub(super) fn followed(&self) -> Option<FollowedPulls> {
        self.with(|it| it.followed.clone())
    }

    pub(super) fn open_pulls(&self) -> Option<OpenPulls> {
        self.with(|it| {
            it.asked_pulls += 1;
            it.pulls.clone()
        })
    }

    pub(super) fn followed_one(&self, number: u64) -> Option<FollowedPull> {
        self.with(|it| {
            it.asked_by_number.push(number);
            it.by_number.iter().find(|pull| pull.number == number).cloned()
        })
    }

    pub(super) fn open_one(&self, number: u64) -> Option<OpenPull> {
        self.with(|it| {
            it.asked_by_number.push(number);
            it.open_by_number.iter().find(|pull| pull.number == number).cloned()
        })
    }

    pub(super) fn open_of(&self, author: &str) -> Option<OpenPulls> {
        self.with(|it| {
            it.asked_by_author.push(author.to_string());
            let theirs = |pull: &&OpenPull| {
                pull.author.as_ref().is_some_and(|by| by.as_written().eq_ignore_ascii_case(author))
            };
            it.open_by_author.as_ref().map(|pulls| pulls.iter().filter(theirs).cloned().collect())
        })
    }
}
