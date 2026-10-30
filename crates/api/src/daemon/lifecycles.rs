//! Workspaces, lifecycles and rules, read: what applies to a repository, and where a Job's tickets,
//! pull requests, deploys and started Jobs stand. Nothing here saves a piece or moves an instance.
//!
//! **Its own surface**, for [`Watches`](super::Watches)' reason: whole on its own, and the Job's read
//! is placed by the connection as a watch's is.

use std::future::Future;

use ipc::{JobLifecycles, ManifestId, RepositoryLifecycleRules, RepositoryLifecycles, RepositoryWorkspaces, SessionId};

use crate::daemon::Refusal;

pub trait Lifecycles: Send + Sync + 'static {
    /// `list_repository_workspaces` — the Workspaces (sets of repositories) that hold one
    /// repository. Absent `manifest_id` is the repository Fleet started in.
    fn list_repository_workspaces(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<RepositoryWorkspaces, Refusal>> + Send;

    /// `list_lifecycles` — the lifecycles that apply to one repository, and the files set aside.
    fn list_lifecycles(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<RepositoryLifecycles, Refusal>> + Send;

    /// `list_lifecycle_rules` — the rules that apply to one repository, and the files set aside.
    fn list_lifecycle_rules(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<RepositoryLifecycleRules, Refusal>> + Send;

    /// `get_job_lifecycles` — what the placed Job holds and where it stands. `caller` places a hosted
    /// Job; else `session_id` names one. [`Refusal::Unacceptable`] where nothing places a Job,
    /// [`Refusal::NotFound`] where Fleet holds no such Job.
    fn get_job_lifecycles(
        &self,
        caller: Option<crate::Caller>,
        session_id: Option<SessionId>,
    ) -> impl Future<Output = Result<JobLifecycles, Refusal>> + Send;
}
