// A person's own pieces copied onto a Job's branch, made into a package to send, and a package read and added. A slice imports
// protocol, never another slice; `../api.ts` and `../bridge.ts` compose them. `docs/practices/protocol.md`, *Commit to repository*
// and *Share*.

import type {
  AddPackage,
  BindUnboundPiece,
  CommitToRepository,
  CommittedToRepository,
  Outcome,
  PackageAdded,
  PackageRead,
  PieceAdded,
  SharePieces,
  SharedPackage,
  UnboundPieces,
} from "@armada/protocol";

/** What one of these came to: Fleet's answer, or the refusal to word. */
export type SharingAnswer<T> = { ok: true; value: T } | { ok: false; outcome: Outcome };

/** What saving a package came to. A dialog closed without a place is not a failure. */
export type PackageSaved = { saved: true; name: string } | { saved: false; why: "cancelled" | "failed" };

export type SharingApi = {
  /** `POST /repositories/commit`: copy one piece onto the open Job's branch and commit it there. */
  commitToRepository: (body: CommitToRepository) => Promise<SharingAnswer<CommittedToRepository>>;
  /** `POST /share/package`: the package's file name and text. Fleet writes nothing. */
  sharePackage: (body: SharePieces) => Promise<SharingAnswer<SharedPackage>>;
  /** `POST /share/read`: every piece and command the text would add. Fleet writes nothing. */
  readPackage: (text: string) => Promise<SharingAnswer<PackageRead>>;
  /** `POST /share/add`: the one ask, writing every piece where the person's own live. */
  addPackage: (body: AddPackage) => Promise<SharingAnswer<PackageAdded>>;
  /** `GET /share/unbound`: pieces whose remote names no repository Fleet serves. */
  listUnboundPieces: () => Promise<SharingAnswer<UnboundPieces>>;
  /** `POST /share/bind`: write one unbound piece onto the repository the person chose. */
  bindUnboundPiece: (body: BindUnboundPiece) => Promise<SharingAnswer<PieceAdded>>;
  /** Asks the person where to put the package and writes `text` there. `name` is a file name, never a path. */
  savePackageFile: (name: string, text: string) => Promise<PackageSaved>;
};

export const SHARING_CHANNELS = {
  commitToRepository: "bridge:commit-to-repository",
  sharePackage: "bridge:share-package",
  readPackage: "bridge:read-package",
  addPackage: "bridge:add-package",
  listUnboundPieces: "bridge:list-unbound-pieces",
  bindUnboundPiece: "bridge:bind-unbound-piece",
  savePackageFile: "bridge:save-package-file",
} as const;
