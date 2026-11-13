// A draft of a workflow, or of a lifecycle, rule or autonomy default, that the owner and an agent edit before it is published: field for field
// with `crates/ipc/src/workflow_drafts.rs` and `crates/ipc/src/piece_drafts.rs`. Protocol 23.95,
// `docs/practices/protocol/protocol-23-95-a-workflow-edited-as-a-draft-and-published.md`.
//
// **A draft is not a save.** `save_workflow` still writes at once. A draft is Fleet's, each change keeps the whole text after it, and the editor steps back by
// putting an earlier change's text again. Publish writes it through the same save, with overwrite.

import type { PieceScope } from "./lifecycles";
import type { LifecyclePieceKind, LifecyclePieceSaved } from "./piece-saving";
import type { WorkflowSaved, WorkflowScope } from "./setup";

/** Who made a change to a draft. */
export type DraftBy = "you" | "agent";

/** One change to a draft: the whole draft after it. */
export type DraftChange = {
  /** Fleet's number for it, rising with each change to any draft. */
  id: number;
  by: DraftBy;
  at: string;
  /** The whole draft after this change. */
  text: string;
  /** What the change did, in a line. An agent's carries the words it was asked for. */
  summary?: string;
  /** The `WorkflowChange` an agent's change came from. Absent on one the owner made. */
  change?: number;
};

/**
 * `GET /workflows/draft?workflow_id=&source=&project=&manifest_id=`: the draft of one workflow in one place. 404 `fleet.workflow_draft_none` where none is kept;
 * 422 `fleet.workflow_project_unnamed`, 404 `fleet.workflow_project_unknown` and 422 `fleet.workflow_project_not_member` for a project's.
 */
export type WorkflowDraft = {
  workflow_id: string;
  /** `armada`, `kit`, `repository` or `project`. A draft of Armada's has no scope: the copy is made when it is published. */
  source: string;
  /** The project, where `source` is `project`. */
  project?: string;
  /** The draft definition, whole: the last change's text. */
  text: string;
  /** The published text the draft began from; for Armada's, the carried text. Empty where the place held none. */
  base: string;
  /** Why `text` would not load, in the loader's own sentence. Absent where it would. */
  unfit?: string;
  /** Oldest first. There is always at least one. */
  changes: DraftChange[];
  started_at: string;
  updated_at: string;
};

/** `PUT /workflows/draft?manifest_id=`'s body: begins the draft from what is published there, or adds a change by `you`. Any text is kept; `unfit` says where it would not load. */
export type PutWorkflowDraft = {
  workflow_id: string;
  source: string;
  /** Required where `source` is `project`. */
  project?: string;
  text: string;
  summary?: string;
};

/** What `DELETE /workflows/draft?workflow_id=&source=&project=&manifest_id=` removed, with every change in it. 404 `fleet.workflow_draft_none` where none was kept. */
export type WorkflowDraftDiscarded = {
  workflow_id: string;
  source: string;
  project?: string;
};

/**
 * `POST /workflows/draft/publish?manifest_id=`'s body. `scope` is required for a draft of an `armada` workflow, and may be any of the three (422
 * `fleet.workflow_draft_scope_needed` without it); for any other source it is absent or names that same place (422 `fleet.workflow_draft_scope_differs`).
 * 409 `fleet.workflow_draft_stale` where what is published there no longer equals the draft's `base`. The loader's refusals pass through the save unchanged:
 * 422 `fleet.workflow_unfit`, 422 `fleet.workflow_id_not_a_name`, 500 `fleet.workflow_unwritable`. Nothing is written on a refusal, and the draft stays.
 */
export type PublishWorkflowDraft = {
  workflow_id: string;
  source: string;
  project?: string;
  scope?: WorkflowScope;
};

/** What `POST /workflows/draft/publish` wrote: the save it made. */
export type WorkflowPublished = {
  saved: WorkflowSaved;
};

/**
 * `GET /lifecycles/pieces/draft?kind=&id=&within=`: the draft of one piece in Kit. `within` is the scope's id, and absent is everywhere. 404
 * `fleet.piece_draft_none` where none is kept.
 */
export type PieceDraft = {
  kind: LifecyclePieceKind;
  id: string;
  scope: PieceScope;
  /** The piece as the JSON `SaveLifecyclePiece.piece` takes, whole. */
  text: string;
  /** The piece Kit held when the draft began, as the same JSON. Empty where Kit held none. */
  base: string;
  /** Why `text` would not publish, in the parser's own sentence. Absent where it would. */
  unfit?: string;
  /** Oldest first. There is always at least one. */
  changes: DraftChange[];
  started_at: string;
  updated_at: string;
};

/** `PUT /lifecycles/pieces/draft`'s body. Any text is kept; `unfit` says where it would not publish. */
export type PutPieceDraft = {
  kind: LifecyclePieceKind;
  id: string;
  /** Everywhere where absent. */
  scope?: PieceScope;
  text: string;
  summary?: string;
};

/** What `DELETE /lifecycles/pieces/draft?kind=&id=&within=` removed. 404 `fleet.piece_draft_none` where none was kept. */
export type PieceDraftDiscarded = {
  kind: LifecyclePieceKind;
  id: string;
  scope: PieceScope;
};

/**
 * `POST /lifecycles/pieces/draft/publish`'s body: decodes the draft and runs the piece save with overwrite. 422 `fleet.piece_unfit` with the parser's own
 * sentence; 409 `fleet.piece_draft_stale` where Kit's piece is no longer the draft's `base`; 404 `fleet.piece_draft_none`.
 */
export type PublishPieceDraft = {
  kind: LifecyclePieceKind;
  id: string;
  /** Everywhere where absent. */
  scope?: PieceScope;
};

/** What `POST /lifecycles/pieces/draft/publish` wrote. */
export type PiecePublished = {
  saved: LifecyclePieceSaved;
};
