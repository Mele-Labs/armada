//! A pull request by repository and number, and the acts a Session takes on
//! one. Since 23.48. `docs/concepts/session.md`, *Acts on a pull request*.
//!
//! **Its own surface**, for [`Sessions`](super::Sessions)' reason: whole on its
//! own, and a handler that takes only this cannot reach for a Job. A pull
//! request here is nobody's Job, which is why the acts name a repository and a
//! number rather than a Job id.

use std::future::Future;
use std::sync::Arc;

use ipc::{ManifestId, PullRequestState};

use crate::daemon::Refusal;

pub trait PullRequests: Send + Sync + 'static {
    /// `get_pull_request` — what the forge shows now, and the same read
    /// refreshed into every Session's `pr` row for it. [`Refusal::Fault`] where
    /// the forge would not answer.
    fn get_pull_request(
        &self,
        manifest_id: ManifestId,
        number: u64,
    ) -> impl Future<Output = Result<PullRequestState, Refusal>> + Send;

    /// `ready_pull_request` — take a draft out of draft. **A write to the
    /// forge, a person's ask**, answered once the forge has taken it: the ledger
    /// follows after the reply, as a `session.changed`. [`Refusal::IllegalMove`] where it is not a
    /// draft or is over.
    fn ready_pull_request(
        self: Arc<Self>,
        manifest_id: ManifestId,
        number: u64,
    ) -> impl Future<Output = Result<PullRequestState, Refusal>> + Send;

    /// `merge_pull_request_by_number` — merge it, **only once the forge's
    /// checks have passed**. The one write that changes what everybody else
    /// builds on. [`Refusal::IllegalMove`] with a code naming the reason.
    fn merge_pull_request_by_number(
        self: Arc<Self>,
        manifest_id: ManifestId,
        number: u64,
    ) -> impl Future<Output = Result<PullRequestState, Refusal>> + Send;

    /// `enable_auto_merge` — ask the forge to merge it once its checks pass,
    /// **while they are still running**. Merges nothing itself.
    fn enable_auto_merge(
        self: Arc<Self>,
        manifest_id: ManifestId,
        number: u64,
    ) -> impl Future<Output = Result<PullRequestState, Refusal>> + Send;

    /// `refresh_pull_request` — read it now rather than at the next pull reading, and **do all the
    /// watch does for it**: the ledger, the merge line, and what its owner is told. A person's
    /// press, never an agent's. [`Refusal::Fault`] where the forge would not answer.
    fn refresh_pull_request(
        self: Arc<Self>,
        manifest_id: ManifestId,
        number: u64,
    ) -> impl Future<Output = Result<PullRequestState, Refusal>> + Send;
}
