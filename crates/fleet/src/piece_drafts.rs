//! The draft of a lifecycle, a rule or an autonomy default, as `workflow_drafts` keeps a workflow's.
//! `docs/concepts/workflow.md`, *Drafts and Publish*.
//!
//! **The text is the piece's JSON**, as `SaveLifecyclePiece.piece` takes it, so what the editor holds is what a save is
//! given. A piece is kept in Kit alone, so a draft has no place and no repository: it is keyed by kind, id and scope, as
//! `DeleteLifecyclePiece` keys a piece. `base` is the piece Kit holds when the draft begins, read back as the same JSON,
//! and a publish is held against Kit still holding it.
//!
//! **Publish is `save_lifecycle_piece` with overwrite**, on the piece the text decodes to, so the piece's own parser refuses
//! what it refuses and nothing is written then. The draft is deleted only once the file is written.

use std::collections::BTreeMap;
use std::path::Path;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    AutonomyToSave, DeleteLifecyclePiece, Instant, LifecyclePiece, LifecyclePieceKind, PieceDraft, PieceDraftDiscarded, PieceScope,
    PiecePublished, PublishPieceDraft, PutPieceDraft, SaveLifecyclePiece, WireError, WireValue,
};
use store::{DraftKey, KeptDraft, NewDraftChange};

use crate::daemon::Fleet;
use crate::lifecycles::wire;
use crate::piece_saving::identity::{declaring, folder_of, read, Identity};
use crate::piece_saving::{written_as, PIECE_UNFIT};
use crate::workflow_drafts::draft_changes;

/// No draft is kept for that piece. A 404.
const DRAFT_NONE: &str = "fleet.piece_draft_none";
/// Kit holds a piece other than the one the draft began from. A 409.
const DRAFT_STALE: &str = "fleet.piece_draft_stale";
/// The store would not answer. A 500.
const DRAFT_NOT_KEPT: &str = "fleet.piece_draft_not_kept";

/// A piece's scope as the word it is kept under: `everywhere`, or `within:<id>`.
fn scope_word(scope: &PieceScope) -> String {
    match scope {
        PieceScope::Everywhere => String::from("everywhere"),
        PieceScope::Within { id } => format!("within:{id}"),
    }
}

/// The key a piece's draft is kept under.
fn key_of(identity: &Identity) -> DraftKey {
    DraftKey {
        kind: identity.kind.as_wire().into(),
        source: String::from("kit"),
        scope: scope_word(&identity.scope),
        item_id: identity.id.clone(),
        ..DraftKey::default()
    }
}

