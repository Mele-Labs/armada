//! Lifecycles and rules: what applies to a repository, where a Job's tickets, pull
//! requests, deploys and started Jobs stand, and saving or deleting one piece in Kit. Nothing here
//! moves an instance.
//!
//! **Its own surface**, for [`Watches`](super::Watches)' reason: whole on its own, and the Job's read
//! is placed by the connection as a watch's is.

use std::future::Future;

use ipc::{
    DeleteLifecyclePiece, JobLanesView, JobLifecycles, KeepDrafts, LifecyclePieceDeleted, LifecyclePieceSaved, ManifestId, PieceDraft,
    PieceDraftDiscarded, PiecePublished, PublishPieceDraft, PutPieceDraft, RepositoryAutonomy, RepositoryLifecycleRules,
    RepositoryLifecycles, SaveLifecyclePiece, SessionId, TeachTicketWord, TicketWordTaught,
};

use crate::daemon::Refusal;

pub trait Lifecycles: Send + Sync + 'static {
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

    /// `get_workflow_lanes` — one workflow's lanes as its editor draws them, at the default level:
    /// `scope` absent or `default`. [`Refusal::Unacceptable`] for a workflow the repository does not
    /// run, or another level.
    fn get_workflow_lanes(
        &self,
        workflow: String,
        scope: Option<String>,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<JobLanesView, Refusal>> + Send;

    /// `list_autonomy_defaults` — Armada's default for every kind of act, and each layer of autonomy
    /// defaults that applies to one repository, unresolved. [`Refusal::Unacceptable`] where Fleet serves no repository by that id.
    fn list_autonomy_defaults(
        &self,
        manifest_id: Option<ManifestId>,
    ) -> impl Future<Output = Result<RepositoryAutonomy, Refusal>> + Send;

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

    /// `teach_ticket_word` — a tracker's own status read as one Ticket state: put on that state in
    /// the Ticket lifecycle Kit keeps for the repository's Workspace and in every live Job's copy
    /// there, and taken off any other. [`Refusal::Unacceptable`] for a blank word, a state the
    /// Ticket lacks, or a Ticket that would not parse; [`Refusal::NotFound`] where Fleet serves no
    /// such repository.
    fn teach_ticket_word(
        &self,
        teach: TeachTicketWord,
    ) -> impl Future<Output = Result<TicketWordTaught, Refusal>> + Send;

    /// `delete_lifecycle_piece` — delete the Kit file that declares one piece.
    /// [`Refusal::Unacceptable`] where Kit holds none.
    fn delete_lifecycle_piece(
        &self,
        delete: DeleteLifecyclePiece,
    ) -> impl Future<Output = Result<LifecyclePieceDeleted, Refusal>> + Send;

    /// `get_piece_draft` — the draft of one piece, keyed as [`Lifecycles::delete_lifecycle_piece`] keys a piece, with its
    /// changes oldest first. [`Refusal::NotFound`] where none is kept.
    fn get_piece_draft(
        &self,
        piece: DeleteLifecyclePiece,
    ) -> impl Future<Output = Result<PieceDraft, Refusal>> + Send;

    /// `put_piece_draft` — keep the owner's text as the draft of a piece, beginning it from what Kit holds where there is
    /// none. **Any text is kept**; `unfit` on the answer says where it would not publish.
    fn put_piece_draft(&self, put: PutPieceDraft) -> impl Future<Output = Result<PieceDraft, Refusal>> + Send;

    /// `discard_piece_draft` — throw away a draft and its changes. [`Refusal::NotFound`] where none is kept.
    fn discard_piece_draft(
        &self,
        piece: DeleteLifecyclePiece,
    ) -> impl Future<Output = Result<PieceDraftDiscarded, Refusal>> + Send;

    /// `publish_piece_draft` — decode the draft and run [`Lifecycles::save_lifecycle_piece`] with overwrite, then delete
    /// it. [`Refusal::Unacceptable`] with the parser's sentence where it does not parse; [`Refusal::IllegalMove`] where what
    /// Kit holds is no longer what the draft began from.
    fn publish_piece_draft(
        &self,
        publish: PublishPieceDraft,
    ) -> impl Future<Output = Result<PiecePublished, Refusal>> + Send;
}
