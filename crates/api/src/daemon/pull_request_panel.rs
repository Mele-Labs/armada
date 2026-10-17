//! What a pull request's panel reads when it opens, and the one write its
//! comment box makes. Since 23.83.
//!
//! Beside [`PullRequests`](super::PullRequests), which answers the pull
//! request's standing and rides every `session.changed`: a conversation is
//! read when somebody looks, not carried.

use std::future::Future;

use ipc::{
    ManifestId, PullRequestFilePatch, PullRequestPage, PullReviewLists, PullThreadResolved,
    ResolvePullThread, ReviewRun, ReviewRuns, ReviewTools, SaveReviewTool, StartReviewRun,
    WritePullRequest,
};

use crate::daemon::Refusal;

pub trait PullRequestPanel: Send + Sync + 'static {
    /// `get_pull_request_detail` — the conversation, checks, files and commits
    /// as the forge shows them. [`Refusal::Fault`] where it would not answer.
    fn get_pull_request_detail(
        &self,
        manifest_id: ManifestId,
        number: u64,
    ) -> impl Future<Output = Result<PullRequestPage, Refusal>> + Send;

    /// `get_pull_request_file` — one file's patch, read when a person opens it.
    /// [`Refusal::Fault`] where the forge would not answer,
    /// [`Refusal::Unacceptable`] for a path the pull request does not list.
    fn get_pull_request_file(
        &self,
        manifest_id: ManifestId,
        number: u64,
        path: String,
    ) -> impl Future<Output = Result<PullRequestFilePatch, Refusal>> + Send;

    /// `write_pull_request` — a comment, an approval or a request for changes.
    /// **A write to a shared place, a person's ask.** Answers the page read
    /// again afterwards. [`Refusal::Unacceptable`] for a comment or a request
    /// with no body, and for a Session the ledger does not know.
    fn write_pull_request(
        &self,
        manifest_id: ManifestId,
        write: WritePullRequest,
    ) -> impl Future<Output = Result<PullRequestPage, Refusal>> + Send;

    /// `get_review_lists` — the pull requests waiting on the person, as four lists read from
    /// the forge across every served repository. **Read when somebody looks, never polled.**
    /// [`Refusal::Fault`] where no list could be read, or no login is known.
    fn get_review_lists(&self) -> impl Future<Output = Result<PullReviewLists, Refusal>> + Send;

    /// `resolve_review_thread` — resolve a conversation on a pull request's diff, or open it
    /// again. **A write to a shared place, taken at once**, so a person's press.
    /// [`Refusal::IllegalMove`] with the forge's own sentence where it would not take it.
    fn resolve_review_thread(
        &self,
        manifest_id: ManifestId,
        resolve: ResolvePullThread,
    ) -> impl Future<Output = Result<PullThreadResolved, Refusal>> + Send;

    /// `list_review_tools` — the review tools and explanations a repository has, repository
    /// over Kit over Armada, and the files left out and why. Read from the folders each time.
    fn list_review_tools(
        &self,
        manifest_id: ManifestId,
    ) -> impl Future<Output = Result<ReviewTools, Refusal>> + Send;

    /// `save_review_tool` — write one definition in Kit's folder or the repository's.
    /// [`Refusal::Unacceptable`] for a blank or wordless name, a blank prompt, and an id the
    /// scope already defines unless `overwrite`; [`Refusal::Fault`] where it cannot be written.
    fn save_review_tool(
        &self,
        manifest_id: ManifestId,
        save: SaveReviewTool,
    ) -> impl Future<Output = Result<ReviewTools, Refusal>> + Send;

    /// `start_review_run` — ask an agent, read-only, about a pull request. **Answers at once**:
    /// a kept result, the run already going, or a new one queued whose work goes on a task of
    /// its own. [`Refusal::Unacceptable`] for a name no definition has or a blank question;
    /// [`Refusal::Fault`] where the forge would not give the pull request.
    fn start_review_run(
        self: std::sync::Arc<Self>,
        manifest_id: ManifestId,
        start: StartReviewRun,
    ) -> impl Future<Output = Result<ReviewRun, Refusal>> + Send;

    /// `get_review_runs` — every run kept or going for a pull request, newest first.
    fn get_review_runs(
        &self,
        manifest_id: ManifestId,
        number: u64,
    ) -> impl Future<Output = Result<ReviewRuns, Refusal>> + Send;
}