/// The piece a file's text declares, as the JSON `SaveLifecyclePiece.piece` takes. `None` where the text is not a piece
/// its parser reads.
fn piece_json(kind: LifecyclePieceKind, text: &str) -> Option<String> {
    let path = Path::new("(piece)");
    let piece = match kind {
        LifecyclePieceKind::Lifecycle => LifecyclePiece::Lifecycle { lifecycle: wire::lifecycle(&config::parse_lifecycle(path, text).ok()?) },
        LifecyclePieceKind::Rule => LifecyclePiece::Rule { rule: wire::rule(&config::parse_rule(path, text).ok()?) },
        LifecyclePieceKind::Autonomy => {
            let read = config::parse_autonomy(path, text).ok()?;
            let defaults: BTreeMap<String, String> =
                wire::autonomy_set(read.defaults()).into_iter().map(|one| (one.kind, one.autonomy)).collect();
            LifecyclePiece::Autonomy { autonomy: AutonomyToSave { id: read.id().as_str().to_string(), scope: wire::scope(read.scope()), defaults } }
        }
    };
    ipc::encode(&piece).ok()
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
    fn piece_draft_not_kept(&self, why: impl std::fmt::Debug) -> Refusal {
        Refusal::Fault(WireError::raised(DRAFT_NOT_KEPT, format!("the draft could not be kept: {why:?}"), self.run_id()))
    }

    fn no_piece_draft(&self, identity: &Identity) -> Refusal {
        Refusal::NotFound(WireError::raised(DRAFT_NONE, format!("no draft of the {} is kept", identity.said()), self.run_id()))
    }

    /// What Kit holds of the piece now, as the JSON a draft's `base` is. Empty where Kit holds none.
    fn published_piece(&self, identity: &Identity) -> String {
        declaring(&self.kit_pieces(folder_of(identity.kind)), identity)
            .first()
            .and_then(|(_, text)| piece_json(identity.kind, text))
            .unwrap_or_default()
    }

    /// Why `text` would not publish as the piece `identity` names, in the parser's own sentence: it is a piece, it is the
    /// kind, id and scope the draft is of, and its file would be read.
    fn piece_draft_unfit(&self, identity: &Identity, text: &str) -> Option<String> {
        let piece: LifecyclePiece = match ipc::decode("a piece", text.as_bytes()) {
            Ok(piece) => piece,
            Err(why) => return Some(format!("The draft is not a piece: {why}")),
        };
        let (kind, written) = written_as(&piece);
        let file = match written {
            Ok(file) => file,
            Err(why) => return Some(format!("The {} could not be written as a file: {why}", kind.as_wire())),
        };
        let declared = match read(kind, Path::new("(draft)"), &file) {
            Ok(declared) => declared,
            Err(why) => return Some(why),
        };
        (&declared != identity).then(|| format!("The draft is the {}, and this is a draft of the {}", declared.said(), identity.said()))
    }

    fn piece_draft_wire(&self, identity: &Identity, draft: &KeptDraft) -> PieceDraft {
        let text = draft.text().to_string();
        PieceDraft {
            kind: identity.kind,
            id: identity.id.clone(),
            scope: identity.scope.clone(),
            unfit: self.piece_draft_unfit(identity, &text),
            text,
            base: draft.base.clone(),
            changes: draft_changes(draft),
            started_at: Instant::carried(draft.started_at.clone()),
            updated_at: Instant::carried(draft.updated_at.clone()),
        }
    }

    pub(crate) async fn piece_draft_read(&self, piece: DeleteLifecyclePiece) -> Result<PieceDraft, Refusal> {
        let identity = Identity::of(piece.kind, &piece.id, piece.scope);
        let kept = self.store().lock().await.workflow_draft(&key_of(&identity)).map_err(|why| self.piece_draft_not_kept(why))?;
        let draft = kept.ok_or_else(|| self.no_piece_draft(&identity))?;
        Ok(self.piece_draft_wire(&identity, &draft))
    }

    /// Keep the owner's text: begin the draft from what Kit holds if there is none, and add a change by `you`. A text the
    /// draft already ends on adds nothing.
    pub(crate) async fn piece_draft_put(&self, put: PutPieceDraft) -> Result<PieceDraft, Refusal> {
        let identity = Identity::of(put.kind, &put.id, put.scope);
        let key = key_of(&identity);
        let base = self.published_piece(&identity);
        let at = self.now();
        let mut store = self.store().lock().await;
        let held = store.workflow_draft(&key).map_err(|why| self.piece_draft_not_kept(why))?;
        let kept = match held {
            Some(draft) if draft.text() == put.text => draft,
            _ => store
                .put_draft_change(
                    &key,
                    &base,
                    &NewDraftChange { made_by: store::DRAFT_BY_YOU, text: &put.text, summary: put.summary.as_deref(), change: None },
                    at.as_str(),
                )
                .map_err(|why| self.piece_draft_not_kept(why))?,
        };
        drop(store);
        Ok(self.piece_draft_wire(&identity, &kept))
    }

    pub(crate) async fn piece_draft_discarded(&self, piece: DeleteLifecyclePiece) -> Result<PieceDraftDiscarded, Refusal> {
        let identity = Identity::of(piece.kind, &piece.id, piece.scope);
        if !self.store().lock().await.discard_draft(&key_of(&identity)).map_err(|why| self.piece_draft_not_kept(why))? {
            return Err(self.no_piece_draft(&identity));
        }
        Ok(PieceDraftDiscarded { kind: identity.kind, id: identity.id, scope: identity.scope })
    }

    /// Decode the draft and save it over what Kit holds, then delete the draft.
    pub(crate) async fn piece_draft_published(&self, publish: PublishPieceDraft) -> Result<PiecePublished, Refusal> {
        let identity = Identity::of(publish.kind, &publish.id, publish.scope);
        let key = key_of(&identity);
        let kept = self.store().lock().await.workflow_draft(&key).map_err(|why| self.piece_draft_not_kept(why))?;
        let draft = kept.ok_or_else(|| self.no_piece_draft(&identity))?;
        if self.published_piece(&identity) != draft.base {
            return Err(Refusal::IllegalMove(
                WireError::raised(
                    DRAFT_STALE,
                    format!(
                        "Kit's {} has changed since this draft began from it; nothing was written, and the draft is kept to read against what is there now",
                        identity.said()
                    ),
                    self.run_id(),
                )
                .with_field("id", WireValue::Str(identity.id.clone())),
            ));
        }
        // Nothing is written on a refusal: the text is a piece, the one this draft is of, and one its parser reads.
        if let Some(why) = self.piece_draft_unfit(&identity, draft.text()) {
            return Err(Refusal::Unacceptable(WireError::raised(PIECE_UNFIT, why, self.run_id())));
        }
        let piece: LifecyclePiece = ipc::decode("a piece", draft.text().as_bytes())
            .map_err(|why| Refusal::Unacceptable(WireError::raised(PIECE_UNFIT, format!("The draft is not a piece: {why}"), self.run_id())))?;
        let saved = self.lifecycle_piece_saved(SaveLifecyclePiece { piece, overwrite: true })?;
        self.store().lock().await.discard_draft(&key).map_err(|why| self.piece_draft_not_kept(why))?;
        Ok(PiecePublished { saved })
    }
}
