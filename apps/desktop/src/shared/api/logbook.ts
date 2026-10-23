// The Logbook page's reads and acts. A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts`
// compose them. `docs/concepts/logbook.md`.
//
// Main answers each channel from Fleet's routes (`main/logbook.ts`); the mock answers them in a walk.

import type { LogbookAnswer, LogbookForgotten, LogbookRead, LogbookTidied } from "@armada/protocol";

export type LogbookApi = {
  /** Every section: each repository's, Habits and This Mac. Forgotten entries are never read. */
  readLogbook: () => Promise<LogbookRead>;
  /** Replaces the entry's text and marks it edited. */
  editLogbookEntry: (id: string, text: string) => Promise<LogbookAnswer>;
  /** A locked entry never fades and is never rewritten. */
  lockLogbookEntry: (id: string, locked: boolean) => Promise<LogbookAnswer>;
  /** Puts the entry on the deny list, so no later Job brings it back. */
  forgetLogbookEntry: (id: string) => Promise<LogbookForgotten>;
  /** Folds near-duplicate entries into one across every repository, and answers how many it merged. */
  tidyLogbook: () => Promise<LogbookTidied>;
};

export const LOGBOOK_CHANNELS = {
  readLogbook: "bridge:read-logbook",
  editLogbookEntry: "bridge:edit-logbook-entry",
  lockLogbookEntry: "bridge:lock-logbook-entry",
  forgetLogbookEntry: "bridge:forget-logbook-entry",
  tidyLogbook: "bridge:tidy-logbook",
} as const;
