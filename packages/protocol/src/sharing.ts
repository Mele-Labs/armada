// Share, field for field with `crates/ipc/src/sharing.rs`. `docs/concepts/kit.md`, *Share*.

import type { PieceToCommit } from "./committing";
import type { FlightControlKind } from "./flight-controls";

/** The package version this Fleet writes and reads. */
export const PACKAGE_VERSION = 1;

/** One chosen piece, and the repository it is kept for: the selector `commit_to_repository` takes. */
export type PieceToShare = {
  /** A root `list_repositories` names. */
  repository: string;
  piece: PieceToCommit;
};

/** `share_package`'s body (`POST /share/package`). Fleet only makes the text; Bridge writes the file and the clipboard. */
export type SharePieces = { pieces: PieceToShare[] };

/** `share_package`'s answer. */
export type SharedPackage = {
  /** Ends in `.armada.toml`, and holds no path. */
  file_name: string;
  text: string;
};

export type PieceKind = "tile" | "workflow" | "review_tool" | "manifest";

/** The list a tile sits in. */
export type TileList = "job" | "deck";

/** `read_package`'s body (`POST /share/read`), writing nothing. */
export type PackageText = { text: string };

/** One piece a package holds, and what adding it would do. */
export type PackagePiece = {
  kind: PieceKind;
  id: string;
  name: string;
  /** A tile's list. */
  list?: TileList;
  /** A tile's own action. */
  action?: FlightControlKind;
  /** Every shell command the piece would run: a manifest's Check and Command lines. A workflow and a review tool run none themselves. */
  commands: string[];
  /** The remote of the repository it is kept for, `owner/name` lowercase. */
  remote?: string;
  /** The one repository Fleet serves that the remote names. Absent where none or several do: Add keeps it unbound. */
  repository?: string;
  /** Whether Kit already holds a piece of this id for that repository. */
  exists: boolean;
};

/** `read_package`'s answer. */
export type PackageRead = { pieces: PackagePiece[] };

/** `add_package`'s body (`POST /share/add`). A malformed package is a 422 `fleet.share_malformed`; an id Kit holds without `replace`, `fleet.share_exists`. */
export type AddPackage = {
  text: string;
  /** Replace a piece of the same id Kit holds. False where absent. */
  replace?: boolean;
};

/** Where an added piece went. */
export type Landed = "added" | "replaced" | "unbound";

/** One piece `add_package` or `bind_unbound_piece` dealt with. */
export type PieceAdded = {
  kind: PieceKind;
  id: string;
  name: string;
  list?: TileList;
  remote?: string;
  /** The root it was written for. Absent where it is unbound. */
  repository?: string;
  landed: Landed;
  /** What `bind_unbound_piece` takes. Present where `landed` is `unbound`. */
  unbound_id?: string;
};

/** `add_package`'s answer. */
export type PackageAdded = { pieces: PieceAdded[] };

/** A piece kept unbound until a repository is chosen. */
export type UnboundPiece = {
  unbound_id: string;
  kind: PieceKind;
  id: string;
  name: string;
  list?: TileList;
  commands: string[];
  remote?: string;
};

/** `list_unbound_pieces`' answer (`GET /share/unbound`). */
export type UnboundPieces = { pieces: UnboundPiece[] };

/** `bind_unbound_piece`'s body (`POST /share/bind`), answered with a `PieceAdded`. Once: the piece is no longer unbound. */
export type BindUnboundPiece = {
  unbound_id: string;
  /** A root `list_repositories` names. */
  repository: string;
  /** Replace a piece of the same id Kit holds for it. False where absent. */
  replace?: boolean;
};
