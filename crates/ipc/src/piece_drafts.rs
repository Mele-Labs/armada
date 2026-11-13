//! A draft of a lifecycle, a rule or an autonomy default, as `workflow_drafts` is of a workflow. Protocol
//! `docs/practices/protocol/protocol-23-95-a-workflow-edited-as-a-draft-and-published.md`.
//!
//! **The draft text is the piece's JSON, as `SaveLifecyclePiece.piece` takes it.** Publish decodes it and runs
//! `save_lifecycle_piece` with `overwrite`, so a piece that is published is one Kit's parser reads. A piece is kept in Kit
//! alone, so there is no source here.

use serde::{Deserialize, Serialize};

use crate::lifecycles::PieceScope;
use crate::piece_saving::everywhere;
use crate::{DraftChange, Instant, LifecyclePieceKind, LifecyclePieceSaved};

/// One draft of one piece, as `get_piece_draft` answers it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PieceDraft {
    pub kind: LifecyclePieceKind,
    pub id: String,
    pub scope: PieceScope,
    /// The draft piece, whole, as the JSON `SaveLifecyclePiece.piece` takes.
    pub text: String,
    /// The piece Kit held when the draft began, as the same JSON. Empty where Kit held none.
    pub base: String,
    /// Why `text` would not publish, in the parser's own sentence. Absent where it would.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unfit: Option<String>,
    /// Oldest first. There is always at least one.
    pub changes: Vec<DraftChange>,
    pub started_at: Instant,
    pub updated_at: Instant,
}

/// `put_piece_draft`'s body. `PUT /lifecycles/pieces/draft`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PutPieceDraft {
    pub kind: LifecyclePieceKind,
    pub id: String,
    /// Absent is everywhere.
    #[serde(default = "everywhere")]
    pub scope: PieceScope,
    /// **Any text**: a draft may be half-edited, and `PieceDraft.unfit` says where it would not publish.
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

/// What `discard_piece_draft` removed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PieceDraftDiscarded {
    pub kind: LifecyclePieceKind,
    pub id: String,
    pub scope: PieceScope,
}

/// `publish_piece_draft`'s body. `POST /lifecycles/pieces/draft/publish`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishPieceDraft {
    pub kind: LifecyclePieceKind,
    pub id: String,
    /// Absent is everywhere.
    #[serde(default = "everywhere")]
    pub scope: PieceScope,
}

/// What `publish_piece_draft` wrote.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PiecePublished {
    pub saved: LifecyclePieceSaved,
}
