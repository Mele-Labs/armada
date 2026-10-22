//! The forge asked over GitHub's API where it can be, and through `gh` where it cannot.
//!
//! **Per repository, never per call's mood.** A checkout whose `origin` is on `github.com` is
//! asked over [`GitHubApi`]; any other, or one with no remote, goes to [`GhForge`] whole. A call
//! `gh` holds no token for ([`ApiFailure::NoToken`]) also goes to `gh`, so nothing answers worse
//! than it did before. Any other failure is the forge's silence, as `gh`'s is.
//!
//! **Moved one method at a time.** A method not yet read over the API is `gh`'s, unchanged.

mod ci;
mod page;
mod patches;
mod review_lists;
mod threads;

use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use adapter_traits::{
    Base, Forge, LandingOnTheForge, Merged, NotDelivered, NotMerged, Opened, Review, UnderReview,
    Worktree,
};
use serde::Deserialize;

use crate::gh_forge::GhForge;
use crate::github_api::{ApiFailure, GitHubApi, GitHubRepository};

/// Every [`Forge`] method, over GitHub's API for a repository on `github.com`.
#[derive(Clone)]
pub struct ApiForge {
    api: GitHubApi,
    gh: GhForge,
    /// Which repository each checkout is, read from its remote once.
    known: Arc<Mutex<HashMap<String, Option<GitHubRepository>>>>,
}

impl fmt::Debug for ApiForge {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str("ApiForge")
    }
}

impl Default for ApiForge {
    fn default() -> ApiForge {
        ApiForge::new()
    }
}

impl ApiForge {
    /// Asking `api.github.com` through a client of its own, which every clone of this shares.
    /// Makes no connection until the first call.
    pub fn new() -> ApiForge {
        ApiForge::through(GitHubApi::new())
    }

    /// Asking through `api`, shared with whoever else holds it.
    pub fn through(api: GitHubApi) -> ApiForge {
        ApiForge { api, gh: GhForge, known: Arc::default() }
    }

    /// The repository `in_repo` is, if it is on `github.com`.
    fn on_github(&self, in_repo: &str) -> Option<GitHubRepository> {
        let mut known = self.known.lock().unwrap_or_else(PoisonError::into_inner);
        known
            .entry(in_repo.to_string())
            .or_insert_with(|| GitHubRepository::of_checkout(Path::new(in_repo)).ok())
            .clone()
    }

    /// The API's answer where the repository is GitHub's and `gh` holds a token, else `gh`'s.
    pub(crate) fn read<T>(
        &self,
        in_repo: &str,
        api: impl FnOnce(&GitHubApi, &GitHubRepository) -> Result<Option<T>, ApiFailure>,
        gh: impl FnOnce(&GhForge) -> Option<T>,
    ) -> Option<T> {
        let Some(repository) = self.on_github(in_repo) else {
            return gh(&self.gh);
        };
        match api(&self.api, &repository) {
            Ok(answer) => answer,
            Err(ApiFailure::NoToken(_)) => gh(&self.gh),
            Err(_) => None,
        }
    }
}

/// A pull request named by number, which is all the API takes; anything else goes to `gh`.
fn numbered(pull_request: &str) -> Option<u64> {
    pull_request.parse().ok()
}

/// `{ "login": ... }`, the shape every actor takes.
#[derive(Deserialize)]
struct Login {
    login: Option<String>,
}

fn login_of(actor: Option<Login>) -> String {
    actor.and_then(|it| it.login).unwrap_or_default()
}

/// A GraphQL list, whatever it holds; a `null` one is empty.
#[derive(Deserialize)]
struct Nodes<T> {
    nodes: Option<Vec<Option<T>>>,
}

impl<T> Nodes<T> {
    fn all(list: Option<Nodes<T>>) -> impl Iterator<Item = T> {
        list.and_then(|it| it.nodes).unwrap_or_default().into_iter().flatten()
    }
}

/// A thread's join, its panic carried on rather than swallowed.
fn joined<T>(thread: std::thread::ScopedJoinHandle<'_, T>) -> T {
    thread.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic))
}

impl Forge for ApiForge {
    fn open_for_review(
        &self,
        worktree: &Worktree,
        base: &Base,
        review: &Review,
    ) -> Result<Opened, NotDelivered> {
        self.gh.open_for_review(worktree, base, review)
    }

    fn under_review(&self, in_repo: &str, pull_request: &str, gate: &str) -> UnderReview {
        self.gh.under_review(in_repo, pull_request, gate)
    }

