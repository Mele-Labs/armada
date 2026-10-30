// Saving and deleting one Workspace, lifecycle, rule or autonomy default in Kit, field for field with `crates/ipc/src/piece_saving.rs`.
// `docs/practices/protocol.md`, *Workspaces, lifecycles and rules, saved*. The reads are `lifecycles.ts`.
//
// **The DTOs a read answered are the input.** Bridge sends back what it was sent, changed, and Fleet writes the piece's file syntax and
// refuses what its own parser would refuse, so Bridge holds no second copy of the file format. A piece is its kind, id and scope together:
// `ticket` for every repository and `ticket` within one repository are two files that rank against each other.

import type { LifecycleDefinition, LifecycleRule, PieceScope } from "./lifecycles";

/** The four kinds of piece a person keeps in Kit's folders. */
export type LifecyclePieceKind = "lifecycle" | "rule" | "workspace" | "autonomy";

/** A Workspace as it is saved: a set of repositories under one id. */
export type WorkspaceToSave = {
  id: string;
  name: string;
  /** Manifest ids, in the order the file should list them. */
  repositories: string[];
};

/** A layer of autonomy defaults as it is saved. */
export type AutonomyToSave = {
  id: string;
  /** Everywhere where absent. */
  scope?: PieceScope;
  /** A kind of act (`tracker_write`, `forge_write`, `draft`, `start_job`, `start_watch`, `tell_agent`, `run_command` or `run_agent`) to `acts`, `asks` or `off`. */
  defaults: Record<string, string>;
};

/** One piece, by kind. */
export type LifecyclePiece =
  | { kind: "lifecycle"; lifecycle: LifecycleDefinition }
  | { kind: "rule"; rule: LifecycleRule }
  | { kind: "workspace"; workspace: WorkspaceToSave }
  | { kind: "autonomy"; autonomy: AutonomyToSave };

/**
 * `save_lifecycle_piece`'s body (`POST /lifecycles/pieces/save`): write one piece to Kit's folder. 422 `fleet.piece_unfit` with the parser's own
 * sentence and nothing written where the piece does not parse; 422 `fleet.piece_exists` naming the file where one declares the same kind, id and
 * scope and `overwrite` is not set; 500 `fleet.piece_unwritable`.
 */
export type SaveLifecyclePiece = {
  piece: LifecyclePiece;
  /** Required to replace the piece that already holds this kind, id and scope. False where absent. */
  overwrite?: boolean;
};

/** What `save_lifecycle_piece` wrote. */
export type LifecyclePieceSaved = {
  kind: LifecyclePieceKind;
  id: string;
  /** `everywhere` for a Workspace, which has none. */
  scope: PieceScope;
  /** The file as written, absolute. */
  file: string;
  /** It took the place of a file that declared the same kind, id and scope. */
  replaced: boolean;
};

/** `delete_lifecycle_piece`'s body (`POST /lifecycles/pieces/delete`): delete the Kit file that declares this piece. 422 `fleet.piece_not_kept` where Kit holds none. */
export type DeleteLifecyclePiece = {
  kind: LifecyclePieceKind;
  id: string;
  /** Everywhere where absent. Ignored for a Workspace. */
  scope?: PieceScope;
};

/** What `delete_lifecycle_piece` removed. */
export type LifecyclePieceDeleted = {
  kind: LifecyclePieceKind;
  id: string;
  scope: PieceScope;
  /** The file that was deleted, absolute. */
  file: string;
};
