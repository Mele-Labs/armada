// The Logbook page's reads and acts. A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts`
// compose them. `docs/concepts/logbook.md`.
//
// **Main is not written yet, and Fleet serves no route.** The renderer is built against these types and the mock
// answers them; in the app an invoke rejects until main handles the channel.

import type { LogbookAnswer, LogbookForgotten, LogbookRead } from "@armada/protocol";

export type LogbookApi = {
  /** Every section: each repository's, Habits and This Mac. Forgotten entries are never read. */
  readLogbook: () => Promise<LogbookRead>;
  /** Replaces the entry's text and marks it edited. */
  editLogbookEntry: (id: string, text: string) => Promise<LogbookAnswer>;
  /** A locked entry never fades and is never rewritten. */
  lockLogbookEntry: (id: string, locked: boolean) => Promise<LogbookAnswer>;
  /** Puts the entry on the deny list, so no later Job brings it back. */
  forgetLogbookEntry: (id: string) => Promise<LogbookForgotten>;
};

export const LOGBOOK_CHANNELS = {
  readLogbook: "bridge:read-logbook",
  editLogbookEntry: "bridge:edit-logbook-entry",
  lockLogbookEntry: "bridge:lock-logbook-entry",
  forgetLogbookEntry: "bridge:forget-logbook-entry",
} as const;
