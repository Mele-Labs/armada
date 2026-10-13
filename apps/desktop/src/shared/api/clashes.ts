// Write clashes: where two agents' changes in one repository would conflict. A slice imports
// protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { Outcome, WriteClash } from "@armada/protocol";

/** What reading the clashes came to: every standing clash on the manifests Fleet lists, or the refusal to word. */
export type WriteClashesRead = { ok: true; value: WriteClash[] } | { ok: false; outcome: Outcome };

export type ClashesApi = {
  /**
   * The clashes standing in every repository Fleet serves. **Names nothing**: main knows which
   * manifests those are. A Fleet before protocol 23.81 answers none rather than a refusal.
   */
  getWriteClashes: () => Promise<WriteClashesRead>;
};

export const CLASHES_CHANNELS = {
  getWriteClashes: "bridge:get-write-clashes",
} as const;
