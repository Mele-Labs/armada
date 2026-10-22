//! [`GitVcs`] answering as a forge by asking the one it holds, so Fleet's
//! `V: Vcs + Delivery` reaches the forge without a second type parameter.

use adapter_traits::{
    Base, FiledIssue, Forge, LandingOnTheForge, Merged, NotDelivered, NotFiled, NotMerged,
    NotRerun, Opened, Remark, Rerun, Review, UnderReview, Worktree,
};

use crate::worktree::GitVcs;

impl<F: Forge> Forge for GitVcs<F> {
    fn open_for_review(
        &self,
        worktree: &Worktree,
        base: &Base,
        review: &Review,
    ) -> Result<Opened, NotDelivered> {
        self.forge.open_for_review(worktree, base, review)
    }

    fn under_review(&self, in_repo: &str, pull_request: &str, gate: &str) -> UnderReview {
        self.forge.under_review(in_repo, pull_request, gate)
    }

    fn inline_remarks(&self, in_repo: &str, pull_request: &str) -> Option<Vec<Remark>> {
        self.forge.inline_remarks(in_repo, pull_request)
    }

    fn base_head_on_the_forge(&self, in_repo: &str, base: &str) -> Option<String> {
        self.forge.base_head_on_the_forge(in_repo, base)
    }

    fn ci_runs_on(&self, in_repo: &str, commit: &str) -> Option<adapter_traits::CiRuns> {
        self.forge.ci_runs_on(in_repo, commit)
    }

    fn queue_commit_of(&self, in_repo: &str, base: &str, number: u64) -> Option<String> {
        self.forge.queue_commit_of(in_repo, base, number)
    }

    fn ci_log(&self, in_repo: &str, run: &adapter_traits::CiRun) -> Option<adapter_traits::FromOutside> {
        self.forge.ci_log(in_repo, run)
    }

    fn merged_by(&self, in_repo: &str, commit: &str) -> Option<adapter_traits::MergedPull> {
        self.forge.merged_by(in_repo, commit)
    }

    fn open_pull_requests(&self, in_repo: &str, gate: &str) -> Option<adapter_traits::OpenPulls> {
        self.forge.open_pull_requests(in_repo, gate)
    }

    fn open_pull_request(
        &self,
        in_repo: &str,
        gate: &str,
        number: u64,
    ) -> Option<adapter_traits::OpenPull> {
        self.forge.open_pull_request(in_repo, gate, number)
    }

    fn merge_queue(&self, in_repo: &str, base: &str) -> Option<adapter_traits::MergeQueue> {
        self.forge.merge_queue(in_repo, base)
    }

    fn pull_watch(&self, in_repo: &str, gate: &str) -> Option<adapter_traits::WatchedPulls> {
        self.forge.pull_watch(in_repo, gate)
    }

    fn watched_pull(
        &self,
        in_repo: &str,
        gate: &str,
        number: u64,
    ) -> Option<adapter_traits::WatchedPull> {
        self.forge.watched_pull(in_repo, gate, number)
    }

    fn forge_login(&self, in_repo: &str) -> Option<String> {
        self.forge.forge_login(in_repo)
    }

    fn recently_merged_pull_requests(
        &self,
        in_repo: &str,
        base: &str,
        limit: usize,
    ) -> Option<adapter_traits::RecentlyMergedPulls> {
        self.forge.recently_merged_pull_requests(in_repo, base, limit)
    }

    fn pull_request_diff(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<adapter_traits::PullRequestDiff> {
        self.forge.pull_request_diff(in_repo, pull_request)
    }

    fn rerun_failed(&self, in_repo: &str, pull_request: &str) -> Result<Rerun, NotRerun> {
        self.forge.rerun_failed(in_repo, pull_request)
    }

    fn recorded_laps(&self, in_repo: &str, commit: &str) -> Result<adapter_traits::LapsOnForge, adapter_traits::NotAsked> {
        self.forge.recorded_laps(in_repo, commit)
    }

    fn file_issue(&self, in_repo: &str, title: &str, body: &str) -> Result<FiledIssue, NotFiled> {
        self.forge.file_issue(in_repo, title, body)
    }

    fn merge(&self, in_repo: &str, pull_request: &str) -> Result<Merged, NotMerged> {
        self.forge.merge(in_repo, pull_request)
    }

    fn enable_auto_merge(&self, in_repo: &str, pull_request: &str) -> Result<(), String> {
        self.forge.enable_auto_merge(in_repo, pull_request)
    }

    fn disable_auto_merge(&self, in_repo: &str, pull_request: &str) -> Result<(), String> {
        self.forge.disable_auto_merge(in_repo, pull_request)
    }

    fn pull_request_facts(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<adapter_traits::PullRequestFacts> {
        self.forge.pull_request_facts(in_repo, pull_request)
    }

    fn mark_ready(&self, in_repo: &str, pull_request: &str) -> Result<(), String> {
        self.forge.mark_ready(in_repo, pull_request)
    }

    fn pull_request_page(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<adapter_traits::PullRequestPageFacts> {
        self.forge.pull_request_page(in_repo, pull_request)
    }

    fn pull_request_file_patch(
        &self,
        in_repo: &str,
        pull_request: &str,
        path: &str,
    ) -> Option<adapter_traits::FilePatch> {
        self.forge.pull_request_file_patch(in_repo, pull_request, path)
    }

    fn pull_request_patches(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<Vec<adapter_traits::PatchedFile>> {
        self.forge.pull_request_patches(in_repo, pull_request)
    }

    fn write_pull_request(
        &self,
        in_repo: &str,
        pull_request: &str,
        write: &adapter_traits::PullRequestWrite,
    ) -> Result<(), String> {
        self.forge.write_pull_request(in_repo, pull_request, write)
    }

    fn pull_review_list(
        &self,
        in_repo: &str,
        ask: &adapter_traits::ReviewListAsk<'_>,
    ) -> Option<Vec<adapter_traits::ListedPull>> {
        self.forge.pull_review_list(in_repo, ask)
    }

    fn resolve_review_thread(
        &self,
        in_repo: &str,
        thread: &str,
        resolved: bool,
    ) -> Result<bool, String> {
        self.forge.resolve_review_thread(in_repo, thread, resolved)
    }

    fn landing(&self, in_repo: &str, pull_request: &str) -> LandingOnTheForge {
        self.forge.landing(in_repo, pull_request)
    }

    fn open_branch_for_review(
        &self,
        in_repo: &str,
        base: &str,
        branch: &str,
        title: &str,
        body: &str,
    ) -> Result<String, NotDelivered> {
        self.forge.open_branch_for_review(in_repo, base, branch, title, body)
    }
}
