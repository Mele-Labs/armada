//! Workspaces, lifecycles and rules: what applies to a repository, where a Job's tickets, pull
//! requests, deploys and started Jobs stand, and saving or deleting one piece in Kit. Nothing here
//! moves an instance.
//!
//! **Its own surface**, for [`Watches`](super::Watches)' reason: whole on its own, and the Job's read
//! is placed by the connection as a watch's is.

use std::future::Future;

use ipc::{
    DeleteLifecyclePiece, JobLifecycles, KeepDrafts, LifecyclePieceDeleted, LifecyclePieceSaved, ManifestId,
    RepositoryLifecycleRules, RepositoryLifecycles, RepositoryWorkspaces, SaveLifecyclePiece, SessionId,
};

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

    /// `list_keep_drafts` — the rules and lifecycles Jobs kept for every Job in one repository that still
    /// wait on the owner's Approve, one per piece, oldest first.
    fn list_keep_drafts(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<KeepDrafts, Refusal>> + Send;

    /// `get_job_lifecycles` — what the placed Job holds and where it stands. `caller` places a hosted
    /// Job; else `session_id` names one. [`Refusal::Unacceptable`] where nothing places a Job,
    /// [`Refusal::NotFound`] where Fleet holds no such Job.
    fn get_job_lifecycles(
        &self,
        caller: Option<crate::Caller>,
        session_id: Option<SessionId>,
    ) -> impl Future<Output = Result<JobLifecycles, Refusal>> + Send;

    /// `save_lifecycle_piece` — write one Workspace, lifecycle, rule or autonomy default in Kit's
    /// folder, as the file its parser reads. **Checked with that parser before anything is
    /// written**: [`Refusal::Unacceptable`] with the parser's own sentence where the piece does not
    /// parse, and where Kit already holds the kind, id and scope and `overwrite` was not set.
    /// [`Refusal::Fault`] where the file cannot be written.
    fn save_lifecycle_piece(
        &self,
        save: SaveLifecyclePiece,
    ) -> impl Future<Output = Result<LifecyclePieceSaved, Refusal>> + Send;

    /// `delete_lifecycle_piece` — delete the Kit file that declares one piece.
    /// [`Refusal::Unacceptable`] where Kit holds none.
    fn delete_lifecycle_piece(
        &self,
        delete: DeleteLifecyclePiece,
    ) -> impl Future<Output = Result<LifecyclePieceDeleted, Refusal>> + Send;
}
