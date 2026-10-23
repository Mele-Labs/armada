//! The `gh` command line as the forge: every [`Forge`] method, each a call into
//! the module that runs `gh` for it.
//!
//! **One adapter of the forge, not the forge.** [`GitVcs`](crate::GitVcs) asks
//! whichever [`Forge`] it was built with, and this is the one
//! [`GitVcs::new`](crate::GitVcs::new) builds with.

use adapter_traits::{
    Base, Forge, LandingOnTheForge, Merged, NotDelivered, NotMerged, Opened, Review, UnderReview,
    Worktree,
};

/// The forge, asked through `gh` in the repository each call names.
#[derive(Debug, Default, Clone, Copy)]
pub struct GhForge;

impl Forge for GhForge {
    fn open_for_review(
        &self,
        worktree: &Worktree,
        base: &Base,
        review: &Review,
    ) -> Result<Opened, NotDelivered> {
        crate::delivery::open_for_review(worktree, base, review)
    }

    fn under_review(&self, in_repo: &str, pull_request: &str, gate: &str) -> UnderReview {
        crate::under_review::read(in_repo, pull_request, gate)
    }

    fn inline_remarks(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<Vec<adapter_traits::Remark>> {
        crate::inline_comments::read(in_repo, pull_request)
    }

    fn base_head_on_the_forge(&self, in_repo: &str, base: &str) -> Option<String> {
        crate::main_ci::head(in_repo, base)
    }

    fn ci_runs_on(&self, in_repo: &str, commit: &str) -> Option<adapter_traits::CiRuns> {
        crate::main_ci::runs(in_repo, commit)
    }

    fn queue_commit_of(&self, in_repo: &str, base: &str, number: u64) -> Option<String> {
        crate::main_ci::queue_commit_of(in_repo, base, number)
    }

    fn ci_log(
        &self,
        in_repo: &str,
        run: &adapter_traits::CiRun,
    ) -> Option<adapter_traits::FromOutside> {
        crate::main_ci::log(in_repo, run)
    }

    fn merged_by(&self, in_repo: &str, commit: &str) -> Option<adapter_traits::MergedPull> {
        crate::main_ci::merged_by(in_repo, commit)
    }

    fn merge_queue(&self, in_repo: &str, base: &str) -> Option<adapter_traits::MergeQueue> {
        crate::merge_queue::read(in_repo, base)
    }

    fn open_pull_requests(&self, in_repo: &str, gate: &str) -> Option<adapter_traits::OpenPulls> {
        crate::main_ci::open_pulls(in_repo, gate)
    }

    fn open_pull_request(
        &self,
        in_repo: &str,
        gate: &str,
        number: u64,
    ) -> Option<adapter_traits::OpenPull> {
        crate::main_ci::open_pull(in_repo, gate, number)
    }

    fn pull_reading(&self, in_repo: &str, gate: &str) -> Option<adapter_traits::FollowedPulls> {
        crate::pull_reading::read(in_repo, gate)
    }

    fn followed_pull(
        &self,
        in_repo: &str,
        gate: &str,
        number: u64,
    ) -> Option<adapter_traits::FollowedPull> {
        crate::pull_reading::read_one(in_repo, gate, number)
    }

    fn forge_login(&self, in_repo: &str) -> Option<String> {
        crate::main_ci::forge_login(in_repo)
    }

    fn recently_merged_pull_requests(
        &self,
        in_repo: &str,
        base: &str,
        limit: usize,
    ) -> Option<adapter_traits::RecentlyMergedPulls> {
        crate::main_ci::recently_merged(in_repo, base, limit)
    }

    fn pull_request_diff(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<adapter_traits::PullRequestDiff> {
        crate::pull_request_diff::read(in_repo, pull_request)
    }

    fn rerun_failed(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Result<adapter_traits::Rerun, adapter_traits::NotRerun> {
        crate::rerunning::rerun_failed(in_repo, pull_request)
    }

    fn recorded_laps(
        &self,
        in_repo: &str,
        commit: &str,
    ) -> Result<adapter_traits::LapsOnForge, adapter_traits::NotAsked> {
        crate::recorded_laps::fetch(in_repo, commit)
    }

    fn file_issue(
        &self,
        in_repo: &str,
        title: &str,
        body: &str,
    ) -> Result<adapter_traits::FiledIssue, adapter_traits::NotFiled> {
        crate::filing::file_issue(in_repo, title, body)
    }

    fn merge(&self, in_repo: &str, pull_request: &str) -> Result<Merged, NotMerged> {
        crate::landing::merge(in_repo, pull_request)
    }

    fn enable_auto_merge(&self, in_repo: &str, pull_request: &str) -> Result<(), String> {
        crate::landing::enable_auto_merge(in_repo, pull_request)
    }

    fn disable_auto_merge(&self, in_repo: &str, pull_request: &str) -> Result<(), String> {
        crate::landing::disable_auto_merge(in_repo, pull_request)
    }

    fn pull_request_facts(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<adapter_traits::PullRequestFacts> {
        crate::pull_request_facts::read(in_repo, pull_request)
    }

    fn mark_ready(&self, in_repo: &str, pull_request: &str) -> Result<(), String> {
        crate::pull_request_facts::mark_ready(in_repo, pull_request)
    }

    fn pull_request_page(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<adapter_traits::PullRequestPageFacts> {
        crate::pull_request_detail::read(in_repo, pull_request)
    }

    fn pull_request_file_patch(
        &self,
        in_repo: &str,
        pull_request: &str,
        path: &str,
    ) -> Option<adapter_traits::FilePatch> {
        crate::pull_request_patch::read(in_repo, pull_request, path)
    }

    fn pull_request_patches(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<Vec<adapter_traits::PatchedFile>> {
        crate::pull_request_patch::read_every(in_repo, pull_request)
    }

    fn write_pull_request(
        &self,
        in_repo: &str,
        pull_request: &str,
        write: &adapter_traits::PullRequestWrite,
    ) -> Result<(), String> {
        crate::pull_request_writes::write(in_repo, pull_request, write)
    }

    fn pull_review_list(
        &self,
        in_repo: &str,
        ask: &adapter_traits::ReviewListAsk<'_>,
    ) -> Option<Vec<adapter_traits::ListedPull>> {
        crate::pull_review_lists::read(in_repo, ask)
    }

    fn resolve_review_thread(
        &self,
        in_repo: &str,
        thread: &str,
        resolved: bool,
    ) -> Result<bool, String> {
        crate::pull_request_threads::resolve(in_repo, thread, resolved)
    }

    fn landing(&self, in_repo: &str, pull_request: &str) -> LandingOnTheForge {
        crate::landing::read(in_repo, pull_request)
    }

    fn open_branch_for_review(
        &self,
        in_repo: &str,
        base: &str,
        branch: &str,
        title: &str,
        body: &str,
    ) -> Result<String, NotDelivered> {
        crate::manifest_edit::opened(in_repo, base, branch, title, body)
    }
}
