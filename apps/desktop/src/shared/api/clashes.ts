// Write clashes: where two agents' changes in one repository would conflict. A slice imports
// protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { ClashFile, ClashSide, Outcome, SessionFileChange, WriteClash } from "@armada/protocol";

/** What reading the clashes came to: every standing clash on the manifests Fleet lists, or the refusal to word. */
export type WriteClashesRead = { ok: true; value: WriteClash[] } | { ok: false; outcome: Outcome };

/** What reading one file of a clash came to: each side's patch, or the refusal to word. */
export type ClashFileAnswer = { ok: true; value: ClashFile } | { ok: false; outcome: Outcome };

/** What reading one file a Job wrote came to: its change, or the refusal to word. */
export type SessionFileChangeAnswer = { ok: true; value: SessionFileChange } | { ok: false; outcome: Outcome };

export type ClashesApi = {
  /**
   * The clashes standing in every repository Fleet serves. **Names nothing**: main knows which
   * manifests those are. A Fleet before protocol 23.81 answers none rather than a refusal.
   */
  getWriteClashes: () => Promise<WriteClashesRead>;
  /**
   * `GET /clashes/file`: one file of the clash between a Session and `other`, as each side changed it. **Names no Manifest**:
   * Fleet finds the clash by the Session. A Fleet without the route answers a transport 404.
   */
  readClashFile: (sessionId: string, other: { kind: ClashSide; id: string }, path: string) => Promise<ClashFileAnswer>;
  /**
   * `GET /sessions/file_change`: one file a Job has been seen writing, as its change since the base. `seen` is the stamp of the change
   * held, which Fleet answers `unchanged` without the diff. A Fleet without the route answers a transport 404.
   */
  readSessionFileChange: (sessionId: string, path: string, seen?: string) => Promise<SessionFileChangeAnswer>;
};

export const CLASHES_CHANNELS = {
  getWriteClashes: "bridge:get-write-clashes",
  readClashFile: "bridge:read-clash-file",
  readSessionFileChange: "bridge:read-session-file-change",
} as const;
