//! A repository's open issues, read from its tracker. #91,
//! `docs/concepts/tracker.md`.
//!
//! **Its own surface**, for [`PullRequests`](super::PullRequests)' reason: a
//! handler that takes only this cannot reach for a Job.

use std::future::Future;

use ipc::{IssueList, ManifestId};

use crate::daemon::Refusal;

pub trait Issues: Send + Sync + 'static {
    /// `list_issues` — the open issues Fleet last read, or a fresh read where
    /// `refresh` asks for one or what is held is old. [`Refusal::Fault`] where
    /// the tracker would not answer and nothing is held.
    fn list_issues(
        &self,
        manifest_id: ManifestId,
        refresh: bool,
    ) -> impl Future<Output = Result<IssueList, Refusal>> + Send;
}
