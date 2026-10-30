//! Keep for every Job, for a rule or a lifecycle a Job changed in its own copy. Protocol
//! `docs/practices/protocol.md`, *A Job's own copy*; the workflow half is `job_copying`.
//!
//! **A piece is proposed as a file in Kit within the Job's repository**, the repository's own level, and
//! nothing is written until the owner approves. `save_lifecycle_piece` writes at once and has no held form,
//! so the proposal waits in the `workflow_changes` row a workflow's does, with `piece` set, and a draft on the
//! Job carries it to the owner's Needs-you list. His Approve is the save; his Dismiss drops the row.

use std::path::Path;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    CopyPieceKind, JobPieceProposed, KeepDraft, KeepDrafts, KeepDraftsChanged, KeepJobPiece, LifecyclePiece, LifecyclePieceKind, ManifestId,
    PieceScope, SaveLifecyclePiece, WireError, WireValue, WorkflowChangeRef,
};
use store::{KeptWorkflowChange, PieceKind};

use super::job_copying::{JOB_COPY_PIECE_REFUSED, JOB_HOLDS_NO_COPY, RULE_LANE};
use crate::daemon::Fleet;
use crate::lifecycles::wire;
use crate::piece_saving::identity::{declaring, folder_of, read, Identity};
use crate::piece_saving::written_as;
use crate::workflow_changes::{CHANGED_SINCE, NOT_READY};

/// A rule or a lifecycle as the file's text says it, at `scope`: the DTO a save is sent.
fn piece_of(kind: LifecyclePieceKind, path: &str, text: &str, scope: &PieceScope) -> Result<LifecyclePiece, String> {
    match kind {
        LifecyclePieceKind::Lifecycle => {
            let held = config::parse_lifecycle(Path::new(path), text).map_err(|why| why.to_string())?;
            Ok(LifecyclePiece::Lifecycle { lifecycle: ipc::LifecycleDefinition { scope: scope.clone(), ..wire::lifecycle(&held) } })
        }
        LifecyclePieceKind::Rule => {
            let held = config::parse_rule(Path::new(path), text).map_err(|why| why.to_string())?;
            Ok(LifecyclePiece::Rule { rule: ipc::LifecycleRule { scope: scope.clone(), ..wire::rule(&held) } })
        }
        LifecyclePieceKind::Workspace | LifecyclePieceKind::Autonomy => Err(String::from("only a rule or a lifecycle is kept from a Job")),
    }
}

/// The kind a change's `piece` column names.
fn kind_of(piece: &str) -> LifecyclePieceKind {
    if piece == "rule" {
        LifecyclePieceKind::Rule
    } else {
        LifecyclePieceKind::Lifecycle
    }
}

/// The kind a change's `piece` column names, as the wire spells it.
fn copy_kind_of(piece: &str) -> CopyPieceKind {
    if piece == "rule" {
        CopyPieceKind::Rule
    } else {
        CopyPieceKind::Lifecycle
    }
}