    fn inline_remarks(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<Vec<adapter_traits::Remark>> {
        self.gh.inline_remarks(in_repo, pull_request)
    }

    fn base_head_on_the_forge(&self, in_repo: &str, base: &str) -> Option<String> {
        self.read(
            in_repo,
            |api, repository| ci::head(api, repository, base),
            |gh| gh.base_head_on_the_forge(in_repo, base),
        )
    }

    fn ci_runs_on(&self, in_repo: &str, commit: &str) -> Option<adapter_traits::CiRuns> {
        self.read(
            in_repo,
            |api, repository| ci::runs(api, repository, commit),
            |gh| gh.ci_runs_on(in_repo, commit),
        )
    }

    fn queue_commit_of(&self, in_repo: &str, base: &str, number: u64) -> Option<String> {
        self.read(
            in_repo,
            |api, repository| ci::queue_commit_of(api, repository, base, number),
            |gh| gh.queue_commit_of(in_repo, base, number),
        )
    }

    fn ci_log(
        &self,
        in_repo: &str,
        run: &adapter_traits::CiRun,
    ) -> Option<adapter_traits::FromOutside> {
        self.read(
            in_repo,
            |api, repository| ci::log(api, repository, run),
            |gh| gh.ci_log(in_repo, run),
        )
    }

    fn merged_by(&self, in_repo: &str, commit: &str) -> Option<adapter_traits::MergedPull> {
        self.read(
            in_repo,
            |api, repository| ci::merged_by(api, repository, commit),
            |gh| gh.merged_by(in_repo, commit),
        )
    }

    fn merge_queue(&self, in_repo: &str, base: &str) -> Option<adapter_traits::MergeQueue> {
        self.gh.merge_queue(in_repo, base)
    }

    fn open_pull_requests(&self, in_repo: &str, gate: &str) -> Option<adapter_traits::OpenPulls> {
        self.gh.open_pull_requests(in_repo, gate)
    }

    fn open_pull_request(
        &self,
        in_repo: &str,
        gate: &str,
        number: u64,
    ) -> Option<adapter_traits::OpenPull> {
        self.gh.open_pull_request(in_repo, gate, number)
    }

    fn pull_watch(&self, in_repo: &str, gate: &str) -> Option<adapter_traits::WatchedPulls> {
        self.gh.pull_watch(in_repo, gate)
    }

    fn watched_pull(
        &self,
        in_repo: &str,
        gate: &str,
        number: u64,
    ) -> Option<adapter_traits::WatchedPull> {
        self.gh.watched_pull(in_repo, gate, number)
    }

    fn forge_login(&self, in_repo: &str) -> Option<String> {
        self.gh.forge_login(in_repo)
    }

    fn recently_merged_pull_requests(
        &self,
        in_repo: &str,
        base: &str,
        limit: usize,
    ) -> Option<adapter_traits::RecentlyMergedPulls> {
        self.gh.recently_merged_pull_requests(in_repo, base, limit)
    }

    fn pull_request_diff(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<adapter_traits::PullRequestDiff> {
        self.gh.pull_request_diff(in_repo, pull_request)
    }

    fn rerun_failed(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Result<adapter_traits::Rerun, adapter_traits::NotRerun> {
        self.gh.rerun_failed(in_repo, pull_request)
    }

    fn file_issue(
        &self,
        in_repo: &str,
        title: &str,
        body: &str,
    ) -> Result<adapter_traits::FiledIssue, adapter_traits::NotFiled> {
        self.gh.file_issue(in_repo, title, body)
    }

    fn merge(&self, in_repo: &str, pull_request: &str) -> Result<Merged, NotMerged> {
        self.gh.merge(in_repo, pull_request)
    }

    fn enable_auto_merge(&self, in_repo: &str, pull_request: &str) -> Result<(), String> {
        self.gh.enable_auto_merge(in_repo, pull_request)
    }

    fn disable_auto_merge(&self, in_repo: &str, pull_request: &str) -> Result<(), String> {
        self.gh.disable_auto_merge(in_repo, pull_request)
    }

    fn pull_request_facts(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<adapter_traits::PullRequestFacts> {
        self.gh.pull_request_facts(in_repo, pull_request)
    }

    fn mark_ready(&self, in_repo: &str, pull_request: &str) -> Result<(), String> {
        self.gh.mark_ready(in_repo, pull_request)
    }

    fn pull_request_page(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<adapter_traits::PullRequestPageFacts> {
        let gh = |gh: &GhForge| gh.pull_request_page(in_repo, pull_request);
        let Some(number) = numbered(pull_request) else { return gh(&self.gh) };
        self.read(in_repo, |api, repository| page::read(api, repository, number), gh)
    }

    fn pull_request_file_patch(
        &self,
        in_repo: &str,
        pull_request: &str,
        path: &str,
    ) -> Option<adapter_traits::FilePatch> {
        self.pull_request_patches(in_repo, pull_request)
            .map(|files| crate::pull_request_patch::folded(files, path))
    }

    fn pull_request_patches(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<Vec<adapter_traits::PatchedFile>> {
        let gh = |gh: &GhForge| gh.pull_request_patches(in_repo, pull_request);
        let Some(number) = numbered(pull_request) else { return gh(&self.gh) };
        self.read(in_repo, |api, repository| patches::read(api, repository, number).map(Some), gh)
    }

    fn write_pull_request(
        &self,
        in_repo: &str,
        pull_request: &str,
        write: &adapter_traits::PullRequestWrite,
    ) -> Result<(), String> {
        self.gh.write_pull_request(in_repo, pull_request, write)
    }

    fn pull_review_list(
        &self,
        in_repo: &str,
        ask: &adapter_traits::ReviewListAsk<'_>,
    ) -> Option<Vec<adapter_traits::ListedPull>> {
        self.read(
            in_repo,
            |api, _| review_lists::read(api, ask),
            |gh| gh.pull_review_list(in_repo, ask),
        )
    }

    fn resolve_review_thread(
        &self,
        in_repo: &str,
        thread: &str,
        resolved: bool,
    ) -> Result<bool, String> {
        self.gh.resolve_review_thread(in_repo, thread, resolved)
    }

    fn landing(&self, in_repo: &str, pull_request: &str) -> LandingOnTheForge {
        self.gh.landing(in_repo, pull_request)
    }

    fn open_branch_for_review(
        &self,
        in_repo: &str,
        base: &str,
        branch: &str,
        title: &str,
        body: &str,
    ) -> Result<String, NotDelivered> {
        self.gh.open_branch_for_review(in_repo, base, branch, title, body)
    }
}
