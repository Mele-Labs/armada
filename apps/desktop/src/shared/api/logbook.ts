// The Logbook page's reads and acts. A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts`
// compose them. `docs/concepts/logbook.md`.
//
// Main answers each channel from Fleet's routes (`main/logbook.ts`); the mock answers them in a walk.

import type { LogbookAnswer, LogbookForgotten, LogbookKind, LogbookRead, LogbookTidied } from "@armada/protocol";

/** Organize's three looks and a finding, as the page's `OrganizeAction` and `OrganizeRow` draw them. */
export type OrganizeAction = "merge" | "stale" | "bugs";
export type OrganizeFolded = { text: string; kind: LogbookKind; confirmed: string };
export type OrganizeRow = { id: string; text: string; kind: LogbookKind; confirmed: string; note: string; absorbed?: readonly OrganizeFolded[] };

export type LogbookApi = {
  /** Every section: each repository's, Habits and This Mac. Forgotten entries are never read. */
  readLogbook: () => Promise<LogbookRead>;
  /** Replaces the entry's text and marks it edited. */
  editLogbookEntry: (id: string, text: string) => Promise<LogbookAnswer>;
  /** A locked entry never fades and is never rewritten. */
  lockLogbookEntry: (id: string, locked: boolean) => Promise<LogbookAnswer>;
  /** Puts the entry on the deny list, so no later Job brings it back. */
  forgetLogbookEntry: (id: string) => Promise<LogbookForgotten>;
  /** Starts folding near-duplicate entries across every repository and answers at once; the Logbook read says when it is done. */
  tidyLogbook: () => Promise<LogbookTidied>;
  /** Mock only: what Organize finds for an action, after a short pass. Fleet serves no such route, so the real window leaves it absent and draws Tidy. */
  organizeLogbook?: (action: OrganizeAction) => Promise<OrganizeRow[]>;
  /** Mock only: carries out the rows left in a proposal and says what it came to. */
  applyOrganizeLogbook?: (action: OrganizeAction, rows: OrganizeRow[]) => Promise<string>;
};

export const LOGBOOK_CHANNELS = {
  readLogbook: "bridge:read-logbook",
  editLogbookEntry: "bridge:edit-logbook-entry",
  lockLogbookEntry: "bridge:lock-logbook-entry",
  forgetLogbookEntry: "bridge:forget-logbook-entry",
  tidyLogbook: "bridge:tidy-logbook",
} as const;