/// What the owner reads a kind as.
fn word_of(kind: CopyPieceKind) -> &'static str {
    match kind {
        CopyPieceKind::Lifecycle => "lifecycle",
        CopyPieceKind::Rule => "rule",
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// `keep_job_piece`: one rule or lifecycle the Job's copy holds and changed, proposed for every Job. **Nothing
    /// is written**: the proposal is a change that is ready, and a draft on the Job asks the owner.
    pub(crate) async fn job_piece_kept(self: &Arc<Self>, keep: KeepJobPiece) -> Result<JobPieceProposed, Refusal> {
        let id = keep.session_id.as_str().trim().to_string();
        let (_, hosting) = self.copy_hosted(&id).await?;
        let (store_kind, kind, word) = match keep.kind {
            CopyPieceKind::Lifecycle => (PieceKind::Lifecycle, LifecyclePieceKind::Lifecycle, word_of(keep.kind)),
            CopyPieceKind::Rule => (PieceKind::Rule, LifecyclePieceKind::Rule, word_of(keep.kind)),
        };
        let piece_id = keep.id.trim().to_string();
        let (held, marks) = {
            let store = self.store().lock().await;
            let fault = |why| self.ledger_fault(why);
            let copy = store.job_copy_of(&id).map_err(fault)?.ok_or_else(|| {
                self.hosted_refusal(JOB_HOLDS_NO_COPY, &format!("job {id} froze no lifecycles or rules, so it has none of its own to keep"))
            })?;
            (copy.pieces.into_iter().find(|one| one.kind == store_kind && one.piece_id == piece_id), store.job_copy_marks(&id).map_err(fault)?)
        };
        // A dropped piece is not in the copy, so there is nothing of it to keep.
        let Some(held) = held else {
            return Err(self.hosted_refusal(JOB_COPY_PIECE_REFUSED, &format!("the copy holds no {word} named `{piece_id}`, so there is none to keep")));
        };
        let (lane, item) = match store_kind {
            PieceKind::Lifecycle => (piece_id.as_str(), ""),
            PieceKind::Rule => (RULE_LANE, piece_id.as_str()),
        };
        if !marks.iter().any(|mark| mark.lane == lane && mark.item == item) {
            return Err(self.hosted_refusal(
                JOB_COPY_PIECE_REFUSED,
                &format!("the {word} `{piece_id}` is as the Job froze it, so there is nothing of its own to keep"),
            ));
        }

        let scope = PieceScope::Within { id: hosting.manifest_id.clone() };
        let refused = |why: String| self.hosted_refusal(JOB_COPY_PIECE_REFUSED, &why);
        let piece = piece_of(kind, &held.path, &held.text, &scope).map_err(refused)?;
        let (_, written) = written_as(&piece);
        let after = written.map_err(refused)?;
        // Read as a save would read it, so a piece that is kept is one that loads.
        read(kind, Path::new("(kit)"), &after).map_err(refused)?;

        let wanted = Identity::of(kind, &piece_id, scope.clone());
        let kit_text = declaring(&self.kit_pieces(folder_of(kind)), &wanted).into_iter().next().map(|(_, text)| text);
        if kit_text.as_deref().is_some_and(|text| piece_of(kind, "(kit)", text, &scope).is_ok_and(|kept| kept == piece)) {
            return Err(self.hosted_refusal(JOB_COPY_PIECE_REFUSED, &format!("Kit already holds this {word} within the repository, so there is nothing to approve")));
        }
        let before = kit_text.unwrap_or_default();

        let at = self.now();
        let change = self
            .store()
            .lock()
            .await
            .begin_proposed_piece_change(&hosting.manifest_id, word, &piece_id, &before, &after, at.as_str())
            .map_err(|why| self.ledger_fault(why))?;
        let title = format!("{}{} {piece_id}", word[..1].to_uppercase(), &word[1..]);
        self.proposal_titled(&id, change, title, &after, Some(before.clone()).filter(|before| !before.is_empty())).await;
        self.keep_drafts_changed(&hosting.manifest_id);
        Ok(JobPieceProposed { change, kind: keep.kind, id: piece_id, scope, replaces: !before.is_empty() })
    }

    /// `list_keep_drafts`: the open proposals of a rule or a lifecycle in one repository, as the files Approve would write.
    pub(crate) async fn keep_drafts_listed(&self, manifest_id: Option<&ManifestId>) -> Result<KeepDrafts, Refusal> {
        let served = self.served_named(manifest_id)?;
        let manifest = served.manifest().id().as_str().to_string();
        let open = self.store().lock().await.open_piece_changes(&manifest).map_err(|why| self.ledger_fault(why))?;
        let drafts = open
            .into_iter()
            .filter_map(|one| Some(KeepDraft { kind: copy_kind_of(&one.piece), id: one.workflow_id, text: one.after_text? }))
            .collect();
        Ok(KeepDrafts { manifest_id: ManifestId::carried(manifest), drafts })
    }

    /// Say a repository's drafts of a rule or a lifecycle changed, so a page holding them reads them again. Told
    /// from the three places that move one, never from an agent's tool call.
    fn keep_drafts_changed(&self, manifest_id: &str) {
        self.publish(ipc::Event::KeepDraftsChanged(KeepDraftsChanged { manifest_id: ManifestId::carried(manifest_id) }));
    }

    /// A proposal on a Job's draft approved: a workflow's is saved as a drafted change is, and a rule's or a
    /// lifecycle's is saved to Kit.
    pub(crate) async fn proposal_approved(&self, change: WorkflowChangeRef) -> Result<(), Refusal> {
        let held = self.store().lock().await.workflow_change(change.change).map_err(|why| self.ledger_fault(why))?;
        match held {
            Some(kept) if !kept.piece.is_empty() => self.piece_change_approved(kept).await,
            _ => self.workflow_change_approved(change).await.map(|_| ()),
        }
    }

    /// A proposal on a Job's draft dismissed.
    pub(crate) async fn proposal_dismissed(&self, change: WorkflowChangeRef) -> Result<(), Refusal> {
        let held = self.store().lock().await.workflow_change(change.change).map_err(|why| self.ledger_fault(why))?;
        match held {
            Some(kept) if !kept.piece.is_empty() => self.piece_change_dismissed(kept).await,
            _ => self.workflow_change_dismissed(change).await.map(|_| ()),
        }
    }

    /// Write a ready piece to Kit through the save, if Kit's file is as the proposal began from it.
    async fn piece_change_approved(&self, kept: KeptWorkflowChange) -> Result<(), Refusal> {
        let said = |code: &str, said: String| {
            Refusal::IllegalMove(WireError::raised(code, said, self.run_id()).with_field("change", WireValue::Str(kept.id.to_string())))
        };
        if kept.state != store::CHANGE_READY {
            return Err(said(NOT_READY, format!("change {} is {}, and only a ready change is approved", kept.id, kept.state)));
        }
        let kind = kind_of(&kept.piece);
        let scope = PieceScope::Within { id: kept.manifest_id.clone() };
        let wanted = Identity::of(kind, &kept.workflow_id, scope.clone());
        let now = declaring(&self.kit_pieces(folder_of(kind)), &wanted).into_iter().next().map(|(_, text)| text);
        // A new piece has no earlier text to have changed; the save refuses it if Kit holds one now.
        if !kept.is_new && now.as_deref() != Some(kept.before_text.as_str()) {
            return Err(said(
                CHANGED_SINCE,
                format!("the {} `{}` in Kit has changed since this was proposed; dismiss it and keep it again", kept.piece, kept.workflow_id),
            ));
        }
        let Some(after) = kept.after_text.as_deref() else {
            return Err(said(NOT_READY, format!("change {} has no piece to write", kept.id)));
        };
        let piece = piece_of(kind, "(kit)", after, &scope).map_err(|why| self.hosted_refusal(JOB_COPY_PIECE_REFUSED, &why))?;
        self.lifecycle_piece_saved(SaveLifecyclePiece { piece, overwrite: !kept.is_new })?;
        let at = self.now();
        self.store().lock().await.workflow_change_approved(kept.id, at.as_str()).map_err(|why| self.ledger_fault(why))?;
        self.keep_drafts_changed(&kept.manifest_id);
        Ok(())
    }

    /// Put an open proposal of a piece aside.
    async fn piece_change_dismissed(&self, kept: KeptWorkflowChange) -> Result<(), Refusal> {
        let at = self.now();
        if self.store().lock().await.workflow_change_dismissed(kept.id, at.as_str()).map_err(|why| self.ledger_fault(why))? {
            self.keep_drafts_changed(&kept.manifest_id);
            return Ok(());
        }
        Err(Refusal::IllegalMove(WireError::raised(
            crate::workflow_changes::SETTLED,
            format!("change {} is {}, so there is nothing to dismiss", kept.id, kept.state),
            self.run_id(),
        )))
    }
}
