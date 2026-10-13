//! What a pull request's panel reads when it opens, and the one write its
//! comment box makes. Since 23.83.
//!
//! Beside [`PullRequests`](super::PullRequests), which answers the pull
//! request's standing and rides every `session.changed`: a conversation is
//! read when somebody looks, not carried.

use std::future::Future;

use ipc::{ManifestId, PullRequestPage, WritePullRequest};

use crate::daemon::Refusal;

pub trait PullRequestPanel: Send + Sync + 'static {
    /// `get_pull_request_detail` — the conversation, checks, files and commits
    /// as the forge shows them. [`Refusal::Fault`] where it would not answer.
    fn get_pull_request_detail(
        &self,
        manifest_id: ManifestId,
        number: u64,
    ) -> impl Future<Output = Result<PullRequestPage, Refusal>> + Send;

    /// `write_pull_request` — a comment, an approval or a request for changes.
    /// **A write to a shared place, a person's ask.** Answers the page read
    /// again afterwards. [`Refusal::Unacceptable`] for a comment or a request
    /// with no body, and for a Session the ledger does not know.
    fn write_pull_request(
        &self,
        manifest_id: ManifestId,
        write: WritePullRequest,
    ) -> impl Future<Output = Result<PullRequestPage, Refusal>> + Send;
}
